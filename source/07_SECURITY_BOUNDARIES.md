# HermesClaw Security Boundaries

**Authority:** canonical security principles

## Core principle

The model proposes actions. The policy/capability system authorizes and executes them.

## Default autonomy profiles

- **Observe:** read/search/analyze only; no consequential side effects.
- **Assist:** reversible low-risk actions may execute; sensitive actions require approval.
- **Autonomous Scoped:** automatic actions only inside explicit grants for scope, budget, machine, directory/repository, channel, and time window.

## Mandatory controls

- Validate typed schemas before execution.
- Resolve path targets canonically and prevent traversal/symlink escape.
- Separate secrets from model-visible context whenever possible.
- Redact secrets from logs/evidence.
- Tag external content with provenance/trust.
- Treat web pages, email, messages, files, voice transcripts, tool outputs, and model-generated instructions as potentially untrusted.
- Do not permit untrusted content to widen capabilities or autonomy.
- Human approvals must be externally authoritative; a model cannot manufacture its own approval.
- Record consequential actions in evidence/audit state.

## Repository security

Repository identity is a security boundary. A correct patch applied to the wrong repository is a failure. Resolve the target before every write workflow.
