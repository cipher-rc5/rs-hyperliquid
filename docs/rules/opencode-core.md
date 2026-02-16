# OpenCode Core Rules

## Scope

These rules apply to all OpenCode sessions for this repository.

## Workflow

- Prefer `just` recipes for routine operations.
- Use `just check` before finalizing implementation work.
- Use `just ci` before opening or updating pull requests.
- Keep changes focused and avoid unrelated refactors.

## Safety

- Do not run destructive git commands unless explicitly requested.
- Do not edit secrets or commit generated credential files.
- Preserve existing user changes in a dirty working tree.

## Documentation

- Keep architecture, usage, and testing docs in `docs/` synchronized with behavior changes.
- Do not use emojis in repository documentation.
- Run `just docs-no-emoji` after docs edits.
