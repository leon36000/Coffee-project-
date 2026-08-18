# HermesClaw Memory and RAG Policy

**Authority:** canonical project-memory policy
**Updated:** 2026-08-18

## Three distinct memory concerns

Do not confuse:

1. **HermesClaw product Memory Engine** — a runtime/product subsystem governed by HermesClaw architecture.
2. **Canonical development memory** — Markdown documents in `source/`; this is the durable project source of truth.
3. **Operational development continuity** — project-scoped ledgers/retrieval stores used by ChatGPT mobile and engineering agents to resume long work without drift.

`NextGen Memory` is a completely separate user project. It is not a HermesClaw memory profile, source, index, test corpus, dependency, or fallback. Do not read, query, index, mutate, or infer HermesClaw state from it unless the user explicitly asks about that separate project in the current conversation.

## Canonical memory

The `.md` files in `source/` are canonical. They must remain inspectable, versioned, and sufficient to reconstruct durable product facts, decisions, state, and handoff without any external memory database.

Authority precedence remains:

current user instruction > project instructions > `00_PROJECT_CONSTITUTION.md` > `01_CANONICAL_FACTS.md` / `02_CANONICAL_DECISIONS.md` > `03_CURRENT_STATE.md` > architecture/workflow > roadmap > historical references.

Retrieval relevance never overrides this authority order.

## ChatGPT mobile operational continuity profile

The dedicated development-memory profile is:

- profile: `hermesclaw-chatgpt-mobile-v1`;
- MCP TO PC project: `project_1395c599202e486db911f3cdcc27d600`;
- official repository: `leon36000/Coffee-project-`;
- dedicated Neon project: `holy-star-76486119` (`HermesClaw ChatGPT Memory`);
- database/schema: `neondb.hermesmem`.

This operational memory is a continuity/retrieval layer only. It may contain hot blockers, exact next actions, local-worktree state, verified checkpoints, historical milestones, retrieval receipts, and utility feedback. It must not silently promote any record into canonical truth.

Do not store Neon connection strings, database passwords, API keys, tokens, private keys, or unredacted credentials in project source, MCP memory, Neon memory records, or RAG.

## Memory tiers

- **Tier 0 — canonical:** `source/*.md` plus live Git/CI evidence required to verify volatile claims.
- **Tier 1 — hot:** active dirty worktree, blocker, exact next action, current branch/SHA, pending review.
- **Tier 2 — durable episodic:** verified capability checkpoints, decisions, tests, CI runs, and handoffs.
- **Tier 3 — historical:** genesis, superseded branches/artifacts, archaeology, failed approaches.
- **Tier 4 — derived search:** FTS/vector embeddings/RAG indexes. Disposable and never authoritative.

## Safe retrieval order

For every substantive HermesClaw session:

1. Read `00_PROJECT_CONSTITUTION.md`, `01_CANONICAL_FACTS.md`, `02_CANONICAL_DECISIONS.md`, and `03_CURRENT_STATE.md` in that order.
2. Read task-specific architecture/workflow/source files.
3. Retrieve `MCP_TO_PC.memory_project_context` for `project_1395c599202e486db911f3cdcc27d600` with a sufficiently large limit and keep only records for `hermesclaw-chatgpt-mobile-v1`.
4. When the Neon connector is available, query only `holy-star-76486119` / `neondb` / `hermesmem`, starting with `hot_context`, then task-relevant current records.
5. Apply project/profile isolation, active/superseded status, authority, and verification state before lexical/vector relevance.
6. Surface active blockers and exact next action before broad history.
7. Reverify volatile Git, CI, branch, worktree, permissions, and tool-availability facts live before mutation.

`MCP_TO_PC.memory_search` semantic results are prohibited for HermesClaw until its cross-project isolation bug is independently demonstrated fixed. Use `memory_project_context` instead.

If Neon is unavailable in a future session, continue from canonical source plus safe MCP project context rather than stalling or switching to another project's database.

Retrieval is context, never authorization.

## Neon operational schema policy

The dedicated HermesClaw Neon schema must preserve:

- hard project/profile isolation before ranking;
- provenance/source records;
- immutable or supersedable memory records rather than destructive overwrite;
- authority, epistemic status, verification state, truth plane, and integration state;
- occurred/recorded/validity/freshness timestamps;
- Git SHA/ref/worktree and evidence references when relevant;
- deterministic full-text retrieval;
- optional pgvector embeddings with model/dimension metadata;
- checkpoints, relations, retrieval receipts, sync receipts, and utility feedback;
- row-level security or stronger equivalent isolation.

At small corpus sizes, exact vector search is acceptable. Add approximate indexes only after a stable embedding dimension and measured need; filtering/isolation must still happen before results are trusted.

## RAGLite / DuckDB role

RAGLite 1.1.1 indexes the canonical corpus for local semantic + keyword retrieval. Its DuckDB database remains disposable and may be deleted/rebuilt at any time.

RAGLite/DuckDB is not the long-term operational continuity owner and must never be the only location of a fact or decision. Prefer full rebuild after canonical source changes rather than carrying stale derived records.

## Write-back and consolidation

Write operational memory only for future-useful information: verified facts, decisions, meaningful checkpoints, blockers, exact next actions, procedures, and historical milestones. Do not store every chat turn or hidden chain-of-thought.

Prefer append + supersede over silent destructive updates. Every meaningful record should carry provenance/evidence and Git SHA or worktree identity when applicable.

At the end of meaningful work:

- confirmed durable product fact -> update `01_CANONICAL_FACTS.md` or `03_CURRENT_STATE.md`;
- durable architecture/product choice -> update `02_CANONICAL_DECISIONS.md`;
- work completed / blocker / exact next step -> update `11_HANDOFF.md`;
- append/update the project-scoped MCP continuity ledger;
- sync the dedicated HermesClaw Neon operational memory when available;
- rebuild RAGLite/DuckDB if canonical Markdown changed.

The canonical Markdown write must happen before a derived memory/index is allowed to represent a durable fact as current.
