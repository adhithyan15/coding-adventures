---
category: TypeScript / JavaScript
---

# Repo-root `.gitignore` has `compile.*` (Erlang/Elixir BEAM artefact pattern, line 89) that silently eats TS files named `compile.ts` / `compile.test.ts`

`git add` skips them without warning; `git commit` succeeds without the files; tests pass locally because the file exists on disk. Always `git ls-files` after a fresh-package commit to verify all source files are tracked. Workaround: rename to `orchestrator.ts` / `pipeline.ts` / `runner.ts` — avoid the `compile.*` collision entirely. Found in `forme-style-orchestrator`.
