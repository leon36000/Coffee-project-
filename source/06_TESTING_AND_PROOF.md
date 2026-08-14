# HermesClaw Testing and Proof Policy

**Authority:** canonical quality gate

## Test layers

- Unit: pure domain logic, parsing, policy, state transitions.
- Contract: models, capabilities, channels, nodes, storage, plugins, APIs.
- Differential: frozen upstream reference vs HermesClaw observable behavior.
- Integration: multiple HermesClaw components with real controlled persistence/services.
- End-to-end: user input -> model/agent -> policy -> capability -> evidence -> delivery/UI.
- Security: injection, traversal, shell injection, SSRF, secret exfiltration, cross-session leakage, approval bypass, malicious tool output, replay/bot loops, node impersonation.
- Resilience: crashes, restart, network loss, rate limits, partial tool failure, duplicates, cancellation, corrupt compatibility runtime.
- Performance: startup, idle memory, event latency, model/tool concurrency, mission recovery, persistence, gateway fanout.

## Definition of “ported”

A capability is ported only when:

- its contract is documented;
- tests exist and pass;
- differential comparison passes where applicable;
- policy/security checks pass;
- cancellation/error paths are proven;
- evidence/telemetry is present;
- state/migration compatibility is demonstrated;
- performance is acceptable for the target;
- the old compatibility fallback can be disabled for that capability.

## No fake evidence

Generated code, screenshots of code, a successful compile alone, or a model's own statement are not sufficient completion evidence.
