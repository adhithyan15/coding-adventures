# Changelog

## 0.1.0 — unreleased

- **The first parse-level list, and the CI gate.** Measured at test262
  `7ab7faf`: **69,017 of 102,955 results pass** (67%); 843 are skipped (module
  goal). `expected/parse.txt` lists every passing id, so the gate now fails on
  any regression. The largest failing areas show what the parser lacks: early
  errors and goals (`language/expressions` 8,908 and `language/statements`
  6,711 failing), then newer syntax (`built-ins/Temporal`, `RegExp`).
  `.github/workflows/v8-test262.yml` fetches test262 (tests and harness only,
  blob-filtered) at the pinned commit and runs the parse level on changes to
  the runner, the JavaScript lexer/parser/tokens or the ECMAScript grammars.

- **The test262 runner exists before the engine (V8C09).** First crate of the
  revised V8C series: the conformance gate for our JavaScript engine.
  - `header`: reads the YAML slice test262 headers use (`negative` phase and
    type, `flags`, `features`, `includes`); anything outside that slice is an
    error naming the line, never a guess.
  - `variants`: unflagged tests run twice (sloppy and `"use strict"`);
    `onlyStrict`, `noStrict`, `raw` and `module` narrow that. Each run has its
    own id (`path#strict`).
  - The **parse level**: a `negative.phase: parse` test passes when the parser
    rejects it; every other test passes when the parser accepts it and its
    harness includes. Module and resolution tests are skipped with a reason,
    never passed. The real parser runs on a large-stack thread, and a panic is
    a rejected parse, not a crash.
  - `expected`: the grow-only expected-pass list. A listed id that stops
    passing is a regression; `--update` only adds.
  - `v8-test262` binary: `--test262 DIR` / `TEST262_DIR`, path filters,
    `--json`, and a refusal to compare against a checkout that is not at the
    pinned `TEST262_REVISION` (read from `.git` without running git).
  - Pinned test262 `7ab7fafa0003f73fc85c1b95d88094d33f7eb8bd`. The first
    parse-level list is empty until the first run against that revision.
  - Hardened after security review: an `includes:` entry must be a bare file
    name (no path, `..` or absolute path, so nothing outside `harness/` or a
    device like `/dev/zero` is read); test and harness files must be regular
    files of at most 8 MiB; file names with control characters are skipped.
    A parser panic is a rejected parse; a stack overflow still aborts, which
    the large parser stack makes unlikely.
  - 19 tests: the runner's rules on hand-written fixtures with a stub parser,
    plus the real `javascript-parser` on the same fixtures. No test262 file is
    copied into the repository.
