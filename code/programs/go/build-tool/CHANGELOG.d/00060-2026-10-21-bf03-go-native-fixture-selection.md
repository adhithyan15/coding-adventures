## 2026-10-21 — BF03 Go native fixture selection

- Seed go/compiler-ir from the exact compiler-ir-v1/cases.json external path
  before fixture early return for all/Go filters; preserve downstream closure,
  deleted/renamed old paths, source union and three-platform build plans.
- Fail closed on a missing native reader; ignore lookalike/schema/docs paths
  and unrelated languages. Add exact selector and atomic-plan regressions.


All notable changes to the Go build tool will be documented in this file.
