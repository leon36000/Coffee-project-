# HermesClaw Project Memory + RAG Design

**Status:** accepted by explicit user instruction to generate the project instructions, RAGLite setup, and Markdown source files before resuming implementation.

## Goal

Create a durable anti-drift context layer that survives chat/session boundaries and can be consumed by ChatGPT Projects, coding agents, and a local RAG service.

## Architecture

1. Markdown is canonical. The RAG database is derived and disposable.
2. Project instructions define agent behavior and source precedence.
3. Small, purpose-specific Markdown files separate facts, decisions, current state, architecture, workflow, roadmap, and history.
4. `SOURCE_MANIFEST.json` assigns authority/status metadata to each indexed document.
5. RAGLite 1.1.1 indexes the corpus into local DuckDB. The RAG layer is retrieval only; it never silently writes project truth.
6. Historical artifacts are clearly marked and cannot override current canonical files.

## Conflict resolution

Current user instruction > project instructions > constitution > canonical facts/decisions > current state > architecture/workflow > roadmap > historical references > RAG retrieval ranking.

RAG relevance never outranks authority.

## Anti-drift controls

- Official HermesClaw GitHub target is `leon36000/Coffee-project-`.
- `leon36000/GitSpace` is unrelated and forbidden for HermesClaw work unless the user explicitly asks in the current conversation.
- Before repository mutations, resolve repository + branch from GitHub; never infer them from old chat context.
- Never claim implementation/test status without current evidence or an explicitly labeled historical record.
- Every durable architecture concern has one canonical owner.
- Python/Node backend compatibility is temporary; Rust is the canonical backend destination and React/TypeScript the canonical UI stack.

## RAG model

RAGLite uses DuckDB by default because current upstream RAGLite 1.1.1 supports DuckDB or PostgreSQL. Runtime LLM/embedder identifiers are explicit environment configuration; no secret is stored in the corpus.

## Validation

The pack includes a standard-library validator that checks manifest consistency, file existence, unique priorities, required canonical files, and SHA-256 integrity.
