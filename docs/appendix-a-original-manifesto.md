# Appendix A — Original Manifesto (preserved)

*This is the original framing document for Ultranet, preserved verbatim. It has been superseded by `00-manifesto.md`, which widens the audience and reframes the "why." The technical content below remains load-bearing and is still correct; only the framing has changed.*

---

# Ultranet: The Sovereign Pivot
## A Personal Note on Why & How We Build the Real Internet

### The Core Thesis
The current internet backbone has been weaponized. It is a panopticon that translates digital breadcrumbs into kinetic strikes. Ultranet exists because networking is human connection, and human connection should not be a kill chain. This is the pivot — an internet built not for surveillance capitalism or military targeting, but for sovereign, unstoppable collaboration. The hardware vision of floating cylinders speaking via lasers is just the physical manifestation of this: communication that cannot be tapped, routed, or bombed from 7,000 miles away.

### Foundational Principles (Non-Negotiable)
- **Sovereign Topology:** The network has no center. No Tier 1 ISP. No DNS root. No BGP hijacking. Discovery is cryptographically assured rendezvous, not geographic broadcast.
- **Kinetic Resistance:** The ultimate form factor must be physically untethered from the grid and geographically ambiguous. No permanent address. No copper wire to trace.
- **Trustless Compute:** You share your hardware's cycles without sharing your hardware's memory. The node operator cannot see what is being computed, even if they hold the machine.
- **Dark by Default:** There is no "clearnet mode." Ultranet is the only mode. Traffic is indistinguishable from noise to anyone without the precise cryptographic handshake.

### The Path: From Rendezvous Code to Free-Space Optics
This is the decomposition of that vision into actionable technical milestones. We start with software that disappears you. We end with hardware that cannot be found.

#### Phase 1: The Software Fortress (v0.1 PoC)
Goal: A network where two nodes can find each other and compute without revealing an IP address or trusting a central authority.

| Component   | Technical Requirement                | Implementation Path                                                                                                                                                                                                                 |
|-------------|--------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Transport   | Tor-forked Onion Routing             | Use `arti` (Rust Tor) as a library. All libp2p streams are forced through SOCKS5 to the Arti daemon. No exceptions.                                                                                                                 |
| Rendezvous  | Hidden Service Logic                 | Implement Gosling or `tapir-rs`. Nodes have `.onion` addresses. They publish signed "compute capability" descriptors to a distributed hash table (DHT) running over the anonymous layer itself.                                     |
| Consensus   | Distributed Directory Authority      | Avoid the Tor DA trust problem. Use a Byzantine Fault Tolerant (BFT) gossip protocol among a set of bootstrap nodes, eventually moving to a cryptoeconomic stake model (inspired by Oxen/LokiNet Service Nodes).                    |
| Execution   | Blind Compute                        | Inference runs inside a Firecracker microVM. The host sees only encrypted RAM and CPU cycles. Attestation is a stub in v0.1, a full AMD SEV-SNP remote attestation in v1.0.                                                          |
| Egress      | Sovereign Gateway                    | Optional exit node role. Sign a policy: "This node will relay traffic to X domains only." No open internet passthrough.                                                                                                             |

#### Phase 2: The Hardware Pivot (The Laser Mesh)
Goal: Remove the ISP from the equation entirely. The network becomes a physical, line-of-sight mesh.

| Layer         | Specification                         | Purpose                                                                                                                                 |
|---------------|---------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------|
| Interconnect  | Free-Space Optical (FSO) Array        | 4x Infrared Laser Diodes with MEMS mirror steering. Data Rate: 10 Gbps+ per link. Security: Beam divergence < 1 mrad. Interception requires physically entering the beam path. |
| Network Stack | Geospatial Routing                    | Packets are routed based on physical angle and time-of-flight, not IP prefixes. Neighbor discovery via retroreflector scanning.         |
| Power         | Resonant Inductive Coupling           | Node hovers/rests on a charging pad. Battery buffer for 4–8 hours of mobility. No cables to cut.                                        |
| Form Factor   | Sealed Cylinder (V5)                  | Passive cooling via high-emissivity coating. Zero ingress points. Tamper-evident epoxy resin shell.                                     |

### The Threat Model We Are Engineering Against

| Attack                           | Mitigation                                                                                                                                                    |
|----------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Remote Exploit (Pegasus)         | The only open port is the Onion Service. Attack surface is minimized to the cryptographic handshake. Sandboxed execution prevents VM escape.                  |
| Physical Node Seizure            | Nodes contain no plaintext user data. Enclave memory is encrypted with keys held in a TPM that wipes on chassis intrusion.                                    |
| Traffic Correlation (NSA/GCHQ)   | Garlic bundling + constant-rate cover traffic (future integration with Nym mixnet). The network always looks busy.                                             |
| Kinetic Targeting (Airstrike)    | Nodes are mobile and untethered. There is no permanent infrastructure to target. The network reconstitutes ad-hoc.                                             |

### Summary: The Real Internet
This is the internet we should have built if the goal was human connection, not behavioral futures trading. Ultranet is the exit strategy.
