# HermesClaw Local RAGLite

This directory creates a **derived local retrieval index** over the canonical HermesClaw project source.

Pinned RAGLite: **1.1.1**. Current upstream RAGLite 1.1.1 supports DuckDB or PostgreSQL; this scaffold defaults to local DuckDB.

## Requirements

- Python 3.10–3.13
- network access for first dependency/model/database-extension setup
- an LLM + embedding model configured through LiteLLM identifiers

## Install

```bash
cd source/raglite
python -m venv .venv
source .venv/bin/activate        # Windows: .venv\\Scripts\\activate
pip install -e .
```

For RAGLite's llama.cpp local-model extra:

```bash
pip install -e '.[local]'
```

## Configure

The wrapper intentionally does not silently choose a cloud provider. Export explicit model identifiers:

```bash
export HERMESCLAW_RAG_LLM='gpt-4o-mini'
export HERMESCLAW_RAG_EMBEDDER='text-embedding-3-large'
export OPENAI_API_KEY='...'
```

Or use the local identifiers shown in `.env.example` after installing llama.cpp support.

`HERMESCLAW_RAG_DB_URL` defaults to a DuckDB file under `source/.raglite/`.

## Validate corpus

No RAGLite install is required:

```bash
python validate_source.py
```

## Rebuild index

For this small corpus, prefer a complete rebuild so removed/edited documents cannot remain as stale records:

```bash
python -m hc_rag.index --rebuild
```

## Search

```bash
python -m hc_rag.search 'What repository is canonical for HermesClaw?' --k 6
```

Output contains source name, authority metadata, and the retrieved span.

## Ask with RAG generation

```bash
python -m hc_rag.ask 'What must I verify before resuming implementation?'
```

## MCP

RAGLite itself includes an MCP server exposing a knowledge-base search tool. After indexing, you can configure/install it with the same DuckDB URL and model identifiers. See `MCP_SETUP.md`.

## Important

The DuckDB database is **not memory truth**. Delete it freely and rebuild it from `../SOURCE_MANIFEST.json` + Markdown files.
