# HermesClaw Agent Startup Checklist

**Authority:** operational checklist
**Updated:** 2026-08-18

Before substantive work:

- [ ] Read `00_PROJECT_CONSTITUTION.md`.
- [ ] Read `01_CANONICAL_FACTS.md`.
- [ ] Read `02_CANONICAL_DECISIONS.md`.
- [ ] Read `03_CURRENT_STATE.md`.
- [ ] Read the task-specific architecture/workflow/source files.
- [ ] Retrieve `MCP_TO_PC.memory_project_context(project_1395c599202e486db911f3cdcc27d600, limit>=50)`.
- [ ] Keep only memory records for profile `hermesclaw-chatgpt-mobile-v1`.
- [ ] If Neon is available, query only project `holy-star-76486119`, database `neondb`, schema `hermesmem`; inspect `hot_context` before broad retrieval.
- [ ] Never use `MCP_TO_PC.memory_search` semantic results until its project-isolation bug is independently verified fixed.
- [ ] Never read/query/index/mutate `NextGen Memory` or GitSpace for HermesClaw.
- [ ] Resolve the official GitHub repository, default branch, current contents, permissions, and relevant worktree live.
- [ ] Distinguish merged truth, committed-local work, dirty WIP, historical reference, derived retrieval, and hypothesis.
- [ ] Reverify volatile Git/CI/worktree/tool facts before acting on remembered state.
- [ ] Identify the smallest capability/contract being changed.
- [ ] Determine what executable evidence will prove success before implementation.
- [ ] Use MCP TO PC and available agentic IDE/agent infrastructure when it materially improves rigor or throughput.
- [ ] Give independent agents isolated worktrees/responsibilities; keep one writer per worktree and one integration owner.
- [ ] Use VS Code/agentic debugging for hard failures when available; OpenClaw may operate pc1 terminal/desktop/GUI when appropriate, without bypassing policy or repository boundaries.

Before finishing work:

- [ ] Run relevant current verification; do not rely on remembered GREEN evidence if the tested content may have changed.
- [ ] Report failures honestly; do not hide them behind “best effort.”
- [ ] Update `03_CURRENT_STATE.md` if volatile operational truth changed materially.
- [ ] Update `01_CANONICAL_FACTS.md` for durable product facts.
- [ ] Update `02_CANONICAL_DECISIONS.md` only when a durable architectural/product decision changed.
- [ ] Update `11_HANDOFF.md` with exact commit/worktree/tests/blockers/next step/evidence.
- [ ] Append useful continuity records to the project-scoped MCP memory profile.
- [ ] Sync the dedicated HermesClaw Neon operational memory when available.
- [ ] Rebuild RAGLite/DuckDB if canonical source changed.
- [ ] Mark stale operational records superseded/expired rather than letting them compete with current truth.
