# HermesClaw Project Memory + RAG Implementation Plan

> **For agentic workers:** the output of this plan is documentation/configuration only. It must not resume HermesClaw product implementation.

**Goal:** Produce a ready-to-upload `source/` corpus plus a local RAGLite 1.1.1 scaffold.

**Architecture:** Canonical Markdown + authority manifest + disposable DuckDB index. RAG retrieval is read-only with respect to project truth.

**Tech Stack:** Markdown, JSON, Python 3.10–3.13, RAGLite 1.1.1, DuckDB.

## Tasks

- [x] Capture repository identity and anti-drift rules.
- [x] Separate facts, decisions, state, architecture, workflow, roadmap, security, terminology, and handoff.
- [x] Preserve prior architecture/implementation artifacts as explicitly historical references.
- [x] Create authority-ranked `SOURCE_MANIFEST.json` with content hashes.
- [x] Create RAGLite package, environment examples, ingestion/search/ask CLIs, and MCP instructions.
- [x] Add a source validator that requires no third-party dependencies.
- [x] Validate Python syntax, JSON, manifest hashes, and archive layout.
- [x] Build a ZIP ready to upload/store.
