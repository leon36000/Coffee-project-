# HermesClaw Project Constitution

**Authority:** highest source-file authority  
**Scope:** all HermesClaw work  
**Last reviewed:** 2026-08-13

## Mission

HermesClaw is a **new, unified autonomous computer-agent application**. Its purpose is to absorb the strongest useful behaviors of Hermes Agent and OpenClaw while removing duplicated runtimes, duplicated state, duplicated scheduling, duplicated memories, and the feeling of two products connected together.

The final user experience must present one identity, one interface, one memory, one mission system, one permission model, one automation system, one device/network view, and one update mechanism.

## Non-negotiable architecture

1. **Rust is the canonical backend destination.**
2. **React + TypeScript is the canonical UI stack.**
3. **Tauri is the preferred desktop shell** unless evidence later supports a different choice.
4. Python and Node backend runtimes are temporary migration/reference tools, not permanent canonical backends.
5. There is exactly one canonical owner for each durable concern.
6. Migration unit = behavior/capability, not file or language syntax.
7. Compatibility adapters may translate or temporarily execute behavior; they may not own canonical truth.
8. A capability is not considered ported merely because it compiles. Tests, failure paths, policy, evidence, state compatibility, and acceptable performance are required.
9. Consequential actions must pass the central policy boundary. An LLM cannot approve its own privilege escalation.
10. The UI is a new HermesClaw interface, not an OpenClaw reskin.

## Repository boundary

**Official HermesClaw target repository:** `leon36000/Coffee-project-`

**Forbidden accidental target:** `leon36000/GitSpace`

GitSpace belonged to unrelated prior work. No HermesClaw mutation may target GitSpace unless the user explicitly names GitSpace in the current conversation.

## Source-of-truth rule

Markdown documents in this `source/` corpus are canonical memory. `SOURCE_MANIFEST.json` declares document authority and status. RAGLite is only a derived index.

If an agent retrieves an old but semantically similar statement from RAG, it must compare authority/status before using it.

## Definition of truth labels

- **Verified fact:** directly checked against current source, repository, test output, or authoritative external source.
- **Decision:** a chosen product/architecture rule; remains active until explicitly superseded.
- **Goal:** intended future behavior; not an implementation claim.
- **Historical:** useful past context; never current truth by itself.
- **Unknown:** unresolved; must not be filled by inference.

## Completion discipline

No unsupported completion claims. When verification cannot be rerun, the correct status is “last-known/historical validation, current verification unavailable,” not “passing.”
