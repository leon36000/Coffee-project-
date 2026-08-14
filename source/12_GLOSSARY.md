# HermesClaw Glossary

**Authority:** terminology reference

- **HermesClaw:** the unified product being built; not merely a bridge between Hermes Agent and OpenClaw.
- **Capability:** typed action that an agent may request; execution is owned by the capability/policy pipeline.
- **Mission:** durable unit of multi-step work with state, budgets, actions, approvals, evidence, artifacts, and result.
- **Policy Kernel:** central authority deciding whether a capability may execute.
- **Model Fabric:** provider-neutral model abstraction and routing layer.
- **Evidence:** structured proof of a meaningful decision/action/test, distinct from ordinary logs.
- **Provenance:** origin/trust metadata attached to external/model/tool content.
- **Compatibility adapter:** temporary boundary to an upstream runtime while behavior is being migrated; never canonical truth.
- **Differential test:** comparison of frozen upstream observable behavior with the HermesClaw replacement.
- **Port:** behavior reimplemented behind HermesClaw contracts and proven sufficiently to disable its fallback.
- **RAGLite:** external Python RAG toolkit used here only for project-development recall.
- **Project source:** canonical Markdown corpus uploaded to the project and indexed by RAGLite.
- **Frozen baseline:** a specific upstream version/commit deliberately held stable for reproducible migration analysis.
