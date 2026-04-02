use futures::StreamExt;
use libp2p::{
    identify,
    identity,
    noise,
    ping,
    request_response::{self, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm,
};
use node_core::{JobEnvelope, JobResultEnvelope};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type MeshBehaviour = request_response::json::Behaviour<MeshRequest, MeshResponse>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeshRequest {
    RunJob(JobEnvelope),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeshResponse {
    JobResult(JobResultEnvelope),
    Error(String),
}

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    pub reqres: MeshBehaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
}

#[derive(Debug, Error)]
pub enum NetError {
    #[error("build error: {0}")]
    Build(String),
}

pub struct MeshNode {
    pub peer_id: PeerId,
    pub swarm: Swarm<Behaviour>,
}

impl MeshNode {
    pub async fn new(listen_addr: Multiaddr) -> Result<Self, NetError> {
        let local_key = identity::Keypair::generate_ed25519();

        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| NetError::Build(e.to_string()))?
            .with_behaviour(|key| Behaviour {
                reqres: request_response::json::Behaviour::new(
                    [(
                        request_response::StreamProtocol::new("/ai-mesh/job/1.0.0"),
                        ProtocolSupport::Full,
                    )],
                    request_response::Config::default(),
                ),
                identify: identify::Behaviour::new(identify::Config::new(
                    "/ai-mesh/0.1.0".into(),
                    key.public(),
                )),
                ping: ping::Behaviour::new(ping::Config::new()),
            })
            .map_err(|e| NetError::Build(e.to_string()))?
            .build();

        swarm
            .listen_on(listen_addr)
            .map_err(|e| NetError::Build(e.to_string()))?;

        Ok(Self {
            peer_id: *swarm.local_peer_id(),
            swarm,
        })
    }

    pub fn send_job(
        &mut self,
        peer: &PeerId,
        job: JobEnvelope,
    ) -> request_response::OutboundRequestId {
        self.swarm
            .behaviour_mut()
            .reqres
            .send_request(peer, MeshRequest::RunJob(job))
    }

    pub fn respond(
        &mut self,
        channel: request_response::ResponseChannel<MeshResponse>,
        response: MeshResponse,
    ) {
        let _ = self
            .swarm
            .behaviour_mut()
            .reqres
            .send_response(channel, response);
    }

    pub async fn next(&mut self) -> SwarmEvent<BehaviourEvent> {
        self.swarm.select_next_some().await
    }
}
