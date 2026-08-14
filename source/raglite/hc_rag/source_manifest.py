"""Read and validate the authority-ranked HermesClaw source manifest."""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path

SOURCE_ROOT = Path(__file__).resolve().parents[2]
MANIFEST_PATH = SOURCE_ROOT / "SOURCE_MANIFEST.json"


@dataclass(frozen=True)
class SourceEntry:
    path: str
    title: str
    authority: str
    status: str
    priority: int
    sha256: str
    indexed: bool

    @property
    def absolute_path(self) -> Path:
        return SOURCE_ROOT / self.path


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_manifest(*, verify_hashes: bool = True) -> list[SourceEntry]:
    payload = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    entries = [SourceEntry(**entry) for entry in payload["documents"]]
    paths = [entry.path for entry in entries]
    if len(paths) != len(set(paths)):
        raise ValueError("SOURCE_MANIFEST.json contains duplicate paths")
    for entry in entries:
        if not entry.absolute_path.is_file():
            raise FileNotFoundError(entry.absolute_path)
        if verify_hashes and sha256_file(entry.absolute_path) != entry.sha256:
            raise ValueError(f"hash mismatch: {entry.path}")
    return sorted(entries, key=lambda item: item.priority, reverse=True)
