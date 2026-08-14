# Official HermesClaw Baseline and CI Evidence — 2026-08-14

## Repository

- Target: `leon36000/Coffee-project-`
- Default branch: `main`
- Exact recovered baseline: `c3fd965d438a5a471c969f3fa1b447e3b6e34b31`
- Current verified commit: `2dff93bd73eda777a6e305ee5a658d146e5aa6c6`
- Baseline source provenance: `d79d2ebb3c7a0092f833d5883f1522954566e031`

## GitHub Actions evidence

### Branch

Run `31833887410` on `agent/fix-web-ci-prefix`: success.

### Pull Request

Run `31837393708` for PR #6: success.

### Post-merge main

Run `31837639423` on commit `2dff93bd73eda777a6e305ee5a658d146e5aa6c6`: success.

Completed jobs:

- Rust proof gates — success;
- Web proof gates — success;
- Tauri desktop compile gate — success.

## Local corroboration

Executed against a clean clone restored from the verified Git bundle using Rust 1.97.1 and checksum-verified vendored dependencies:

```text
cargo fmt --all --check
PASS

cargo test --workspace --exclude hermesclaw-desktop --locked --offline
14 passed; 0 failed

cargo clippy --workspace --exclude hermesclaw-desktop --all-targets --locked --offline -- -D warnings
PASS

npm test
1 passed; 0 failed

npm run lint
PASS

npm run build
PASS
```

## Scope of the proof

This evidence verifies repository establishment and the existing architectural vertical proof. It does not prove later Hermes/OpenClaw capability migration waves.
