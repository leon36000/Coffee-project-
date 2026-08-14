# HermesClaw Memory and RAG Policy

**Authority:** canonical project-memory policy

## Two different memories

Do not confuse:

1. **HermesClaw product Memory Engine** — a future/runtime product subsystem.
2. **Project development memory** — these source documents plus a RAGLite index helping agents remember the project.

They are independent systems.

## Canonical memory

The `.md` files in `source/` are canonical. They should be concise enough to inspect directly and structured enough for retrieval.

## RAGLite role

RAGLite 1.1.1 indexes the canonical corpus into DuckDB for semantic + keyword retrieval. The database is disposable and can be deleted/rebuilt at any time.

The RAG database must never be the only location of a fact or decision.

## Authority vs relevance

A highly relevant historical chunk cannot override a less similar canonical decision. Retrieval consumers must use `authority`, `status`, and `priority` metadata from `SOURCE_MANIFEST.json`.

## Write-back rules

- Confirmed product fact -> `01_CANONICAL_FACTS.md`.
- Durable architecture/product choice -> `02_CANONICAL_DECISIONS.md`.
- Volatile operational state -> `03_CURRENT_STATE.md`.
- Work completed / next step -> `11_HANDOFF.md`.
- Long historical detail -> a clearly labeled historical reference file.

## Reindex policy

Rebuild after canonical source changes. For this small corpus, a full DuckDB rebuild is preferred over stale incremental records.

## Sensitive data

Do not put API keys, tokens, passwords, private keys, or unredacted secrets in project source or the RAG database.
