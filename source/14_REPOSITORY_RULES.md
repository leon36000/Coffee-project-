# HermesClaw Repository Rules

**Authority:** repository safety rules

## Official target

`leon36000/Coffee-project-`

## Explicit non-target

`leon36000/GitSpace`

The existence of HermesClaw-related CI/bootstrap files in GitSpace from a previous mistaken session does not make GitSpace a HermesClaw repository.

## Required mutation procedure

Before any create/update/delete/branch/commit/push/PR action:

1. Resolve repository metadata from GitHub.
2. Confirm exact `owner/name` equals the intended target.
3. Inspect default/current branch and existing contents.
4. Inspect local `git status`/diff if working from a local checkout.
5. Avoid staging unrelated work.
6. Run verification on the tree that will actually be integrated.
7. Report exact resulting branch/commit/PR target.

## Never infer repository identity

Names such as “this repo”, “our repo”, “HermesClaw repo”, or a local folder name are insufficient for remote mutations unless the connected repository context is already unambiguous and verified in the current session.
