"""Dependency-free integrity checks for HermesClaw project source."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

SOURCE = Path(__file__).resolve().parent.parent
MANIFEST = SOURCE / "SOURCE_MANIFEST.json"
REQUIRED = {
    "00_PROJECT_CONSTITUTION.md",
    "01_CANONICAL_FACTS.md",
    "02_CANONICAL_DECISIONS.md",
    "03_CURRENT_STATE.md",
    "11_HANDOFF.md",
}


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    payload = json.loads(MANIFEST.read_text(encoding="utf-8"))
    docs = payload["documents"]
    paths = [item["path"] for item in docs]
    assert len(paths) == len(set(paths)), "duplicate manifest paths"
    assert REQUIRED.issubset(paths), f"missing required canonical files: {sorted(REQUIRED - set(paths))}"
    for item in docs:
        path = SOURCE / item["path"]
        assert path.is_file(), f"missing: {item['path']}"
        assert sha(path) == item["sha256"], f"hash mismatch: {item['path']}"
        assert isinstance(item["priority"], int), f"priority must be int: {item['path']}"
        assert item["status"] in {"canonical", "current", "reference", "historical"}, item
    assert max(item["priority"] for item in docs) == next(
        item["priority"] for item in docs if item["path"] == "00_PROJECT_CONSTITUTION.md"
    ), "constitution must hold highest manifest priority"
    print(f"OK: {len(docs)} source documents validated")


if __name__ == "__main__":
    main()
