# `workspace.read` Verification Evidence — 2026-08-14

## Verification target

- repository: `leon36000/Coffee-project-`;
- merged pull request: #8;
- merge commit: `ec3cfff0e938d4bc5e17d36d30a6d84f52cbdd19`;
- feature implementation commit before squash: `67026ab4eff7d7c895d746dcf0a641f85a73a44b`;
- approved specification: `docs/superpowers/specs/2026-08-14-workspace-read-design.md`.

The target is a bounded read-only text-file capability, not a general filesystem framework.

## Requirement-to-evidence matrix

| Requirement or risk | Direct evidence | Result |
|---|---|---|
| Canonical domain call | `workspace_read_constructor_is_low_risk_read_only` | Pass |
| Existing list behavior preserved | list regression plus agent/API list tests | Pass |
| Result/evidence separation | `capability_execution_can_hold_different_result_and_evidence` | Pass |
| UTF-8 text read and exact byte count | `workspace_read_returns_text_and_sanitized_evidence` | Pass |
| SHA-256 evidence, no content field | tools, agent, Axum, and React tests | Pass |
| Parent and absolute escape rejection | dedicated `hc-tools` tests | Pass |
| External symlink escape rejection | Unix security test | Pass |
| Internal symlink canonical target | Unix behavior test | Pass |
| Directory rejection | dedicated `hc-tools` test | Pass |
| `65_537` byte rejection | dedicated limit test | Pass |
| Invalid UTF-8 and NUL rejection | dedicated content tests | Pass |
| Observe policy before execution | existing policy test plus read agent mission | Pass |
| Deterministic and OpenAI codecs | five `hc-models` tests | Pass |
| HTTP and SQLite evidence boundary | two Axum integration tests | Pass |
| React response/evidence boundary | two Vitest tests | Pass |
| Desktop registration/compile | Tauri compile gate | Pass |

## TDD evidence

GitHub Actions run `31844182703`, Rust job `94907147108`, failed for the intended compile error: `ToolCall::workspace_read` did not exist. The minimal constructor implementation was added only after this red result.

Subsequent local commits preserved test-first ordering for the capability contract, security behavior, model codecs, agent evidence, API integration, and UI behavior.

## Local executable evidence

Environment:

- Rust `1.97.1`;
- locked Cargo graph with checksum-verified vendored dependencies;
- Node.js `22.16.0`;
- checksum-verified Tauri Debian package set containing 412 packages.

Commands and results:

```text
cargo fmt --all --check
PASS

cargo test --workspace --exclude hermesclaw-desktop --locked --offline
30 passed; 0 failed

cargo clippy --workspace --exclude hermesclaw-desktop --all-targets --locked --offline -- -D warnings
PASS

npm --prefix apps/web test
2 passed; 0 failed

npm --prefix apps/web run lint
PASS

npm --prefix apps/web run build
PASS

cargo check -p hermesclaw-desktop --locked --offline
PASS

python source/raglite/validate_source.py
OK: 19 source documents validated

git diff --check
PASS
```

Rust test distribution:

- `hc-agent`: 2;
- `hc-api` integration: 2;
- `hc-domain`: 3;
- `hc-mission`: 2;
- `hc-models`: 5;
- `hc-policy`: 2;
- `hc-state`: 1;
- `hc-tools`: 13;
- total: 30.

## GitHub Actions evidence

- branch run `31846047390`: Rust job `94912572591`, Web job `94912572486`, Tauri job `94912572553` — success;
- PR run `31846845864`: Rust job `94914955207`, Web job `94914955743`, Tauri job `94914955313` — success;
- post-merge `main` run `31847082077`: Rust job `94915614768`, Web job `94915614867`, Tauri job `94915614757` — success.

Transfer provenance was separately verified by one-shot run `31845995476`, job `94912417197`, using patch SHA-256 `dbe301a1065cbd82f7c1b19ef9e8a372a20e19f1b572c9eab5f91271d55a878a`. All transfer files and the one-shot workflow were removed before PR review.

## Security and privacy result

The model-visible result includes the authorized file text. The persisted `capability_execution` evidence includes only:

```json
{
  "path": "alpha.txt",
  "bytes": 17,
  "sha256": "<64 lowercase hexadecimal characters>"
}
```

Agent and Axum tests assert that neither a `content` field nor the sentinel text `alpha secret text` exists in persisted evidence. The React test independently asserts that the response renders the text while the evidence panel does not.

## Independent engineering review

**Specification verdict:** Pass for the approved bounded slice.

**Engineering-quality verdict:** Pass with one explicit residual limitation.

No material correctness, policy, evidence-redaction, boundary, codec, integration, or regression finding remained after executable checks.

Tool availability:

- Fallow CLI was not installed, so no graph-grounded Fallow judgment is claimed;
- no `sonar-project.properties`, Sonar scanner, or SonarQube CLI was present, so no SonarQube result is claimed.

## Residual limitation

The implementation canonicalizes and inspects a target before `tokio::fs::read`. It rechecks the actual byte count after reading, so oversized content is not returned. It does not, however, use an OS-specific descriptor-relative/openat-style broker that prevents a hostile local process from replacing the target between checks. This tranche therefore does not claim adversarial filesystem TOCTOU hardening.

## Explicit non-claims

This evidence does not establish support for:

- binary files;
- invalid UTF-8 recovery;
- partial/range reads;
- streaming reads;
- writes, patches, deletion, or rename;
- process execution;
- race-free hostile-local-filesystem operation;
- completion of the broader Hermes/OpenClaw migration.
