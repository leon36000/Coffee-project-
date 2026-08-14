# RAGLite MCP Setup for HermesClaw Project Memory

RAGLite 1.1.1 includes an MCP server with a knowledge-base search tool.

After installing/indexing, use the same database/model configuration as the wrapper. Example with API models:

```bash
raglite \
  --db-url duckdb:///ABSOLUTE/PATH/source/.raglite/hermesclaw.duckdb \
  --llm gpt-4o-mini \
  --embedder text-embedding-3-large \
  mcp install
```

Set the required provider API key in the environment rather than committing it.

For a local llama.cpp setup, substitute your configured `llama-cpp-python/...` LLM and embedder identifiers.

The MCP tool is retrieval-only project context. An agent must still obey `00_PROJECT_CONSTITUTION.md` and manifest authority precedence.
