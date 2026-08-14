"""Search HermesClaw project memory and emit authority-aware results."""

from __future__ import annotations

import argparse
import json

from raglite import hybrid_search, search_and_rerank_chunk_spans

from hc_rag.settings import config


def _first(value: object, default: object = None) -> object:
    if isinstance(value, list):
        return value[0] if value else default
    return value if value is not None else default


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("query")
    parser.add_argument("--k", type=int, default=6)
    args = parser.parse_args()

    spans = search_and_rerank_chunk_spans(
        args.query,
        num_results=args.k,
        search=hybrid_search,
        config=config(),
    )
    results = []
    for rank, span in enumerate(spans, start=1):
        meta = span.document.metadata_
        results.append(
            {
                "rank": rank,
                "source": span.document.filename,
                "authority": _first(meta.get("authority"), "unknown"),
                "status": _first(meta.get("status"), "unknown"),
                "priority": _first(meta.get("priority"), 0),
                "content": span.content,
            }
        )
    print(json.dumps(results, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
