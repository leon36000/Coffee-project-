# HermesClaw Current State

**Authority:** current operational state  
**Snapshot date:** 2026-08-13  
**Volatility:** high — reverify before repository/code actions

## Official repository

The intended official repository is `leon36000/Coffee-project-`.

Verified live on 2026-08-13:

- repository exists;
- default branch is `main`;
- connected account reports admin/maintain/push permissions;
- repository remains empty at the time of this checkpoint.

`leon36000/GitSpace` was not used or modified during this recovery checkpoint.

## Recovered local baseline

The recovery workspace `/mnt/data/HermesClaw` was verified to exist. The prior implementation history through `4e63056` was recovered, and canonical project memory was added in local commit `e5479d3`.

A shortened uncommitted README replacement was preserved outside the repository at `/mnt/data/hermesclaw-recovery/backups/README.uncommitted-2026-08-13.md`; the documented committed README was restored before verification.

## Verification performed now

The following gates were executed against the clean local baseline on 2026-08-13:

- `cargo fmt --all --check` — **verified now: pass**;
- `cargo test --workspace --exclude hermesclaw-desktop --offline` — **verified now: 14 passed, 0 failed**;
- `cargo clippy --workspace --exclude hermesclaw-desktop --all-targets --offline -- -D warnings` — **verified now: pass**;
- `npm --prefix apps/web test` — **verified now: 1 passed, 0 failed**;
- `npm --prefix apps/web run build` — **verified now: pass**;
- `npm --prefix apps/web run lint` — **verified now: pass**;
- `python source/raglite/validate_source.py` — **verified now: 19 documents validated**.

Rust 1.97.1 and the vendored dependency corpus were restored from checksum-verified local artifacts.

## Desktop verification

A local Tauri sysroot was reconstructed from 412 checksum-verified Debian packages, and the required `gtk+-3.0`, `webkit2gtk-4.1`, `ayatana-appindicator3-0.1`, and `librsvg-2.0` pkg-config contracts resolved successfully.

The complete `cargo check -p hermesclaw-desktop --offline` gate did not finish inside the tool execution windows while compiling the Tauri dependency graph. Therefore desktop compilation is **not currently verified** in this checkpoint; the prior green result remains historical until official CI reruns it.

## Publication blocker

The current project exposes GitHub repository reads but no GitHub create/update/commit/branch operations. The execution container also has no outbound network/DNS, so `git push` cannot reach GitHub.

The local baseline is packaged as a full-history Git bundle and tracked-source archives for transfer once a write-capable GitHub connection is available.

## Next safe action

1. enable a write-capable GitHub connection for `leon36000/Coffee-project-` or import the prepared Git bundle;
2. establish `main` from the recovered baseline;
3. run the repository CI, including the Tauri desktop gate;
4. record the official GitHub commit and CI run results here;
5. only then start the next capability migration.
