# HermesClaw RAG Query Guide

**Authority:** retrieval-use guidance

Use RAG to recall details; use source authority to decide truth.

## Good retrieval questions

- “What decisions constrain the HermesClaw backend language?”
- “What is the official GitHub repository and what repo is forbidden?”
- “What did the prior vertical proof attempt contain?”
- “What are the criteria for calling a capability ported?”
- “What is the next safe action before coding resumes?”
- “Which upstream commit is the frozen OpenClaw baseline?”

## Retrieval workflow for agents

1. Search with 5–10 results.
2. Prefer documents with higher manifest priority/authority.
3. If a retrieved statement is historical or volatile, verify current state before acting.
4. When two retrieved chunks conflict, do not blend them. Resolve by authority and timestamp.
5. Cite the source filename in internal work notes/handoff when a decision depends on retrieved context.

## Avoid

- treating vector similarity as truth ranking;
- retrieving old implementation plans and assuming unchecked boxes reflect current state;
- using RAG output to authorize external actions;
- storing secrets in searchable project memory.
