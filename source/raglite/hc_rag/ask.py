"""Answer a question with retrieved HermesClaw project context."""

from __future__ import annotations

import argparse

from raglite import add_context, rag, retrieve_context

from hc_rag.settings import config


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("question")
    args = parser.parse_args()

    cfg = config()
    context = retrieve_context(query=args.question, num_chunks=8, config=cfg)
    messages = [
        add_context(
            user_prompt=(
                args.question
                + "\n\nAuthority rule: prefer current canonical HermesClaw source over historical references; "
                "if context is insufficient or conflicting, say so rather than inventing a fact."
            ),
            context=context,
            config=cfg,
        )
    ]
    for update in rag(messages, config=cfg):
        print(update, end="")
    print()


if __name__ == "__main__":
    main()
