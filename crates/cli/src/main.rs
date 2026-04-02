use anyhow::{Context, Result};
use api::{router, ApiState};
use clap::{Parser, Subcommand};
use libp2p::{request_response, swarm::SwarmEvent, Multiaddr};
use model_runtime::LlamaRuntime;
use net_libp2p::{BehaviourEvent, MeshNode, MeshRequest, MeshResponse};
use node_core::{JobResultEnvelope, NodeIdentity};
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Debug, Parser)]
#[command(name = "ai-mesh", version, about = "Decentralized local-first AI mesh node")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run {
        #[arg(long, default_value = "/ip4/127.0.0.1/tcp/9000")]
        p2p_listen: String,
        #[arg(long, default_value = "127.0.0.1:8080")]
        api_listen: String,
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        llama_base_url: String,
        #[arg(long, default_value = "local-model")]
        model_name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run {
            p2p_listen,
            api_listen,
            llama_base_url,
            model_name,
        } => {
            let identity = NodeIdentity::new();
            tracing::info!(peer = %identity.peer_id_hex(), "node identity created");

            let runtime = Arc::new(LlamaRuntime::new(llama_base_url, model_name));
            let api_state = ApiState {
                runtime: runtime.clone(),
            };

            let listener = TcpListener::bind(&api_listen)
                .await
                .with_context(|| format!("failed to bind api listener on {api_listen}"))?;

            tokio::spawn(async move {
                if let Err(err) = axum::serve(listener, router(api_state)).await {
                    tracing::error!(error = %err, "api server exited with error");
                }
            });

            let p2p_addr: Multiaddr = p2p_listen
                .parse()
                .with_context(|| format!("invalid p2p multiaddr: {p2p_listen}"))?;
            let mut mesh = MeshNode::new(p2p_addr).await?;

            tracing::info!(peer = %mesh.peer_id, api_listen = %api_listen, "ai-mesh node running");

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
                                                Ok(value) => value,
                                                Err(err) => format!("runtime error: {err}"),
                                            };

                                            let response = match JobResultEnvelope::new(
                                                job.job_id,
                                                identity.peer_id_hex(),
                                                job.model,
                                                output,
                                            ).sign(&identity) {
                                                Ok(signed) => MeshResponse::JobResult(signed),
                                                Err(err) => MeshResponse::Error(err.to_string()),
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
                                tracing::debug!(?other, "other swarm behaviour event");
                            }
                            other => {
                                tracing::debug!(?other, "swarm event");
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
