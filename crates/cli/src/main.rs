use anyhow::{anyhow, Result};
use api::{router, ApiState};
use axum::serve;
use clap::{Parser, Subcommand};
use config::NodeConfig;
use libp2p::{Multiaddr, PeerId};
use model_runtime::LlamaRuntime;
use net_libp2p::{
    extract_capability, extract_request, MeshNode, MeshRequest, MeshResponse, RoutingDecision,
};
use node_core::{JobEnvelope, JobResultEnvelope, NodeCapability, NodeIdentity};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tracing::{info, warn};

#[derive(Parser)]
#[command(author, version, about = "Decentralized P2P AI Mesh Node")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the full mesh node (daemon mode)
    Run,
    /// Send a one-off job to a peer (useful for testing)
    SendJob {
        /// Target peer ID (base58 string from logs)
        #[arg(long)]
        to: String,
        /// Prompt / task to send
        #[arg(long)]
        prompt: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let config = NodeConfig::load()?;

    // Persistent identity
    let identity_dir = config::NodeConfig::config_dir().join("identity");
    let identity_path = identity_dir.join("identity.key");

    let identity = if identity_path.exists() {
        info!("Loading existing identity");
        NodeIdentity::load(&identity_path)?
    } else {
        info!("Generating new identity");
        std::fs::create_dir_all(&identity_dir)?;
        let id = NodeIdentity::new();
        id.save(&identity_path)?;
        id
    };

    info!("Node Peer ID: {}", identity.peer_id_hex());

    match cli.command {
        Some(Command::SendJob { to, prompt }) => {
            send_one_shot_job(&config, &identity, &to, &prompt).await?;
        }
        _ => {
            run_daemon(config, identity).await?;
        }
    }

    Ok(())
}

async fn run_daemon(config: NodeConfig, identity: NodeIdentity) -> Result<()> {
    let runtime = Arc::new(LlamaRuntime::new(&config.llama_base_url, &config.model_name));

    // Start local API
    let api_addr: SocketAddr = config.api_listen.parse()?;
    let api_state = ApiState {
        runtime: runtime.clone(),
    };
    let listener = TcpListener::bind(api_addr).await?;
    tokio::spawn(async move {
        let app = router(api_state);
        info!("API listening on http://{}", api_addr);
        if let Err(e) = serve(listener, app).await {
            warn!("API server error: {}", e);
        }
    });

    // Start P2P
    let p2p_addr: Multiaddr = config.p2p_listen.parse()?;
    let mut mesh = MeshNode::new(p2p_addr.clone()).await?;

    // Announce our capability profile on mesh startup.
    let capability = NodeCapability {
        models: vec![config.model_name.clone()],
        ..NodeCapability::default()
    };
    if let Err(e) = mesh.publish_capability(&capability) {
        warn!("Failed to publish capability: {}", e);
    } else {
        info!("Published node capabilities");
    }

    // Bootstrap dialing
    for addr_str in &config.bootstrap_peers {
        if let Ok(addr) = addr_str.parse::<Multiaddr>() {
            if let Err(e) = mesh.dial(addr) {
                warn!("Failed to dial bootstrap peer {}: {}", addr_str, e);
            } else {
                info!("Dialed bootstrap peer: {}", addr_str);
            }
        }
    }

    info!("P2P listening on {}", p2p_addr);

    // Main event loop
    loop {
        if let Some(event) = mesh.next_event().await {
            if let Some((peer, capability)) = extract_capability(&event) {
                info!(
                    "Discovered capability from {}: models={:?} quant={} max_context={} speed={} tasks={:?}",
                    peer,
                    capability.models,
                    capability.quant,
                    capability.max_context,
                    capability.estimated_speed,
                    capability.supported_tasks,
                );
            }

            if let Some((peer, request, channel)) = extract_request(&event) {
                match request {
                    MeshRequest::RunJob(job) => {
                        info!("Received job {} from {}", job.job_id, peer);

                        let my_cap = NodeCapability {
                            models: vec![config.model_name.clone()],
                            ..NodeCapability::default()
                        };
                        let score = my_cap.score_for_job(&job.model);

                        let decision = if score > 15.0 {
                            RoutingDecision::Local
                        } else {
                            // Peer-aware forwarding comes next; keep explicit placeholder.
                            RoutingDecision::Forward(peer)
                        };

                        match decision {
                            RoutingDecision::Local => {
                                info!("Running job locally (score: {:.1})", score);

                                let output = match runtime.chat(&job.payload).await {
                                    Ok(s) => s,
                                    Err(e) => format!("runtime error: {}", e),
                                };

                                let result = JobResultEnvelope::new(
                                    job.job_id,
                                    identity.peer_id_hex(),
                                    job.model.clone(),
                                    output,
                                )
                                .sign(&identity)?;

                                mesh.send_response(channel, MeshResponse::JobResult(result));
                                info!("Sent signed local result for job {}", job.job_id);
                            }
                            RoutingDecision::Forward(best_peer) => {
                                info!(
                                    "Job score too low ({:.1}), would forward if better peers known (placeholder peer: {})",
                                    score,
                                    best_peer,
                                );

                                let output =
                                    "forwarding not implemented yet - running locally".to_string();
                                let result = JobResultEnvelope::new(
                                    job.job_id,
                                    identity.peer_id_hex(),
                                    job.model.clone(),
                                    output,
                                )
                                .sign(&identity)?;

                                mesh.send_response(channel, MeshResponse::JobResult(result));
                            }
                            RoutingDecision::Reject(reason) => {
                                mesh.send_response(channel, MeshResponse::Error(reason));
                            }
                        }
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn send_one_shot_job(
    config: &NodeConfig,
    identity: &NodeIdentity,
    target: &str,
    prompt: &str,
) -> Result<()> {
    // Prefer regular PeerId text format, but also accept hex-encoded bytes.
    let target_peer: PeerId = target.parse().or_else(|_| {
        let bytes = hex::decode(target).map_err(|e| anyhow!("invalid --to value: {e}"))?;
        PeerId::from_bytes(&bytes).map_err(|e| anyhow!("invalid peer id bytes: {e}"))
    })?;

    let job = JobEnvelope::new(
        "chat".to_string(),
        config.model_name.clone(),
        prompt.to_string(),
        identity.peer_id_hex(),
    )
    .sign(identity)?;

    let p2p_addr: Multiaddr = config.p2p_listen.parse()?;
    let mut mesh = MeshNode::new(p2p_addr).await?;

    // Dial first bootstrap or assume target is reachable.
    if let Some(bootstrap) = config.bootstrap_peers.first() {
        if let Ok(addr) = bootstrap.parse::<Multiaddr>() {
            let _ = mesh.dial(addr);
        }
    }

    let _request_id = mesh.send_job(&target_peer, job);
    info!("Sent one-shot job to peer {}", target);

    // For simplicity we do not wait for response in one-shot mode yet.
    tokio::time::sleep(Duration::from_secs(3)).await;
    Ok(())
}
