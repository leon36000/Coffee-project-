"""Explicit configuration for the disposable HermesClaw RAG index."""

from __future__ import annotations

import os
from pathlib import Path

from raglite import RAGLiteConfig, hybrid_search

SOURCE_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_DB = SOURCE_ROOT / ".raglite" / "hermesclaw.duckdb"


def _required(name: str) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        raise RuntimeError(
            f"{name} is required. See source/raglite/.env.example and README.md for remote/local examples."
        )
    return value


def config() -> RAGLiteConfig:
    """Build a RAGLite config with explicit model selection and local DuckDB by default."""
    db_url = os.environ.get(
        "HERMESCLAW_RAG_DB_URL", f"duckdb:///{DEFAULT_DB.resolve().as_posix()}"
    )
    return RAGLiteConfig(
        db_url=db_url,
        llm=_required("HERMESCLAW_RAG_LLM"),
        embedder=_required("HERMESCLAW_RAG_EMBEDDER"),
        reranker=None,
        search_method=hybrid_search,
        self_query=False,
        vector_search_query_adapter=False,
    )
