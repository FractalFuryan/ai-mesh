# Protocol

Core wire concepts for ai-mesh:

1. `JobEnvelope`
	- Contains `job_id`, requester identity, model hint, and message payload.
	- Includes created-at timestamp and expiration.
2. `SignedJobEnvelope`
	- Carries an envelope plus signature metadata.
	- Supports identity-bound validation by peers.
3. `Receipt`
	- Produced by worker nodes after execution.
	- Contains deterministic hash of request/response metadata and execution status.
4. Mesh transport behavior
	- request-response for job exchange.
	- identify + ping for peer health and capability discovery.

Future revisions should pin exact JSON schema, version negotiation, replay protection, and hash/signature algorithms.
