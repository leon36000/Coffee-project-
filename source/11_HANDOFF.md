# HermesClaw Handoff

**Authority:** current handoff checkpoint
**Updated:** 2026-08-18

## Canonical direction

- Official HermesClaw GitHub target: `leon36000/Coffee-project-`.
- GitSpace is explicitly excluded.
- `NextGen Memory` is a separate user project and is explicitly excluded from HermesClaw development memory.
- Backend destination: Rust.
- UI: React + TypeScript.
- Desktop shell: Tauri.
- Markdown in `source/` is canonical memory; operational ledgers and RAG are derived/continuity infrastructure.

## Merged repository baseline

GitHub `main` was reverified during the 2026-08-18 memory consolidation:

- current merged checkpoint SHA: `1ed1e33b07a53826974f591b5d1d7d0d6c917837`;
- canonical `workspace.read` merge commit: `ec3cfff0e938d4bc5e17d36d30a6d84f52cbdd19`;
- merged pull request: #8;
- documentation checkpoint: PR #9;
- post-merge GitHub Actions run `31847082077` succeeded for Rust/Web/Tauri.

`workspace.read` remains the last merged product capability checkpoint represented by canonical `main`. Do not confuse later local workspace.write/Approval/SQLite worktree commits with merged GitHub truth.

## Current local workspace.write / Approval / SQLite lineage

The clean reconstruction continued in isolated pc1 worktrees.

Verified durable local checkpoints include:

- Task 3c Approval Engine closure: `e69cbb5ba796e56019249d3a2cc7551473f1ec69`;
- Task 4 SQLite approval repository: `44c8187`;
- typed approval decision actor fix: `2d016a5`;
- storage invariant hardening: `8d1c29c`;
- versioned canonical SQLite state schema: `a4b072c012650070bdc9df80b2529473913f30fd`.

These are local/worktree checkpoints, not merged `main`.

The active Task 4 worktree/controller project is `project_3e79efb12456421e9a84e9ab352c4a3a`. Live status on 2026-08-18 still showed exactly two modified files:

- `crates/hc-state/src/lib.rs`;
- `crates/hc-state/tests/state_migrations.rs`.

The uncommitted review fix hardens version-1 SQLite schema validation against deceptive constraints and wrong index definitions. Do not reset or overwrite this worktree.

A GREEN measurement of that hardening recorded:

- state migration tests: **7 passed, 0 failed**;
- approval repository tests: **18 passed, 0 failed**;
- full `hc-state`: **26 passed, 0 failed**;
- full `hc-policy`: **25 passed, 0 failed**;
- full `hc-domain`: **8 passed, 0 failed**;
- strict Clippy for `hc-state`, `hc-policy`, `hc-domain`: pass with zero warnings;
- `git diff --check`: pass.

Because the work is still uncommitted, rerun the relevant gates on the exact current diff immediately before any completion/commit claim.

## ChatGPT mobile development memory

Project continuity profile:

- MCP profile: `hermesclaw-chatgpt-mobile-v1`;
- canonical MCP TO PC project: `project_1395c599202e486db911f3cdcc27d600`;
- dedicated Neon project: `holy-star-76486119` (`HermesClaw ChatGPT Memory`);
- Neon branch: `br-jolly-cherry-axvnmasv`;
- database/schema: `neondb.hermesmem`.

On 2026-08-18 the complete project-scoped MCP memory snapshot was imported into the dedicated Neon store and the V2 operational decisions were synchronized. Verification showed **27 memory records** after deployment of the retrieval API.

The schema includes project/profile isolation, forced RLS, provenance, authority/epistemic/verification state, truth plane, integration state, temporal validity, full-text retrieval, optional pgvector embeddings, checkpoints, relations, retrieval receipts, utility feedback, and sync receipts. Isolation probes attempting a NextGen profile and an unrelated project identity were rejected. `pgvector 0.8.6` and `pgcrypto 1.4` are active.

Verified retrieval functions:

- `hermesmem.context_pack(query, limit)` — HOT continuity first; exact next action and dirty blocker precede broad checkpoints/procedures;
- `hermesmem.search_lexical(query, limit)` — PostgreSQL full-text retrieval;
- `hermesmem.search_hybrid(query, embedding, limit)` — lexical + exact pgvector candidates combined with RRF when a compatible embedding is supplied.

No embeddings are currently populated and no HNSW/IVFFlat index is installed. This is intentional until a stable embedding model/dimension and measured scale requirement exist.

No connection secret is stored in project memory.

Known memory-tool blocker: `MCP_TO_PC.memory_search` semantic results previously leaked unrelated project memories despite a HermesClaw project filter. Do not use that semantic backend until isolation is independently proven fixed. Use `memory_project_context` plus the dedicated HermesClaw Neon store instead.

## Infrastructure execution

The canonical checkout is `/home/pc1/HermesClaw`. Available infrastructure has included MCP TO PC, Codex, Claude Code, OpenHands, OpenClaw desktop/GUI and agentic IDE/debug facilities. Reverify actual tool health each session.

Use independent agents in isolated worktrees for bounded responsibilities and keep one integration writer. OpenClaw may operate the pc1 terminal/desktop/GUI when useful, but it does not bypass policy, repository boundaries, or evidence requirements.

## Historical workspace.write archaeology constraint

Historical remote `agent/workspace-write-design` was verified at `92166d3eb86a1490f0c21093aa08f8268d2d129e`, 94 commits ahead of the old main merge-base. It contains useful behavior mixed with temporary workflows/bootstrap transport.

Never merge or cherry-pick that branch wholesale. Continue reconstructing bounded capabilities from verified behavior with TDD.

## Exact next action

Resume from the protected Task 4 dirty worktree, not from historical branches:

1. verify the exact current diff and rerun Task 4 gates on that exact content;
2. fix only observed failures;
3. commit the schema-validation review fix only with current executable evidence;
4. obtain independent review on the immutable commit SHA;
5. fast-forward/clean the intended Task 4 and integration worktrees;
6. only after Tasks 1–4 combined gates are green, begin Task 5 create-only `workspace.write` executor with filesystem-security RED tests;
7. keep Mission Durability deferred until workspace.write vertical stabilization and reuse the existing Approval/crypto/key-provider/state owners.
