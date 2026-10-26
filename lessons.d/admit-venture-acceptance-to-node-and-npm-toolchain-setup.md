---
category: CI & GitHub Actions
---

# Admit Venture acceptance to Node and npm toolchain setup

A workflow-only change selected Venture acceptance independently of the
TypeScript package-language flag. The Windows React backend install failed with
`Cannot read properties of null (reading 'edgesOut')`; both Node and pinned npm
setup steps had been skipped. The runner's npm version was not captured, so the
log does not establish which resolver version crashed.

Admit the existing Venture flag to both toolchain setup steps, retaining the
workflow's Node version and npm pin. Exercise the Venture-only Boolean case,
the existing TypeScript-only case, neither, and both. Scope the test to actual
build-job step fields so comments or another job's setup cannot satisfy it.
Keep generated frontend install/test/build acceptance enabled and validate it
with the pinned resolver; a routing assertion alone is not runtime evidence.
