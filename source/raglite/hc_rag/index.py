"""Build the derived RAGLite index from canonical Markdown source."""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
from urllib.parse import urlparse

from raglite import Document, insert_documents

from hc_rag.settings import DEFAULT_DB, config
from hc_rag.source_manifest import load_manifest


def _stable_id(path: str, content: str) -> str:
    # Content-sensitive IDs avoid incorrectly treating edited content as already indexed.
    return hashlib.sha256(f"{path}\0{content}".encode()).hexdigest()[:16]


def _safe_rebuild(db_url: str) -> None:
    parsed = urlparse(db_url)
    if parsed.scheme != "duckdb":
        raise RuntimeError("--rebuild only deletes the local DuckDB profile; rebuild PostgreSQL manually")
    db_path = Path(parsed.path)
    expected_root = DEFAULT_DB.parent.resolve()
    resolved = db_path.resolve()
    if expected_root not in resolved.parents and resolved != DEFAULT_DB.resolve():
        raise RuntimeError(f"refusing to delete DuckDB outside {expected_root}: {resolved}")
    for candidate in (resolved, resolved.with_suffix(resolved.suffix + ".wal"), resolved.with_suffix(".lock")):
        if candidate.exists():
            candidate.unlink()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rebuild", action="store_true", help="delete/rebuild the local derived DuckDB index")
    args = parser.parse_args()

    cfg = config()
    if args.rebuild:
        _safe_rebuild(str(cfg.db_url))

    documents = []
    for entry in load_manifest():
        if not entry.indexed:
            continue
        content = entry.absolute_path.read_text(encoding="utf-8")
        documents.append(
            Document.from_text(
                content,
                id=_stable_id(entry.path, content),
                filename=entry.path,
                project="HermesClaw",
                authority=entry.authority,
                status=entry.status,
                source_path=entry.path,
                priority=entry.priority,
            )
        )

    insert_documents(documents, config=cfg)
    print(f"Indexed {len(documents)} canonical/reference source documents into {cfg.db_url}")


if __name__ == "__main__":
    main()
