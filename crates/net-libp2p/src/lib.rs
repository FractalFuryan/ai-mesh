use futures::StreamExt;
use libp2p::{
    gossipsub::{self, IdentTopic, MessageAuthenticity},
    identify, identity, noise, ping,
    request_response::{self, json, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm,
};
use node_core::{JobEnvelope, JobResultEnvelope, NodeCapability};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const CAPABILITY_TOPIC: &str = "ai-mesh/capabilities";

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
    pub reqres: json::Behaviour<MeshRequest, MeshResponse>,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
}

#[derive(Debug, Error)]
pub enum NetError {
    #[error("build error: {0}")]
    Build(String),
    #[error("dial error: {0}")]
    Dial(String),
}

pub struct MeshNode {
    pub peer_id: PeerId,
    pub swarm: Swarm<Behaviour>,
}

impl MeshNode {
    pub async fn new(listen_addr: Multiaddr) -> Result<Self, NetError> {
        let local_key = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(local_key.public());

        let reqres = json::Behaviour::new(
            [(
                request_response::StreamProtocol::new("/ai-mesh/job/1.0.0"),
                ProtocolSupport::Full,
            )],
            request_response::Config::default(),
        );

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .build()
            .map_err(|e| NetError::Build(e.to_string()))?;

        let gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Author(peer_id),
            gossipsub_config,
        )
        .map_err(|e| NetError::Build(e.to_string()))?;

        let behaviour = Behaviour {
            reqres,
            identify: identify::Behaviour::new(identify::Config::new(
                "/ai-mesh/0.1.0".into(),
                local_key.public(),
            )),
            ping: ping::Behaviour::new(ping::Config::new()),
            gossipsub,
        };

        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| NetError::Build(e.to_string()))?
            .with_behaviour(|_| behaviour)
            .map_err(|e| NetError::Build(e.to_string()))?
            .build();

        swarm
            .listen_on(listen_addr)
            .map_err(|e| NetError::Build(e.to_string()))?;

        let topic = IdentTopic::new(CAPABILITY_TOPIC);
        swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic)
            .map_err(|e| NetError::Build(e.to_string()))?;

        Ok(Self { peer_id, swarm })
    }

    pub fn dial(&mut self, addr: Multiaddr) -> Result<(), NetError> {
        self.swarm
            .dial(addr)
            .map_err(|e| NetError::Dial(e.to_string()))
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

    pub fn send_response(
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

    pub async fn next_event(&mut self) -> Option<SwarmEvent<BehaviourEvent>> {
        self.swarm.next().await
    }

    pub fn publish_capability(&mut self, cap: &NodeCapability) -> Result<(), NetError> {
        let topic = IdentTopic::new(CAPABILITY_TOPIC);
        let data = serde_json::to_vec(cap).map_err(|e| NetError::Build(e.to_string()))?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, data)
            .map_err(|e| NetError::Build(e.to_string()))?;
        Ok(())
    }
}

pub fn extract_request(
    event: &SwarmEvent<BehaviourEvent>,
) -> Option<(
    PeerId,
    MeshRequest,
    request_response::ResponseChannel<MeshResponse>,
)> {
    if let SwarmEvent::Behaviour(BehaviourEvent::Reqres(request_response::Event::Message {
        peer,
        message:
            request_response::Message::Request {
                request, channel, ..
            },
        ..
    })) = event
    {
        Some((*peer, request.clone(), channel.clone()))
    } else {
        None
    }
}

pub fn extract_capability(
    event: &SwarmEvent<BehaviourEvent>,
) -> Option<(PeerId, NodeCapability)> {
    if let SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(gossipsub::Event::Message {
        propagation_source,
        message,
        ..
    })) = event
    {
        let capability = serde_json::from_slice::<NodeCapability>(&message.data).ok()?;
        Some((*propagation_source, capability))
    } else {
        None
    }
}
