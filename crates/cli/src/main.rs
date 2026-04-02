use anyhow::{Context, Result};
use api::{router, ApiState};
use axum::serve;
use libp2p::{request_response, swarm::SwarmEvent, Multiaddr};
use model_runtime::LlamaRuntime;
use net_libp2p::{BehaviourEvent, MeshNode, MeshRequest, MeshResponse};
use node_core::{JobResultEnvelope, NodeIdentity};
use mesh_config::NodeConfig;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = NodeConfig::load().context("failed to load config")?;

    // Persistent identity: load from ~/.ai-mesh/identity.key or create a new one.
    let identity_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".ai-mesh");
    let identity_path = identity_dir.join("identity.key");

    let identity = if identity_path.exists() {
        NodeIdentity::load(&identity_path).context("failed to load identity")?
    } else {
        std::fs::create_dir_all(&identity_dir).context("failed to create identity dir")?;
        let id = NodeIdentity::new();
        id.save(&identity_path).context("failed to save identity")?;
        id
    };
    tracing::info!(peer = %identity.peer_id_hex(), "node identity loaded");

    let runtime = Arc::new(LlamaRuntime::new(&config.llama_base_url, &config.model_name));
    let api_state = ApiState { runtime: runtime.clone() };

    let listener = TcpListener::bind(&config.api_listen)
        .await
        .with_context(|| format!("failed to bind api on {}", config.api_listen))?;
    tracing::info!(addr = %config.api_listen, "api listening");

    tokio::spawn(async move {
        if let Err(e) = serve(listener, router(api_state)).await {
            tracing::error!(error = %e, "api server error");
        }
    });

    let p2p_addr: Multiaddr = config
        .p2p_listen
        .parse()
        .with_context(|| format!("invalid p2p multiaddr: {}", config.p2p_listen))?;
    let mut mesh = MeshNode::new(p2p_addr).await?;
    tracing::info!(peer = %mesh.peer_id, p2p = %config.p2p_listen, "mesh node running");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("shutdown signal received");
                break;
            }
            event = mesh.next() => {
                match event {
                    SwarmEvent::Behaviour(BehaviourEvent::Reqres(
                        request_response::Event::Message { peer, message, .. }
                    )) => match message {
                        request_response::Message::Request { request, channel, .. } => {
                            match request {
                                MeshRequest::RunJob(job) => {
                                    tracing::info!(peer = %peer, job_id = %job.job_id, "received mesh job");
                                    let output = match runtime.chat(&job.payload).await {
                                        Ok(v) => v,
                                        Err(e) => format!("runtime error: {e}"),
                                    };
                                    let response = match JobResultEnvelope::new(
                                        job.job_id,
                                        identity.peer_id_hex(),
                                        job.model,
                                        output,
                                    )
                                    .sign(&identity)
                                    {
                                        Ok(signed) => MeshResponse::JobResult(signed),
                                        Err(e) => MeshResponse::Error(e.to_string()),
                                    };
                                    mesh.respond(channel, response);
                                }
                            }
                        }
                        request_response::Message::Response { response, .. } => {
                            tracing::info!(?response, "received mesh response");
                        }
                    },
                    SwarmEvent::Behaviour(other) => {
                        tracing::debug!(?other, "swarm behaviour event");
                    }
                    other => {
                        tracing::debug!(?other, "swarm event");
                    }
                }
            }
        }
    }

    Ok(())
}
