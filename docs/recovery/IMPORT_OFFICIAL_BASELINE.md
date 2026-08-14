# Import the HermesClaw Official Baseline

The preferred transfer artifact is `HermesClaw-official-baseline.bundle`, which preserves the complete recovered Git history.

## Import into an empty checkout

```bash
git clone -b main HermesClaw-official-baseline.bundle HermesClaw
cd HermesClaw
git remote remove origin
git remote add origin https://github.com/leon36000/Coffee-project-.git
git push -u origin main
```

Then inspect the GitHub Actions run created by `.github/workflows/ci.yml`. Do not continue capability migration until the Rust, web, and Tauri jobs are green.

## Source-only fallback

`HermesClaw-official-baseline.zip` and `.tar.gz` contain the same tracked tree but do not preserve Git history. Use them only when importing the bundle is impossible.

## Integrity

Verify the matching `.sha256` file before import:

```bash
sha256sum -c HermesClaw-official-baseline.bundle.sha256
```
