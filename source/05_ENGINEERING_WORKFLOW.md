# HermesClaw Engineering Workflow

**Authority:** canonical implementation workflow

## Every capability migration

1. Resolve current repository/branch and verify a clean baseline.
2. Identify the user-visible behavior to preserve or improve.
3. Read relevant upstream docs/tests from the frozen baseline.
4. Classify the capability: reference/adapt/port/merge/redesign/drop/new.
5. Write a small HermesClaw contract.
6. Capture deterministic fixtures or golden observations.
7. Write a failing HermesClaw test first.
8. Confirm the failure is caused by missing behavior, not a broken harness.
9. Implement the smallest behavior that passes.
10. Run unit/contract tests.
11. Run differential tests where upstream parity matters.
12. Run security and failure/cancellation tests.
13. Run integration/E2E gates for affected vertical slices.
14. Record evidence.
15. Cut over only after proof.
16. Remove compatibility paths only when retirement criteria are satisfied.

## Commit discipline

- Small, purpose-specific commits.
- Do not mix unrelated refactors with a capability port.
- Never stage unrelated user work.
- Never push to an inferred repository.

## Verification language

Use precise states:

- **verified now** — command/output checked in current environment/CI;
- **last-known green** — prior evidence exists but has not been rerun;
- **not verified** — no current proof;
- **blocked** — verification cannot run and reason is known.

Never collapse these into “works.”

## Required end-of-session write-back

Update:

- `03_CURRENT_STATE.md` — exact verified state;
- `02_CANONICAL_DECISIONS.md` — only if a durable decision changed;
- `11_HANDOFF.md` — what changed, exact evidence, blockers, next action.

Then rebuild the RAG index if local RAG is in use.
