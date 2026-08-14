# Local Baseline Verification — 2026-08-13

## Scope

Recovered HermesClaw vertical proof at local history `4e63056`, plus canonical source-memory commit `e5479d3`.

## Current successful gates

```text
cargo fmt --all --check
PASS

cargo test --workspace --exclude hermesclaw-desktop --offline
14 passed; 0 failed

cargo clippy --workspace --exclude hermesclaw-desktop --all-targets --offline -- -D warnings
PASS

npm --prefix apps/web test
1 passed; 0 failed

npm --prefix apps/web run build
PASS

npm --prefix apps/web run lint
PASS

python source/raglite/validate_source.py
OK: 19 source documents validated
```

## Desktop gate

The local Tauri sysroot was reconstructed from 412 Debian packages whose supplied SHA-256 manifest verified successfully. `pkg-config` resolved:

- GTK 3.24.49;
- WebKitGTK 2.52.5;
- Ayatana AppIndicator 0.5.94;
- librsvg 2.60.0.

`cargo check -p hermesclaw-desktop --offline` advanced through the Tauri dependency graph but did not complete inside the available command windows. Desktop compilation is therefore not claimed as current evidence.

## Publication state

The official repository was verified as `leon36000/Coffee-project-`, default branch `main`, empty, with administrative permissions reported by the connected GitHub account. Remote publication is blocked because this project exposes no GitHub write operations and the local container has no outbound GitHub network access.
