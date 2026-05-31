# Changelog

All notable changes to the `coding-adventures-closurec` binary will be documented in this file.

## [0.41.0] - 2026-05-30

### Added — CLOC11.78: end-to-end integration test for NDJSON CV sidecar

New integration test `tests/diff_cv_ndjson_streaming.rs` exercises the CLOC11.69 NDJSON format end-to-end:

```bash
closurec \
  --correlation_vector \
  --correlation_vector_format NDJSON \
  --js tests/diff/cv-ndjson-streaming/input/a.js \
  --js_output_file <tmpdir>/out.js
```

### Contract pinned

1. Sidecar lands at `<js_output_file>.cv.json` (the CLOC11.67 default-path policy).
2. Sidecar is **newline-delimited JSON** — every non-empty line parses standalone.
3. At least 2 lines (≥1 entry + the `_meta` footer).
4. Final line is the `{"_meta": {"pass_order": [...], "enabled": ...}}` footer so streaming consumers (`tail -f`, `jq`) can reliably extract `pass_order` after the producer finishes.
5. closurec exits 0.

### Why this exists as its own integration test

CLOC11.69 unit tests verify `format_cv_log_ndjson` in isolation. This test exercises the full path — CLI parse → wire → `run_compiler` → formatter → disk write → consumer-style readback — through the actual binary. Catches drift in any of those layers, especially path resolution and the `--js_output_file`-sibling sidecar convention.

### Changed

- Versions: `Cargo.toml` `0.40.0` → `0.41.0`, `cli.spec.json` `0.40.0` → `0.41.0`.

## [0.40.0] - 2026-05-30

### Added — CLOC11.77: end-to-end integration test for CV pure-analysis combo

New integration test `tests/diff_cv_pure_analysis.rs` exercises the combo built up across CLOC11.60 → 11.76:

```
--correlation_vector              (CLOC11.60)
--correlation_vector_summary      (CLOC11.73)
--correlation_vector_summary_only (CLOC11.76)
--correlation_vector_format NONE  (CLOC11.69)
```

Contract pinned:
1. With `--correlation_vector_summary_only`, no JS file lands on disk even though `--js` is supplied.
2. With `--correlation_vector_format NONE`, no CV sidecar lands either — pure in-memory analysis, no writes.
3. The CV summary line still makes it to stdout because `--correlation_vector_summary` is on and `summary_stderr` is off (default).
4. closurec exits 0 — pure-analysis is a normal successful invocation.

### Why this exists as a separate integration test

The combination touches CLI parsing, wire reading, four config fields, three skip-gates in `run_compiler`, and the summary serializer. A single end-to-end test through the actual binary catches integration drift that per-feature unit tests would miss — e.g. a future refactor that splits `SpecialModesConfig` and forgets to thread one of the four flags would fail here even if every isolated test still passed.

### Changed

- Versions: `Cargo.toml` `0.39.0` → `0.40.0`, `cli.spec.json` `0.39.0` → `0.40.0`.

## [0.39.0] - 2026-05-30

### Added — CLOC11.76: `--correlation_vector_summary_only` (pure analysis mode)

Boolean flag (default false). When on, the run **skips every output write**: no JS file (or stdout), no source map, no manifest. The CV log is still computed in memory, so `--correlation_vector_summary` can still print real counts.

Pairs naturally with `--correlation_vector_format NONE` to skip the CV sidecar too — a pure-analysis invocation that does no disk writes whatsoever. With both set, the only externally observable output is the summary line on stdout (or stderr under CLOC11.75).

Use case: `closurec --correlation_vector --correlation_vector_summary --correlation_vector_summary_only` answers "what would the CV trace look like" without rebuilding artifacts.

### Implementation

- `SpecialModesConfig` gains `correlation_vector_summary_only: bool`.
- Three call sites in `run_compiler` are gated on `!summary_only`: the JS output write, the source map write, and the manifest write. The matching `js_output_file` / `source_map_output` / `manifest_output` CV records are also skipped (they describe writes that didn't happen).
- The CV sidecar write block (Step 7) is unchanged — `--correlation_vector_format NONE` is the right way to skip it, summary_only doesn't reach into that policy.
- Default false → byte-identical to CLOC11.75.

### Changed

- `wire.rs`: `read_special_modes` pulls the new bool.
- Versions: `Cargo.toml` `0.38.0` → `0.39.0`, `cli.spec.json` `0.38.0` → `0.39.0`.

## [0.38.0] - 2026-05-30

### Added — CLOC11.75: `--correlation_vector_summary_stderr`

Boolean flag (default false). When on, the `--correlation_vector_summary` line is routed to `stderr_text` instead of `stdout_text`. Useful when stdout carries the actual JS payload (no `--js_output_file`) — without this flag, the summary line would corrupt the JS that downstream tooling pipes into.

### Changed

- `CompilerOutput` gains `stderr_text: String` (default empty). Existing callers ignoring stderr see no behavior change; tests can assert on routing without grepping file descriptors.
- `parse_and_run`'s contract is now: returns `(stdout_text + stderr_text, ExitCode)` for back-compat with existing callers. A new `parse_and_run_with_streams` returns `(stdout, stderr, ExitCode)` separately; `main()` calls the streaming variant and writes stderr via `eprint!`.
- `SpecialModesConfig` gains `correlation_vector_summary_stderr: bool`.
- Versions: `Cargo.toml` `0.37.0` → `0.38.0`, `cli.spec.json` `0.37.0` → `0.38.0`.

### Implementation note

Why split the CompilerOutput rather than threading an `io::Write`: keeping run_compiler pure (no I/O, returns a value) preserves the existing test ergonomics — tests inspect the returned struct. The cost is one extra `String` field that's empty on the common path.

## [0.37.0] - 2026-05-30

### Added — CLOC11.74: `--correlation_vector_summary_format` enum (TEXT | JSON | KV)

Machine-readable rendering for the CLOC11.73 summary line. Lets CI/build pipelines consume the summary without regex-matching the human-readable text.

- `TEXT` (default) — CLOC11.73 line: `cv sidecar: <path>: N entries, M contributions, T tombstones, pass_order=[a,b,c]`
- `JSON` — single-line JSON object:
  ```json
  {"cv_sidecar":{"path":"<path>","skipped":false,"entries":N,"contributions":M,"tombstones":T,"pass_order":["a","b","c"]}}
  ```
  Under format=NONE: `"path": null, "skipped": true`.
- `KV` — space-separated `key=value`:
  ```
  cv_sidecar.path="<path>" cv_sidecar.skipped=false cv_sidecar.entries=N cv_sidecar.contributions=M cv_sidecar.tombstones=T cv_sidecar.pass_order="a,b,c"
  ```
  Path and pass_order are quoted on the RHS via `serde_json::to_string` so shell tooling can split on whitespace safely.

Flag is only consulted when `--correlation_vector_summary` is also on. With summary off, the format selector is dead.

### Implementation

- `compute_cv_summary` and `summary_line` gain a `summary_format: CorrelationVectorSummaryFormat` parameter. The count walk is unchanged; only the terminal rendering branches.
- JSON and KV use `serde_json` for string escaping (paths can contain quotes, backslashes, control chars on weird filesystems — let serde handle it).
- New public enum `CorrelationVectorSummaryFormat` with `#[default] = Text`.

### Changed

- `SpecialModesConfig` gains `correlation_vector_summary_format: CorrelationVectorSummaryFormat`.
- `wire::read_special_modes` maps the string value to the enum; unknown / empty falls back to `Text`.
- Versions: `Cargo.toml` `0.36.0` → `0.37.0`, `cli.spec.json` `0.36.0` → `0.37.0`.

## [0.36.0] - 2026-05-30

### Added — CLOC11.73: `--correlation_vector_summary` stdout one-liner

Boolean flag. When on, prints a single summary line to stdout (`CompilerOutput.stdout_text`) after the CV sidecar write (or skipped under `--correlation_vector_format NONE`). Lets build pipelines see how many entries / contributions / tombstones the run produced without parsing the JSON itself.

Output format:

```
cv sidecar: <path>: N entries, M contributions, T tombstones, pass_order=[a,b,c]
```

When the format is `NONE`:

```
cv sidecar: skipped (format=NONE): N entries, M contributions, T tombstones, pass_order=[a,b,c]
```

### Counts are post-filter

`compute_cv_summary` calls the same `prune_entries_by_source` helper the formatters use (with the same `include_origin` and `invert` flags), so the printed counts describe **what's actually on disk** when a filter is in play.

### Composition

- Default off → no change to existing stdout output.
- Composes orthogonally with every other CV flag — `summary` reads what the formatters wrote, it doesn't second-guess them.
- Trails the JS / source map / manifest stdout (only fires when CV is on; the line ends in `\n`).

### Implementation

- New private `compute_cv_summary(cv_log, filter, include_origin, invert, wrote_path)` returns the rendered line. Parses `cv_log.to_json_string()` once, applies the filter, counts entries / contributions / tombstones, extracts `pass_order`.
- `summary_line` helper formats the rendered string; isolated from the count walk so format changes don't bleed into the counting path.
- `result.stdout_text` is now bound `mut` to allow the summary append.

### Changed

- `SpecialModesConfig` gains `correlation_vector_summary: bool`.
- `wire::read_special_modes` reads the bool.
- Versions: `Cargo.toml` `0.35.0` → `0.36.0`, `cli.spec.json` `0.35.0` → `0.36.0`.

## [0.35.0] - 2026-05-30

### Added — CLOC11.72: `--correlation_vector_filter_invert`

Boolean flag that flips the CLOC11.70 allowlist into a **blocklist**. With:

```bash
closurec --correlation_vector \
         --correlation_vector_filter lex \
         --correlation_vector_filter_invert
```

entries that DO match (i.e. carry a `lex` contribution and/or — under `--correlation_vector_filter_includes_origin` — a `lex` origin) are dropped; everything else is kept.

Use case: "everything except X" filters where enumerating the allowlist would be impractical (the lexer alone produces many `lexer_token` entries; flipping to `--correlation_vector_filter_invert` lets you exclude one stage instead of listing every other).

### Composition

Invert composes orthogonally with `include_origin`:
- `include_origin` selects WHICH sources count as a match (contribution.source only, or contribution.source ∪ origin.source).
- `invert` then decides whether matches are kept or dropped.

| include_origin | invert | Behavior                                                  |
|----------------|--------|-----------------------------------------------------------|
| false          | false  | CLOC11.70 strict allowlist on contribution.source         |
| true           | false  | CLOC11.71 broadened allowlist (also origin.source)        |
| false          | true   | CLOC11.72 blocklist on contribution.source                |
| true           | true   | CLOC11.72 blocklist on contribution.source ∪ origin.source |

### Implementation

- `prune_entries_by_source` signature: now `(root, allowlist, include_origin, invert)`. The match-computation logic is unchanged; only the keep-rule terminal expression is `if invert { !matches } else { matches }`.
- Both `format_cv_log_json` and `format_cv_log_ndjson` thread the flag through. Empty-allowlist short-circuit unchanged: an inverted empty filter is a no-op (no entry can match → none are blocked), so we keep the fast path.

### Changed

- `SpecialModesConfig` gains `correlation_vector_filter_invert: bool`.
- `wire::read_special_modes` reads the bool.
- Versions: `Cargo.toml` `0.34.0` → `0.35.0`, `cli.spec.json` `0.34.0` → `0.35.0`.

## [0.34.0] - 2026-05-30

### Added — CLOC11.71: `--correlation_vector_filter_includes_origin`

Adds an opt-in sub-flag to extend the CLOC11.70 filter so it also matches against each entry's `origin.source`, not just `contribution.source`.

Default `false` preserves CLOC11.70's strict semantics byte-for-byte: only entries with a contribution whose source is in the allowlist survive.

With `--correlation_vector_filter_includes_origin true` and `--correlation_vector_filter lex`, an entry is kept iff:

1. any element of `contributions` has `source` in the allowlist (the CLOC11.70 rule), OR
2. the entry's `origin.source` is in the allowlist.

This is how you get a `--correlation_vector_filter lex` invocation to also retain the per-token CV entries created with `Origin{source: "lexer_token", ...}` — their "lex" association lives in the Origin, not in a contribution.

### Why opt-in rather than the new default

Default-on would silently change the result of every existing `--correlation_vector_filter X` invocation. Default-off keeps CLOC11.70 unchanged; users who want the broader match flip the flag. Standard backward-compat policy for evolving CLI semantics.

### Changed

- `SpecialModesConfig` gains `correlation_vector_filter_includes_origin: bool`.
- `wire::read_special_modes` reads the bool flag.
- `prune_entries_by_source` signature: now `(root, allowlist, include_origin)`. Inline doc updated; the contribution-match branch still short-circuits before the Origin check so the fast path is unchanged.
- Both `format_cv_log_json` and `format_cv_log_ndjson` thread the flag through.
- Versions: `Cargo.toml` `0.33.0` → `0.34.0`, `cli.spec.json` `0.33.0` → `0.34.0`.

## [0.33.0] - 2026-05-30

### Added — CLOC11.70: `--correlation_vector_filter` allowlist flag

Adds a CSV allowlist of CV `contribution.source` names. When non-empty, the sidecar serializer prunes any CV entry whose `contributions` does not include at least one record whose `source` is in the allowlist.

Example: `--correlation_vector_filter lex,defines` writes only entries that the `lex` or `defines` stages touched.

### Semantics

- Strict match on `contribution.source`. The per-token CV entries created with `Origin{source: "lexer_token", ...}` but with zero contributions are dropped when the filter is `lex` — their "lex" association lives in the Origin, not in a contribution. The per-file CV root (which holds the `lex.tokens_emitted` contribution) is kept. Documented in the config-level rustdoc and pinned by tests.
- Empty allowlist = no pruning (default behavior). The fast path in `format_cv_log_json` short-circuits the round-trip when both `pretty` and `filter` are unset.
- Whitespace around CSV tokens is trimmed; empty tokens are ignored. `"lex, defines"` is the same as `"lex,defines"`.

### Implementation

- New shared helper `prune_entries_by_source(&mut serde_json::Value, &[String])` mutates the parsed CV log in-place. Uses a `HashSet` for O(1) source lookup.
- Both `format_cv_log_json` and `format_cv_log_ndjson` now take the filter slice and call the helper between parse and re-emit.

### Changed

- `SpecialModesConfig` gains `correlation_vector_filter: Vec<String>`.
- `wire::read_special_modes` splits the comma-separated string; trims whitespace; drops empty tokens.
- `format_cv_log_json` signature: now `(cv_log, pretty, filter) -> String`. Private to the crate.
- `format_cv_log_ndjson` signature: now `(cv_log, filter) -> String`. Private to the crate.
- Versions: `Cargo.toml` `0.32.0` → `0.33.0`, `cli.spec.json` `0.32.0` → `0.33.0`.

## [0.32.0] - 2026-05-30

### Added — CLOC11.69: `--correlation_vector_format` enum (JSON | NDJSON | NONE)

Adds a sidecar format selector to cover streaming consumers and benchmark modes.

- `JSON` (default) — single JSON document, same shape as CLOC11.60+. `--correlation_vector_pretty` still applies.
- `NDJSON` — newline-delimited JSON: one CV entry per line, ending with a `{"_meta": {"pass_order":[...], "enabled":...}}` footer line. Tooling can `tail -f` mid-build without waiting for a closing brace. The `pretty` flag is ignored under NDJSON (line-delimited JSON is inherently single-line per record).
- `NONE` — compute the CV log but **do not** write the sidecar. Lets benchmarks measure CV compute overhead in isolation from write/serialize overhead.

The flag is ignored when `--correlation_vector` is off.

### Implementation notes

- `format_cv_log_ndjson` round-trips through `serde_json::Value` (same approach as the pretty path) to walk the `entries` map without touching CV crate internals. Fallback chain: any parse/serialize hiccup yields the compact single-doc JSON instead of an empty file.
- The `None` arm is a single-statement no-op gated by the existing `if config.special_modes.correlation_vector` block, so default behavior is unchanged when the flag is absent.

### Changed

- `SpecialModesConfig` gains `correlation_vector_format: CorrelationVectorFormat` (new enum).
- `wire::read_special_modes` reads the enum from the parse result; unknown / empty values fall back to `Json`.
- Versions: `Cargo.toml` `0.31.0` → `0.32.0`, `cli.spec.json` `0.31.0` → `0.32.0`.

## [0.31.0] - 2026-05-30

### Added — CLOC11.68: `--correlation_vector_pretty` flag

Adds a toggle between compact and pretty-printed CV sidecar JSON. Default is compact (single-line, what CI / build pipelines want); `--correlation_vector_pretty` switches to multi-line, 2-space-indented output for human inspection.

Resolution:
- `--correlation_vector_pretty` (default `false`) → compact JSON via `CVLog::to_json_string`.
- `--correlation_vector_pretty true` → round-trip via `serde_json::Value` and `to_string_pretty` for the multi-line form.

The flag is only consulted when `--correlation_vector` is also enabled. With CV off, the formatter never runs.

### Why round-trip rather than a new upstream method

`CVLog::to_json_string` is the only path that knows the `LogSnapshot` shape (the fields aren't `pub`). Parsing back to a `serde_json::Value` and re-emitting via `to_string_pretty` is wasteful but correct, and only happens on the opt-in slow path. The performance hit is irrelevant — humans-eyes mode is already off the critical path of a build.

### Changed

- `SpecialModesConfig` gains `correlation_vector_pretty: bool`.
- `wire::read_special_modes` now reads `correlation_vector_pretty` from the parse result.
- `format_cv_log_json` signature changed from `(&CVLog) -> String` to `(&CVLog, pretty: bool) -> String`. Private to the crate; no external API impact.
- Versions: `Cargo.toml` `0.30.0` → `0.31.0`, `cli.spec.json` `0.30.0` → `0.31.0`.

## [0.30.0] - 2026-05-30

### Added — CLOC11.67: `--correlation_vector_output <path>` flag

Adds an explicit path override for the correlation-vector sidecar JSON. Lets CI pipelines route the CV trace to an artifact directory (or `/dev/null` for benchmarks) without relying on the sidecar-of-output convention.

Resolution order (highest precedence first):

1. `--correlation_vector_output <path>` → that path, verbatim, no decoration.
2. Else if `--js_output_file` is set → `<output>.cv.json` beside it.
3. Else (stdout output) → `closurec-cv.json` in the working directory.

The flag is only consulted when `--correlation_vector` is also enabled — the trace itself is still opt-in. With CV off, the path flag is ignored.

### Changed

- `SpecialModesConfig` gains `correlation_vector_output: Option<PathBuf>`.
- `wire::read_special_modes` now reads `correlation_vector_output` from the parse result; empty string maps to `None`.
- Versions: `Cargo.toml` `0.29.0` → `0.30.0`, `cli.spec.json` `0.29.0` → `0.30.0`.

## [0.29.0] - 2026-05-30

### Added — CLOC11.66: WHITESPACE_ONLY token tombstones

When `--correlation_vector` is on and `--compilation_level WHITESPACE_ONLY` is set, every token CV that the minifier drops (trivia + EOF) now gets a `DeletionRecord` (tombstone) via `CVLog::delete`. The CV trace shows precisely which input bytes the WHITESPACE_ONLY pass killed.

Tombstone shape (one per dropped token):

```
source: "compilation_level"
reason: "whitespace_only_dropped"
meta: {
  kind:                  "trivia" | "eof",
  token_index:           <0-based position in lexer stream>,
  token_lexeme_byte_len: <token.value.len()>,
}
```

Implementation reuses `whitespace_only::is_trivia` / `is_eof` (now `pub(crate)`) — the same predicate the minifier itself uses — so the tombstone set is guaranteed identical to the dropped set without a second lex pass.

Other compilation levels (SIMPLE, ADVANCED, BUNDLE, TRANSPILE_ONLY) are currently identity on the string and don't drop tokens, so no tombstones land for those. As those levels grow real bodies in later CLOC11.* slices, each will need its own tombstone block.

### Changed

- `whitespace_only::is_trivia` and `whitespace_only::is_eof` promoted from private `fn` to `pub(crate) fn` so the per-token CV path can call them directly without duplicating the predicate.
- Versions: `Cargo.toml` `0.28.0` → `0.29.0`, `cli.spec.json` `0.28.0` → `0.29.0`.

## [0.28.0] - 2026-05-30

### Added — CLOC11.65: per-token `defines.applied` contributions

Uses the per-token CV substrate from CLOC11.64. When `--correlation_vector` is on and `--define K[=V]` flags are present, every `Name` token in the input whose lexeme matches a define key gets a `defines.applied` contribution recorded **on its token CV**, not on the per-file root.

Per-token `defines.applied` contribution shape:

```
source: "defines"
tag:    "applied"
meta: {
  define_name:        <token.value>,
  define_value:       <Bool | Number | String | Null>,
  define_value_kind:  "bool" | "number" | "string" | "null",
  token_index:        <0-based position in the lexer stream>,
}
```

The per-file `defines.applied` summary (defines_count, byte deltas) still fires from `transform_source_with_cv` — the per-token records are *in addition*, so visualization tools get both "the stage ran" (file-level) and "this specific token was hit" (token-level).

Implementation: the token loop now keeps a `Vec<String>` of derived token CV IDs in lock-step with the token vector, so post-loop lookups are O(1). The defines check skips non-Name tokens — strings, numbers, regex literals — matching the existing string-level `apply_defines` behaviour.

Caveats (unchanged from the string-level pass):
- Defines inside string literals are not substituted (correct — the Name filter excludes string tokens).
- Object shorthand (`{ FOO }`) would change semantics if substituted; same caveat as `apply_defines`.

### Changed

- Versions: `Cargo.toml` `0.27.0` → `0.28.0`, `cli.spec.json` `0.27.0` → `0.28.0`.

## [0.27.0] - 2026-05-29

### Added — CLOC11.64: per-token CV entries (children of per-file CV)

Continues the "every feature CV-traceable when enabled" series. When `--correlation_vector` is on, after reading each input file we now tokenize with `coding-adventures-javascript-lexer::tokenize_javascript_typed` and derive a **child CV entry per token** under the per-file CV root.

This is the substrate for the next slices (CLOC11.65+) to migrate token-level contributions (`defines.applied`, `whitespace_only` drops, rename mappings) off the per-file summary entry and onto the precise token CV they touched — so the trace tells you *which token* a transform mutated, not just *which file*.

Per-token CV entry shape:

| Field         | Value                                            |
|---------------|--------------------------------------------------|
| parent_ids    | `[per_file_cv_id]` (via `CVLog::derive`)         |
| `source`      | `"lexer_token"`                                  |
| `location`    | `"<path>:<line>:<column>"` (1-based, lexer-native) |
| `meta.kind`   | lowercased `TokenType` debug name                |
| `meta.lexeme_byte_len` | `value.len()` (post escape resolution)  |
| `meta.token_index` | 0-based position in the token stream        |

Per-file CV gains one summary contribution after the token loop:

```
source: "lex", tag: "tokens_emitted", meta: {token_count: N}
```

Error policy: a lex failure does **not** abort the build. The string-only pipeline still runs (WHITESPACE_ONLY can copy verbatim, defines can no-op). We record `lex.failed` with the lexer error message on the per-file CV and skip per-token creation.

Cost: only paid when `--correlation_vector` is on. Default-off path is byte-identical to 0.26.0.

### Changed

- Versions: `Cargo.toml` `0.26.0` → `0.27.0`, `cli.spec.json` `0.26.0` → `0.27.0`.

## [0.26.0] - 2026-05-27

### Added — CLOC11.63: CV records for output writes (JS, source map, manifest)

Extends CLOC11.62 to record the three output-file writes as derived CV entries. Every byte that hits disk now has a CV ID, and the trace forms a proper DAG from per-file sources through combined-output to disk artifacts.

Three new derived CV entities:

| Entity                 | Created via   | Parent(s)            | Records                              |
|------------------------|---------------|----------------------|--------------------------------------|
| `js_output_file`       | `derive()`    | `combined_cv_id`     | `write_output_file.wrote` + byte_len |
| `source_map_output`    | `derive()`    | `combined_cv_id`     | `write_output_file.wrote` + byte_len |
| `manifest_output`      | `merge()`     | `per_file_cv_ids[]`  | `write_output_file.wrote` + byte_len |

**Why manifest uses `merge()` with per-file parents:** the manifest enumerates input files, not the merged output. Conceptually it's an index of the per-file CVs, not a derivative of the merged JS. A consumer following provenance from a manifest entry walks straight back to the per-file CV roots.

**Why JS / source_map use `derive()` with `combined_cv_id`:** they derive their bytes from the combined post-transform substrate.

Gates: each record only contributes when the corresponding flag is set (`--js_output_file`, `--create_source_map`, `--output_manifest`).

### Coverage milestone

After CLOC11.63, the CV trace covers every step:

```
input → per-file CV → combined CV → js_output_file CV → disk
                                  → source_map_output CV → disk
                                  → manifest_output CV (merge of per-file) → disk
```

The user's policy ("every feature CV-traceable when enabled") is structurally complete for the pipeline that exists today. CLOC11.64–66 add granularity (per-token, tombstones) and convenience (`--correlation_vector_output`), not coverage.

### Implementation

- Captured `encoded_byte_len` before the JS-write match block to avoid borrow-of-moved when the None arm consumes `encoded`.
- Output writes now followed by `cv_log.derive(...)` or `cv_log.merge(...)` when CV is on.
- 4 new unit tests in `run::tests`.

## [0.25.0] - 2026-05-27

### Added — CLOC11.62: CV records for post-combine stages

Extends CLOC11.61's per-stage instrumentation to the four post-concatenation pipeline stages: `emit_use_strict`, `output_wrapper`, `isolation_mode` (IIFE), and `charset`. After the per-file loop, the CV log derives a new "combined" entry whose parents are every per-file CV ID — so a downstream output byte's provenance walks `combined → all source files` automatically.

The combined entry is the substrate every post-concat contribution lands on:

| Stage             | `source`           | `tag`           | `meta`                                                        |
|-------------------|--------------------|-----------------|---------------------------------------------------------------|
| emit_use_strict   | `emit_use_strict`  | `prepended`     | `{input_byte_len, output_byte_len}` (only when flag set)      |
| output_wrapper    | `output_wrapper`   | `substituted`   | `{input_byte_len, output_byte_len}` (only when wrapper changed bytes) |
| isolation_mode    | `isolation_mode`   | `iife_wrapped`  | `{input_byte_len, output_byte_len}` (only when IIFE set)      |
| charset           | `charset`          | `normalized`    | `{mode: "US_ASCII"\|"UTF-8", input_byte_len, output_byte_len}` (always) |

Contribution-or-not policy: the `charset` stage always contributes (it always runs); the other three skip the contribution when they're pass-throughs (no flag set / no bytes changed). This keeps the trace focused on actual byte movement while still recording the structural step.

### New CV entity: `concatenated_combined_source`

After the per-file loop, when CV is on, `run_compiler` calls `CVLog::merge(per_file_ids, Some(combined_origin))` to create a new entry whose `parent_ids` are every per-file root. Meta carries `file_count` and `byte_len`. Origin: `source = "concatenated_combined_source"`, no location (it's not a file on disk). All four post-combine contributions attach here.

### Implementation

- **`per_file_cv_ids: Vec<String>`** accumulated through the per-file loop.
- **`combined_cv_id`** computed after the loop via `cv_log.merge(...)`.
- **Each post-combine stage** wrapped with `if let Some(id) = &combined_cv_id { ... cv_log.contribute(id, ...) }`.
- **`isolation_mode = None` branch** had to switch from move-of-`wrapped` to `.clone()` so we can record the input byte length in the CV branch above the move.
- **6 new unit tests** in `run::tests` (combined entry exists with parents, emit_use_strict on, emit_use_strict off → no contribution, output_wrapper changing bytes, IIFE on, charset always with mode).
- **Existing CLOC11.61 `pass_order` test** updated to assert prefix `[compilation_level, defines, ...]` rather than exact `[compilation_level, defines]`, since CLOC11.62 + later slices grow the pass_order.

### Pipeline matrix (unchanged structurally)

Same 15 steps; CLOC11.62 adds per-stage CV records inside steps 8–11 (and a new derived "combined" CV entity between steps 6 and 7).

### Still queued

- CLOC11.63: source-map / manifest writes recorded as derived CV entries.
- CLOC11.64–66: per-token granularity, tombstones for removals, custom `--correlation_vector_output` path flag.

## [0.24.0] - 2026-05-27

### Changed — CLOC11.61: per-stage `--correlation_vector` contributions

Builds on CLOC11.60's plumbing. Replaces the single `transform_source.applied` summary contribution with one record per pipeline stage so the CV trace shows which pass touched the bytes and how much they grew/shrank.

Per-file CV entry now gains:

| Stage              | `source`            | `tag`             | `meta`                                                  |
|--------------------|---------------------|-------------------|---------------------------------------------------------|
| WhitespaceOnly     | `compilation_level` | `whitespace_only` | `{input_byte_len, output_byte_len}`                     |
| Simple             | `compilation_level` | `identity`        | `{level: "SIMPLE"}`                                     |
| Advanced           | `compilation_level` | `identity`        | `{level: "ADVANCED"}`                                   |
| Bundle             | `compilation_level` | `identity`        | `{level: "BUNDLE"}`                                     |
| TranspileOnly      | `compilation_level` | `identity`        | `{level: "TRANSPILE_ONLY"}`                             |
| Defines            | `defines`           | `applied`         | `{input_byte_len, output_byte_len, defines_count}`      |

The `defines.applied` contribution lands even when `--define` is empty (`defines_count: 0`) — the stage *ran*, it just had nothing to substitute. Keeps the trace symmetric across files; visualization tools don't have to special-case zero-defines runs.

### Implementation

- **New `transform_source_with_cv(source, config, cv) -> Result<String>`** with `cv: Option<(&mut CVLog, &str id)>`. When `cv` is `None`, byte-identical behavior to `transform_source`.
- **`transform_source` is now a thin facade** delegating to `transform_source_with_cv(..., None)`.
- **`run_compiler`'s per-file loop** calls `transform_source_with_cv` when CV is on, passing the per-file `cv_id`. The CLOC11.60 post-call summary contribution is removed (superseded by the per-stage records).
- **5 new unit tests** in `run::tests`:
  - WHITESPACE_ONLY → `compilation_level.whitespace_only` contribution lands
  - SIMPLE default → `compilation_level.identity` with `level: "SIMPLE"` lands
  - `--define` entries → `defines.applied` with `defines_count`
  - Both stages present + `pass_order: [compilation_level, defines]`
  - `transform_source` facade ≡ `transform_source_with_cv(_, _, None)`
- **CLOC11.60 multi-file test updated** to count 2 × `compilation_level` + 2 × `defines` contributions instead of 2 × the old `transform_source` summary.

### Pipeline matrix (unchanged structurally)

Same 15 steps as 0.23.0; the change is in step 5's instrumentation, not in pipeline order.

### Still queued

- CLOC11.62: CV records for wrapper / IIFE / charset stages.
- CLOC11.63: source-map / manifest writes recorded as derived CV entries.
- CLOC11.64–66: per-token granularity, tombstones for removals, custom `--correlation_vector_output` path.

## [0.23.0] - 2026-05-27

### Added — CLOC11.60: opt-in `--correlation_vector` plumbing through pipeline

**Architectural milestone.** First slice of the correlation-vector traceability work specified in `feedback_closurec_correlation_vectors.md`. When `--correlation_vector` is set, the pipeline threads a [`coding_adventures_correlation_vector::CVLog`] through every input file and records per-file contributions for the transform stage. When the flag is unset (default), the `CVLog` is constructed in disabled mode — every `create`/`contribute` call is a no-op, so the existing zero-overhead pipeline behavior is preserved.

The CV trace is written as a JSON sidecar file at the end of the run. Path policy:

- When `--js_output_file` is set, the sidecar lives next to it as `<output>.cv.json`. Build pipelines consuming the compiled JS automatically pick up the trace without a separate flag.
- When `--js_output_file` is absent (stdout output), the sidecar lands at `closurec-cv.json` in the current working directory.

### What this slice covers (intentionally narrow)

- **Per-file root CV entry**: assigns a CV ID at file ingestion with `Origin::source = "input_file"` and `location = <path>`.
- **One summary contribution per file**: tags the entry with `source = "transform_source"`, `tag = "applied"`, and includes input + output byte lengths in `meta`. This is the placeholder for the deeper per-stage instrumentation queued in CLOC11.61..11.66.
- **JSON sidecar emission** via `CVLog::to_json_string()`.

### What's still queued

- CLOC11.61: split the per-file summary contribution into one-per-stage (`whitespace_only`, `defines`).
- CLOC11.62: wrapper / IIFE / charset stages.
- CLOC11.63: source-map / manifest writes recorded as derived CV entries.
- CLOC11.64–66: per-token granularity, tombstones for removals, custom `--correlation_vector_output` path flag.

### Implementation

- **`SpecialModesConfig.correlation_vector: bool`** field + cli.spec.json entry + wire.rs parsing.
- **`run_compiler` instantiates `CVLog::new(config.special_modes.correlation_vector)`** once before the per-input loop. The boolean toggle threads down into every CV call; disabled-mode short-circuits at the crate level.
- **`format_cv_log_json(&CVLog) -> String`** wraps `CVLog::to_json_string()` with a `{}` fallback so a serialization error doesn't break the otherwise-successful run.
- **Step 7 (NEW) in `run_compiler`** — writes the sidecar after the manifest write so `wrote_files` ends up in pipeline order: JS, source map, manifest, CV sidecar.
- **4 new unit tests** in `run::tests` (default → no sidecar, opt-in → sidecar next to output, opt-in stdout → default sidecar in CWD, multi-file → entry-per-file).

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Resolve `--js` globs
2. Resolve `--externs` globs
3. `--print_tree` short-circuit
4. `--print_tree_json` short-circuit
5. Per-input `transform_source` (now records one CV contribution per file when `--correlation_vector` is set)
6. Concatenate transformed inputs
7. `--checks_only` short-circuit
8. `--emit_use_strict` prepend
9. `--output_wrapper` substitution
10. `--isolation_mode IIFE` wrap
11. `--charset` US_ASCII escape
12. Write JS
13. Write source map
14. Write input manifest
15. **Write CV sidecar (CLOC11.60, NEW) if `--correlation_vector` was set**

## [0.22.0] - 2026-05-26

### Added — CLOC11.04: `--define` numeric edge case test coverage

Test-coverage slice pinning behavior of `--define VALUE` for the full set of numeric literals CC's `Double.parseDouble` accepts. Rust's `f64::parse` covers all of these already — these tests make the contract explicit so a future refactor (e.g. switching to a hand-rolled number parser, or tightening to integer-only) can't quietly regress CC compat.

Forms covered:

| Form          | Accepted by closurec | Example       |
|---------------|----------------------|---------------|
| Integer       | ✓                    | `42`          |
| Negative int  | ✓ (NEW PIN)          | `-42`         |
| Float         | ✓                    | `1.5`         |
| Negative float| ✓ (NEW PIN)          | `-1.5`        |
| Fractional-only | ✓ (NEW PIN)        | `.5`          |
| Scientific    | ✓ (NEW PIN)          | `1e3`         |
| Negative exp  | ✓ (NEW PIN)          | `1e-6`        |
| Leading `+`   | ✓ (NEW PIN)          | `+1`          |
| Zero          | ✓ (NEW PIN)          | `0`           |
| Negative zero | ✓ (NEW PIN)          | `-0`          |
| `NaN`         | ✗ (NEW PIN)          | rejected      |
| `Infinity`    | ✗ (NEW PIN)          | rejected      |
| Hex `0xFF`    | ✗ (NEW PIN)          | rejected      |

NaN/Infinity rejection is deliberate — they parse as `f64` in Rust but aren't valid JS *literals* (you'd write `0/0` or `1/0` to get them at runtime). Hex literals would be valid JS but CC's `Double.parseDouble` rejects them; we match.

- **10 new unit tests** in `wire::tests`, no behavior change.

### Pipeline matrix (unchanged)

Same 14-step pipeline as 0.21.0. This is a test-only release that pins the contract on existing config-build behavior.

## [0.21.0] - 2026-05-26

### Added — CLOC11.34: `--output_manifest` writes input file list

Behavioral compat slice with CC's `--output_manifest=path` flag. Previously closurec parsed the flag and stored the path but never wrote any file — build systems (Bazel `rules_closure`, ninja-driven builds) that read the manifest to verify input set saw nothing.

Now writes a newline-separated list of every input the compilation consumed (post-glob expansion), one path per line, with a trailing newline so `wc -l` and concatenation behave.

- **Pipeline placement**: Step 6 in `run_compiler`, after JS write + source-map write. So `wrote_files` in `CompilerOutput` lists outputs in pipeline order: JS, source map (if `--create_source_map`), manifest (if `--output_manifest`).
- **Empty inputs case** (banner mode): writes an empty manifest file (0 bytes) — still useful as a "compilation ran" marker, matches CC.
- **Paths in the manifest are the resolved form** (after glob expansion), not the raw user patterns. This lets the user see exactly which files the compilation consumed.
- **New `format_manifest(&[PathBuf]) -> String`** private helper. Pure function: no I/O.
- **5 new unit tests** in `run::tests` (empty-inputs format, multi-line format with newline count, end-to-end write with resolved path verification, no-write when flag unset, trifecta with JS + source map + manifest in pipeline order).

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Resolve `--js` globs
2. Resolve `--externs` globs
3. `--print_tree` short-circuit
4. `--print_tree_json` short-circuit
5. Per-input `transform_source`
6. Concatenate transformed inputs
7. `--checks_only` short-circuit
8. `--emit_use_strict` prepend
9. `--output_wrapper` substitution
10. `--isolation_mode IIFE` wrap
11. `--charset` US_ASCII escape
12. Write JS to `--js_output_file` or stdout
13. Write source map to `--create_source_map` path if set
14. **Write input list to `--output_manifest` path if set (CLOC11.34, NEW)**

## [0.20.0] - 2026-05-26

### Added — CLOC11.42: `--create_source_map` writes minimal v3 source map

Behavioral compat slice with CC's `--create_source_map=path` flag. Previously closurec parsed the flag and stored the path but never wrote any file — build scripts expecting a source map at the path saw nothing. Now writes a minimal valid v3 source map JSON at the path.

Wire format:

```json
{
  "version": 3,
  "file": "<--js_output_file basename or empty>",
  "lineCount": 0,
  "sourceRoot": "",
  "sources": [],
  "sourcesContent": [],
  "names": [],
  "mappings": ""
}
```

The mappings are intentionally empty — real position tracking lands with the parser-bridge in CLOC11.07+. The goal of this slice is that build pipelines (Bazel rules, webpack `source-map-loader` shims, etc.) expecting a file at the path see one with the right shape. Debuggers that try to use the map for position lookup get the correct response of "no information available" rather than a broken document.

- **New `source_map` module** with `format_minimal_v3(Option<&Path>) -> String`. Pure function: no I/O, fully deterministic.
- **Pipeline placement**: Step 5 in `run_compiler`, after the JS output write. Source map write runs *after* the JS write so callers get a consistent on-disk pair (or no source map at all if the flag is unset).
- **Source-map writing works even when `--js_output_file` is absent** (compiled JS goes to stdout). In that case the map's `file` field is empty.
- **`file` field is the basename** of the compiled-output path, not the full path — keeps the map portable across CDN paths.
- **9 new unit tests** in `source_map::tests` (empty-path → empty file key, basename extraction, version-3 marker, eight required keys present, empty arrays well-formed, empty mappings string, trailing newline, JSON escaping of weird file names, byte-stable output).
- **4 new unit tests** in `run::tests` (writes file when path set, no-write when path empty, stdout+map combination, basename-only `file` field).

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Resolve `--js` globs
2. Resolve `--externs` globs
3. `--print_tree` short-circuit
4. `--print_tree_json` short-circuit
5. Per-input `transform_source`
6. Concatenate transformed inputs
7. `--checks_only` short-circuit
8. `--emit_use_strict` prepend
9. `--output_wrapper` substitution
10. `--isolation_mode IIFE` wrap
11. `--charset` US_ASCII escape
12. Write JS to `--js_output_file` or stdout
13. **Write source map to `--create_source_map` path if set (CLOC11.42, NEW)**

## [0.19.0] - 2026-05-26

### Changed — CLOC11.16: `--charset` US_ASCII output escaping (BEHAVIOR DEFAULT CHANGE)

Behavioral compat slice with CC's documented `--charset` default. Previously closurec accepted `--charset` and stored it but never escaped non-ASCII characters in the output — every non-ASCII codepoint passed through verbatim regardless of flag value. That diverged from CC's documented default of "UTF-8 in, US_ASCII out".

Now matches CC:

| `--charset` value | Output behavior                              |
|-------------------|----------------------------------------------|
| (unset)           | **US_ASCII — escape non-ASCII as `\uXXXX`** (matches CC default) |
| `US_ASCII`        | same as unset                                |
| `US-ASCII`        | accepted alias                               |
| `UTF-8` / `UTF8`  | pass-through (raw UTF-8 bytes)               |
| anything else     | pass-through (CC ignores unknown values)     |

**This is a default-behavior change**: existing users who relied on raw-UTF-8 output and didn't pass `--charset` will now see `\uXXXX` escapes. To restore prior behavior, pass `--charset UTF-8` explicitly. CC users get this default already, so closurec invocations that worked against CC will continue to work against closurec.

Escape format: BMP codepoints (`U+0000..U+FFFF`) emit `\uXXXX`. Astral codepoints (`U+10000..U+10FFFF`) emit a UTF-16 surrogate pair (`\uXXXX\uXXXX`) — not the ES2015 `\u{XXXXX}` form — for maximum compatibility with legacy minifiers / ES5-only environments.

- **New `charset` module** with `OutputCharset::from_raw(&str)` + `apply_charset(&str, OutputCharset) -> String`. Pure-function, no I/O, fully deterministic.
- **Pipeline placement**: Step 3.75 in `run_compiler`, between IIFE wrap and write. Runs *after* the output wrapper so any non-ASCII the user injected via `--output_wrapper` (e.g. a `©` banner) gets escaped too.
- **12 new unit tests** in `charset::tests` (default → US_ASCII, value parsing for all aliases + case-insensitive, unknown → UTF-8 fallback, UTF-8 pass-through, US_ASCII pass-through for pure-ASCII text, BMP escape, CJK escape, surrogate pair, lowercase hex, byte-identical ASCII).
- **Diff fixtures** `tests/diff/charset-us-ascii/` (default) and `tests/diff/charset-utf8/` (opt-out).
- **New integration test** `tests/diff_charset.rs` pinning both ends of the toggle, including `is_ascii()` invariant under default.
- **`tests/diff/js-glob/expected.stdout` regenerated**: em-dashes in test input comments now appear as `—`. This is the new default; the test continues to exercise glob expansion logic.

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Resolve `--js` globs
2. Resolve `--externs` globs
3. `--print_tree` short-circuit
4. `--print_tree_json` short-circuit
5. Per-input `transform_source`
6. Concatenate transformed inputs
7. `--checks_only` short-circuit
8. `--emit_use_strict` prepend
9. `--output_wrapper` substitution (validated)
10. `--isolation_mode IIFE` wrap
11. **`--charset` US_ASCII escape (CLOC11.16, NEW)**
12. Write to `--js_output_file` or stdout

## [0.18.0] - 2026-05-26

### Changed — CLOC11.55: `--version` emits CC-style banner

Drop-in compat surface fix. Previously `--version` printed just `0.18.0\n` — a bare semver with no marker that tools could grep for to identify this binary as a Closure Compiler drop-in. Now matches the shape of CC's `closure-compiler.jar --version`:

```
Closure Compiler (closurec — drop-in replacement, https://github.com/adhithyan15/coding-adventures)
Version: 0.18.0
```

Why this shape:
- First line starts with `Closure Compiler ` — toolchains that grep CC's stdout for that marker (e.g. Bazel rules that pin a compiler identity) keep working.
- Second line is `Version: <semver>` — standard hook for version-extracting scripts.
- Project URL points at this clone rather than upstream so users know what they're running.
- No `Built on:` line (CC has one) — we don't embed build timestamps. The two `grep`-worthy lines are what tools actually depend on.

- **Updated `ParserOutput::Version` arm** in `main::parse_and_run`. cli-builder still surfaces `--version` as `ParserOutput::Version(v)`; we just format `v.version` differently.
- **4 new unit tests** in `main::tests` (starts-with-marker, Version-colon line, embedded semver still present, trailing-newline cleanliness).
- **Diff fixture** `tests/diff/version-banner/` with `flags.txt` driving the integration test.
- **New integration test** `tests/diff_version_banner.rs` pinning the structural invariants. We don't pin byte-for-byte because the embedded semver changes every release.

### Pipeline matrix (unchanged)

Same 11-step pipeline; change is in `main.rs`'s top-level dispatch, not `run_compiler`.

## [0.17.0] - 2026-05-26

### Changed — CLOC11.41: `--source_map_location_mapping` malformed values now error

Sibling fix to CLOC11.40. The same silent-drop `filter_map` bug existed in `read_source_map`'s `source_map_location_mapping` parser. Pre-CLOC11.41, a typo'd `--source_map_location_mapping src/` (no `|`) silently vanished, leaving the user wondering why their map URLs didn't rewrite.

Now the parser errors out with a typed `ConfigError::InvalidSourceMapLocationMapping { raw }`:

```
--source_map_location_mapping <raw>: missing required `|` separator (expected `filesystem-path|web-server-path`)
```

Argv-order processing — first bad entry surfaces, matching the CLOC11.40 policy.

Edge cases preserved (match CC):
- `|web/` and `fs/|` remain well-formed (only pipe presence is checked).

- **New `ConfigError::InvalidSourceMapLocationMapping { raw }` variant** + Display arm.
- **`filter_map` replaced** with an explicit `for` loop that propagates the typed error.
- **4 new unit tests** in `wire::tests` (missing-pipe errors, error message format, multi-entry first-bad-wins, empty halves still well-formed).
- **Diff fixture** `tests/diff/source-map-location-mapping-bad/`.
- **New integration test** `tests/diff_source_map_location_mapping_bad.rs`.

### Pipeline matrix (unchanged)

Same 11-step pipeline; change is config-build validation only.

## [0.16.0] - 2026-05-25

### Changed — CLOC11.40: `--source_map_input` malformed values now error

Behavioral compat slice with CC's source-map-input handling. Previously closurec parsed `--source_map_input` entries via `filter_map(|s| s.split_once('|'))`, which silently dropped malformed values that lacked the required `|` separator. Effect on users: typo'd separator → entry quietly vanishes → user wonders why their source map chain didn't apply.

Now the parser errors out with a typed `ConfigError::InvalidSourceMapInput { raw }`. The error message names both the flag and the offending value:

```
--source_map_input <raw>: missing required `|` separator (expected `input-file-path|input-source-map`)
```

Processing order: argv-order, first bad entry surfaces. So a user fixes typos one at a time rather than playing whack-a-mole after each retry.

Edge cases preserved:
- `|map.map` and `input.js|` are still well-formed (only the *presence* of the pipe is checked; empty halves are accepted, matching CC). When the source-map chain step lands later, the FS resolver will catch missing files separately.

- **New `ConfigError::InvalidSourceMapInput { raw }` variant** + Display arm.
- **`filter_map` replaced** with an explicit `for` loop that propagates the typed error.
- **5 new unit tests** in `wire::tests` (happy path two paths, missing pipe errors, error message format, multi-entry first-bad-wins, empty-halves still well-formed).
- **Diff fixture** `tests/diff/source-map-input-bad/` exercising the error path.
- **New integration test** `tests/diff_source_map_input_bad.rs` pinning that both the flag and the offending value appear in the error.

### Pipeline matrix (unchanged)

Same 11-step pipeline as 0.15.0; the change is config-build validation (`wire.rs::read_source_map`), not pipeline behavior.

## [0.15.0] - 2026-05-25

### Changed — CLOC11.05: `--externs` is now glob-resolved + validates missing files

Behavioral compat slice with CC's externs file handling. Previously closurec accepted `--externs <path>` as a literal `PathBuf` and never touched the filesystem to verify the path existed — a typo would silently drop the externs definitions and only manifest later (or never, at the current pipeline stage where externs aren't yet consumed).

Now `--externs` goes through the same glob expansion as `--js`:

- Patterns like `externs/*.js` are expanded against the filesystem.
- Exclusion patterns (`!path/to/skip.js`) are respected.
- A pattern that matches zero files errors out with `JSC_NO_JS_FILES_FOUND_FOR_PATTERN`-style behavior.
- The error is prefixed `--externs: ...` so the user sees which flag's glob was bad without re-reading the command line.

The resolved externs list is discarded today — the goal of this slice is to catch typos at config-validation time. When the typechecker bridge lands (CLOC11.07+), the resolved list will flow into the typecheck stage.

### Internal: `IoConfig.externs` shape

Refactored from `Vec<PathBuf>` to `Vec<String>` (raw pattern strings). Resolution happens in `run.rs::resolve_externs` rather than `wire.rs`, keeping `wire.rs` pure (no FS I/O during config build). Matches the `js_patterns`/`resolve_inputs` shape.

- **New `CompilerError::ExternsGlobExpansion(GlobError)` variant** + Display arm with `--externs: ` prefix.
- **New `resolve_externs(&CompilerConfig) -> Result<Vec<PathBuf>, CompilerError>`** helper. Empty-externs fast path (most invocations don't pass it) bypasses the glob machinery.
- **Pipeline insertion**: Step 1.25 in `run_compiler`, right after `resolve_inputs`. So both `--js` and `--externs` patterns are validated before any transform pass runs.
- **5 new unit tests** in `run::tests` (empty → empty, real-files → expanded, missing → typed error, end-to-end flag-prefix Display, happy path with both `--js` + `--externs`).
- **Diff fixture** `tests/diff/externs-missing/` exercising the missing-pattern error path end-to-end.
- **New integration test** `tests/diff_externs_missing.rs` pinning the `--externs:` flag prefix + missing-path in the error.

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Resolve `--js` globs
2. **Resolve `--externs` globs (CLOC11.05, NEW) — validate-only today, flows into typecheck post-CLOC11.07**
3. `--print_tree` short-circuit (CLOC11.52)
4. `--print_tree_json` short-circuit (CLOC11.53)
5. Per-input `transform_source` (level + defines)
6. Concatenate transformed inputs
7. `--checks_only` short-circuit (CLOC11.51)
8. `--emit_use_strict` prepend (CLOC11.18)
9. `--output_wrapper` substitution (CLOC11.30, validated CLOC11.32)
10. `--isolation_mode IIFE` wrap (CLOC11.31)
11. Write to `--js_output_file` or stdout

## [0.14.0] - 2026-05-25

### Changed — CLOC11.32: `--output_wrapper` missing `%output%` is now a typed error

Behavioral compat slice with CC's `AbstractCommandLineRunner.checkFlags`. When `--output_wrapper` (or `--output_wrapper_file`) is set but the resolved template contains no `%output%` placeholder, closurec now errors out with the **exact** CC message:

```
ERROR - No %output% placeholder in the output wrapper
```

Previously closurec accepted any template silently — which meant a typo'd wrapper (e.g. `(function(){%otput%})()`) produced output that didn't contain the compiled JS at all, leaving the user to chase a confusing empty-bundle bug.

- **New `WrapperError::MissingOutputPlaceholder` variant** + Display arm pinned to CC's wording so toolchains that grep stderr for the message keep working when they swap `closure-compiler.jar` for `closurec`.
- **Validation runs in `apply_output_wrapper` after template resolution** — i.e. after `--output_wrapper_file` content is read. So a bad wrapper coming from a file produces the same typed error as a bad inline `--output_wrapper`.
- **Empty wrapper is still pass-through.** An empty/absent wrapper means "no wrapping requested," not "user supplied an invalid wrapper" — the fast-path early return for empty templates runs *before* the validation.
- **7 new unit tests** in `wrapper::tests` (inline-missing errors, exact CC message wording, empty wrapper pass-through still works, file-missing-placeholder also errors, happy path still works with placeholder, `%n%` still expands alongside `%output%`, `std::error::Error` impl pinned).
- **1 unit test updated** (`wrapper_without_output_placeholder_drops_compiled_js` → `wrapper_without_output_placeholder_errors_per_cc`).
- **Diff fixture** `tests/diff/output-wrapper-error/` exercising the error path end-to-end.
- **New integration test** `tests/diff_output_wrapper_error.rs` pinning the CC-compat message + non-zero exit.

### Pipeline matrix (unchanged structurally)

Same 10-step pipeline as 0.13.0; the change is that step 7 (`--output_wrapper` substitution) now rejects placeholder-less templates instead of silently passing them through.

## [0.13.0] - 2026-05-25

### Added — CLOC11.54: `--help_markdown` markdown flag dump

Fourth slice of Track 11 (special modes). When `--help_markdown` is set, closurec prints a markdown document listing every flag in the CLI spec — name, type, default, description — and exits successfully. Mirrors CC's `--help_markdown`, intended for documentation tooling that pipes the output into a docs page.

- **Wire format**: per-flag `### `--long` (type, default: X)` heading + body description. Heading-per-flag (not table) chosen so a diff stays readable when flags are added or descriptions change, and so GitHub's auto-anchors give linkable section IDs.
- **Pipeline placement**: short-circuit in `main::parse_and_run` Step 3.5 — after `cfg` is built from parsed flags, before `run_compiler`. So a config-level user error (e.g. invalid `--define` value) still surfaces, but the markdown dump replaces the rest of the run.
- **No new dependencies**. Uses `cli_builder::types::{CliSpec, FlagDef}` directly (already a transitive dep) and `serde_json::Value` for default-value rendering (already in the dep tree).
- **Spec re-use**: clones the loaded `CliSpec` before passing it to `Parser::new` so the help-markdown branch can iterate the flag list after parsing. The clone is ~10 KB; cheap.
- **7 new unit tests** in `help_markdown::tests` (title, version line, one section per flag, type+default in heading, body carries description, empty-string default disambiguated, no-default omits clause).
- **2 new unit tests** in `main::tests` (flag emits markdown, doesn't run pipeline even with bogus `--js`).
- **Diff fixture** `tests/diff/help-markdown/` with the full pinned markdown output (~400 lines, 100 flags).
- **New integration test** `tests/diff_help_markdown.rs`. Pins the exact output so any change to the user-facing flag surface — a new flag, a renamed flag, a re-described flag — fails the diff and must be acknowledged by regenerating `expected.stdout`.

### Pipeline matrix (cumulative across CLOC11)

`main::parse_and_run`:
1. Load embedded `cli.spec.json`
2. cli-builder parses argv → typed flags
3. `wire::config_from_parsed` → `CompilerConfig`
4. **`--help_markdown` short-circuit (CLOC11.54, NEW) — markdown dump, return**
5. `run::run_compiler(&cfg)`:
   1. Resolve `--js` globs
   2. `--print_tree` short-circuit (CLOC11.52)
   3. `--print_tree_json` short-circuit (CLOC11.53)
   4. Per-input `transform_source` (level + defines)
   5. Concatenate transformed inputs
   6. `--checks_only` short-circuit (CLOC11.51)
   7. `--emit_use_strict` prepend (CLOC11.18)
   8. `--output_wrapper` substitution (CLOC11.30)
   9. `--isolation_mode IIFE` wrap (CLOC11.31)
   10. Write to `--js_output_file` or stdout

## [0.12.0] - 2026-05-25

### Added — CLOC11.53: `--print_tree_json` JSON token-stream dump

Third slice of Track 11 (special modes), companion to `--print_tree` from 0.11.0. When `--print_tree_json` is set, closurec dumps the lexer's token stream as a JSON document to stdout and exits. Same diagnostic intent as CC's `--print_tree_json` — until our parser produces the typed AST (CLOC11.07+ bridge), tokens are the closest analogue.

- **Two wire shapes** depending on input count:
  - **Single file** (typical): a bare JSON array of token objects:
    ```json
    [
      {"type": "KEYWORD", "value": "var"},
      {"type": "NAME", "value": "x"}
    ]
    ```
  - **Multi-file**: an array of file-objects so consumers can disambiguate which tokens came from which file:
    ```json
    [
      {"path": "a.js", "tokens": [{"type": "KEYWORD", "value": "var"}]},
      {"path": "b.js", "tokens": [{"type": "KEYWORD", "value": "let"}]}
    ]
    ```
- **Same trivia + EOF filter** as `--print_tree`. Comments/whitespace/newlines/indent/dedent never appear; significant tokens only.
- **Hand-rolled JSON emission** (no `serde_json` dep) to keep the format byte-stable for diff fixtures. Escapes `"`, `\`, U+0000..U+001F (short forms for `\b \f \n \r \t`); non-ASCII printables pass through as UTF-8.
- **Pipeline placement**: extends Step 1.5's short-circuit alongside `--print_tree`. If both flags are set, `--print_tree` (older, simpler) wins. Glob expansion still runs; the rest of the pipeline (transform, wrap, write) is skipped. `--js_output_file` is ignored.
- **6 new unit tests** in `print_tree::tests` (empty → `[]`, one-object-per-token, trivia drop, quote/backslash escaping, control-char escape, bracket framing).
- **5 new unit tests** in `run::tests` (single-file array, multi-file file-objects, no-write-when-output-file-set, both-flags-set precedence, lex-error surfaces).
- **Diff fixture** `tests/diff/print-tree-json/` with `expected.stdout` pinned for `var x = 1;`.
- **New integration test** `tests/diff_print_tree_json.rs`.

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Resolve `--js` glob patterns
2. `--print_tree` short-circuit (CLOC11.52)
3. **`--print_tree_json` short-circuit (CLOC11.53, NEW) — JSON token dump, return**
4. Per-input `transform_source` (level + defines)
5. Concatenate transformed inputs
6. `--checks_only` short-circuit (CLOC11.51)
7. `--emit_use_strict` prepend (CLOC11.18)
8. `--output_wrapper` substitution (CLOC11.30)
9. `--isolation_mode IIFE` wrap (CLOC11.31)
10. Write to `--js_output_file` or stdout

## [0.11.0] - 2026-05-25

### Added — CLOC11.52: `--print_tree` token-stream dump

Second slice of Track 11 (special modes). When `--print_tree` is set, closurec dumps the lexer's token stream to stdout and exits without running the rest of the pipeline. Stand-in for the upstream Java Closure Compiler's `--print_tree`, which dumps the parsed AST — until our parser produces the typed AST (CLOC11.07+ bridge), the token stream is the closest analogue diagnostic users actually find useful.

- **Wire format.** Per input file:
  - One banner line `=== <path> ===\n`.
  - One line per significant token: `<TYPE_NAME>\t<value>\n`.
  - Trivia (comments, whitespace, newlines, indent/dedent) and EOF filtered.
  - `TYPE_NAME` is the grammar-supplied `type_name` when present, else the upper-cased `TokenType` debug name (fallback).
- **New module `print_tree`** holds the pure-string formatter `format_token_dump(&str, EsVersion) -> Result<String, PrintTreeError>`. 5 unit tests inline.
- **Pipeline insertion.** Added a "Step 1.5" guard at the top of `run_compiler`, right after `resolve_inputs` returns — before `transform_source`, `--checks_only` short-circuit, wrapping, and write. So:
  - Glob expansion still runs (catches `JSC_NO_JS_FILES_FOUND_FOR_PATTERN`-equivalent errors).
  - The compilation-level transform and the rest of the pipeline are skipped entirely.
  - `--js_output_file` is ignored under `--print_tree` (CC's behavior too — diagnostic dumps go to stdout).
- **New `CompilerError::PrintTree(print_tree::PrintTreeError)`** variant + Display arm so lex failures during the dump surface as typed errors, not panics.
- **Diff fixture** `tests/diff/print-tree/` with input/, flags.txt, and pinned expected.stdout for `var x = 1;`.
- **New integration test** `tests/diff_print_tree.rs`.
- **4 new unit tests** in `run::tests`: basic dump with banner, multi-file banner ordering, no-write-when-output-file-set, lex-error-surfaces.

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Resolve `--js` glob patterns
2. **`--print_tree` short-circuit (CLOC11.52, NEW) — token-stream dump, return**
3. Per-input `transform_source` (level + defines)
4. Concatenate transformed inputs
5. `--checks_only` short-circuit (CLOC11.51)
6. `--emit_use_strict` prepend (CLOC11.18)
7. `--output_wrapper` substitution (CLOC11.30)
8. `--isolation_mode IIFE` wrap (CLOC11.31)
9. Write to `--js_output_file` or stdout

## [0.10.0] - 2026-05-25

### Added — CLOC11.51: `--checks_only` mode skips emission

First slice of Track 11 (special modes). When `--checks_only` is set, closurec validates the inputs (runs transform_source over each, so any tokenizer/parser errors still surface) but emits **no** JS — no stdout text, no file write. Matches CC's behavior.

- **Pipeline insertion.** Added a guard at "Step 2.25" in `run_compiler`, right after the per-input transform loop accumulates `combined` but before `--emit_use_strict` prepend, `--output_wrapper` substitution, and `--isolation_mode IIFE` wrap. So:
  - The tokenizer/transform validation still runs (errors propagate normally).
  - The wrapping/write stages never run when `checks_only` is true.
  - Returns `CompilerOutput { stdout_text: "", wrote_files: [] }`.
- **CI-script-friendly semantics.** Exit code 0 on validation success; non-zero on any error from earlier stages (lex/glob/IO). Matches what a CI invocation expects from a `closure-compiler --checks_only` lint step.
- **No `--js_output_file` interaction**: even when set, no file is written. Pinned by `checks_only_does_not_write_output_file`.
- **Diff fixture** `tests/diff/checks-only/` with an empty `expected.stdout`.
- **New integration test** `tests/diff_checks_only.rs`.
- **3 new unit tests** in `run::tests`: empty-output basic, no-write-when-output-file-set, lex-error-still-surfaces.

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Per-input `transform_source` (level + defines)
2. Concatenate transformed inputs
3. **`--checks_only` short-circuit (CLOC11.51, NEW) — return empty if set**
4. `--emit_use_strict` prepend (CLOC11.18)
5. `--output_wrapper` substitution (CLOC11.30)
6. `--isolation_mode IIFE` wrap (CLOC11.31)
7. Write to `--js_output_file` or stdout

### Behavior changes (user-visible)

- `closurec --checks_only --js app.js` now actually skips emission. Previously the flag was parsed but ignored (output emitted anyway).
- `closurec --checks_only --js broken.js` still surfaces lex/parse errors (exit 2) — validation runs even though emission is skipped.

### Tests

120 unit + 8 integration tests passing. Clippy clean.

### Version

Bumps closurec 0.9.0 → 0.10.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.9.0] - 2026-05-25

### Added — CLOC11.18: `--emit_use_strict` prelude

First slice of Track 3 (language level). When `--emit_use_strict` is passed, closurec prepends `"use strict";` to the compiled output. Matches CC's behavior.

- **Pipeline ordering.** The directive is prepended to `combined` *before* both `--output_wrapper` template substitution and `--isolation_mode IIFE` wrapping. Reason: a `"use strict"` directive only takes effect when it's the *first* directive of the function body it governs. Both wrapping layers build syntactic envelopes around the body, so the directive has to sit just inside the innermost wrapper — which means we attach it to `combined` and let the outer wrappers wrap *around* it. Matches CC.
- **No `--output_wrapper_file` interaction.** Same ordering — the directive is part of the body that gets substituted into `%output%`.

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Per-input `transform_source`:
   - 1a. `--compilation_level` (WHITESPACE_ONLY active)
   - 1b. `--define / -D` substitution (CLOC11.19)
2. Concatenate transformed inputs (`combined`)
3. **`--emit_use_strict` prepend (CLOC11.18, new)**
4. `--output_wrapper` template substitution (CLOC11.30)
5. `--isolation_mode IIFE` wrap (CLOC11.31)
6. Write to `--js_output_file` (auto-create parents) or stdout

### Tests

4 new unit tests in `run::tests`:
- `emit_use_strict_prepends_directive` — basic prelude at top of output.
- `emit_use_strict_default_does_not_prepend` — flag off → no directive.
- `emit_use_strict_lands_inside_iife` — pipeline order pinned: directive sits between the IIFE opener and the body.
- `emit_use_strict_lands_inside_output_wrapper` — directive sits inside the `%output%` slot of a user template.

Plus diff fixture `tests/diff/emit-use-strict/` + new integration test `tests/diff_emit_use_strict.rs`.

117 unit + 8 integration tests passing. Clippy clean.

### Behavior changes (user-visible)

- `closurec --emit_use_strict --js app.js` now actually emits the directive. Previously parsed but ignored.

### Version

Bumps closurec 0.8.0 → 0.9.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.8.0] - 2026-05-25

### Added — CLOC11.31: `--isolation_mode IIFE` wrapping

Companion to CLOC11.30 in Track 6. When `--isolation_mode IIFE` is passed, the compiled output is wrapped in `(function(){…}).call(this);` — matching CC's `CompilerOptions` IIFE behavior. `--isolation_mode NONE` (the default) is unchanged.

- **New `wrapper::apply_iife_wrap(compiled) -> String`.** Emits `(function(){<compiled>}).call(this);` — using `.call(this)` rather than the simpler `()` form to preserve outer `this` binding the same way CC has since the option was introduced. Pinned by a test (`iife_wrap_uses_call_this_not_bare_invocation`) so a future "simplification" can't silently regress.
- **Pipeline ordering.** IIFE wrapping runs *after* `--output_wrapper` template substitution but *before* writing to disk/stdout. So a `--output_wrapper '// banner%n%%output%'` + `--isolation_mode IIFE` produces `(function(){// banner\n<compiled>}).call(this);` — banner sits *inside* the IIFE, matching CC's layered behavior.
- **Diff fixture** `tests/diff/isolation-iife/` per CLOC11 §3.
- **New integration test** `tests/diff_isolation_iife.rs` drives the built binary against the fixture.
- **4 new unit tests** in `wrapper::tests`: basic wrap, empty body, content-preservation, `.call(this)` form is pinned.

### Pipeline matrix (cumulative across CLOC11)

After CLOC11.31 lands, `run_compiler` does:

1. Per-input `transform_source`:
   - 1a. `--compilation_level` (CLOC11.06: WHITESPACE_ONLY active)
   - 1b. `--define / -D` substitution (CLOC11.19)
2. Concatenate transformed inputs (`combined`)
3. `--output_wrapper` template substitution (CLOC11.30)
4. **`--isolation_mode IIFE` wrap (CLOC11.31, new)**
5. Write to `--js_output_file` (auto-create parents) or stdout

### Behavior changes (user-visible)

- `closurec --isolation_mode IIFE --js app.js` now actually wraps. Previously the flag was parsed but ignored.

### Tests

113 unit + 7 integration tests passing. Clippy clean.

### Version

Bumps closurec 0.7.0 → 0.8.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.7.0] - 2026-05-25

### Added — CLOC11.30: `--output_wrapper` template substitution

Third behavioral slice of [CLOC11], landing the first piece of Track 6 (output formatting). `closurec` now honors `--output_wrapper <template>` (and the companion `--output_wrapper_file <path>`) end-to-end.

- **New `wrapper` module.** Single forward-scan template substituter recognizing two placeholders per CC's documented behavior:
  - `%output%` → the compiled JS (the result of all prior pipeline stages: transform_source + defines + concatenation).
  - `%n%` → a literal newline character.
  
  Unrecognized `%name%` placeholders (e.g. `%foo%`) pass through verbatim — CC's behavior. Lone `%` signs without a closing partner before a non-name character (e.g. `50% off`) also pass through unchanged.

- **`--output_wrapper_file` overrides `--output_wrapper`** when both are supplied. The file's contents become the wrapper template. Matches CC's documented behavior ("loads the specified file and passes its contents to `--output_wrapper`").

- **Pipeline ordering**: applied in `run_compiler` *after* the per-input transform and concatenation, *before* writing to disk or stdout. So the wrapper sees the final compiled JS — including everything WHITESPACE_ONLY, defines, and any future passes contributed.

- **Fast-path passthrough.** When neither `--output_wrapper` nor `--output_wrapper_file` is set, `apply_output_wrapper` returns the compiled string unchanged without allocating. The common case stays cheap.

- **New `CompilerError::Wrapper(WrapperError)`** variant. The single failure path today is `--output_wrapper_file` pointing at a non-readable path; we surface a typed `WrapperFileReadError` with the path, `io::ErrorKind`, and message.

- **Diff fixture** `tests/diff/output-wrapper/` per CLOC11 §3: a tiny input file, a flags file invoking `--output_wrapper '(function(){%output%})();'`, and the expected wrapped output.

- **New integration test** `tests/diff_output_wrapper.rs` drives the built binary against the fixture and asserts byte-equal stdout.

- **14 new unit tests in `wrapper::tests`** covering: no-wrapper passthrough, `%output%` substitution, `%n%` newline expansion, unrecognized placeholder passthrough, lone `%` passthrough (`50% off`), wrapper without `%output%`, multiple `%output%`s all substitute, `--output_wrapper_file` override, missing file → typed error, trailing `%n%`, empty compiled + wrapper, Unicode-in-template, error display, low-level scanner edge cases.

### Pipeline matrix (cumulative across CLOC11)

After CLOC11.30 lands, `transform_source` per input does:

1. `--compilation_level` transform (CLOC11.06: WHITESPACE_ONLY active; others identity until CLOC11.07+).
2. `--define / -D` substitution (CLOC11.19).

Then `run_compiler` does:

3. Concatenation of transformed inputs.
4. **`--output_wrapper` template substitution (CLOC11.30, new).**
5. Write to `--js_output_file` (auto-create parent dirs) or stdout.

### Behavior changes (user-visible)

- `closurec --output_wrapper '(function(){%output%})();' --js app.js` now actually wraps. Previously the flag was parsed but ignored.
- `closurec --output_wrapper_file banner.txt --js app.js` reads `banner.txt` as the wrapper template.

### Tests

109 unit + 6 integration tests passing. Clippy clean.

### Version

Bumps closurec 0.6.0 → 0.7.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.6.0] - 2026-05-25

### Added — CLOC11.19: `--define / -D` value substitution

Second behavioral slice of [CLOC11]. Users can now pass `--define NAME=value` (or `-D NAME=value`) and closurec will substitute every reference to `NAME` with `value` in the output.

- **New `defines` module.** Token-level substitution: tokenize via `javascript-lexer`, walk tokens, replace any identifier-type token whose value matches a `--define` key with the typed value rendered as JS source. Keywords (`if`, `var`, etc.) are explicitly NOT eligible. String-literal content is NOT substituted — `"DEBUG"` stays a string even if `DEBUG` is defined.
- **`DefineValue` rendering** for each variant of [`crate::config::DefineValue`]:
  - `Bool(true)` → `true`
  - `Bool(false)` → `false`
  - `Null` → `null`
  - `Number(42.0)` → `42` (integer-valued doubles emit without trailing `.0`, matching CC)
  - `Number(3.14)` → `3.14`
  - `Number(NaN)` → `NaN` sentinel
  - `Number(Infinity)` → `Infinity` (or `-Infinity`)
  - `String("hi")` → `"hi"` (re-quoted with JS escapes for `"`, `\`, LF, CR, TAB)
- **`transform_source` now runs in two phases:**
  1. **Level transform** — WHITESPACE_ONLY / identity per the compilation level (CLOC11.06 behavior).
  2. **Define substitution** — applies `cfg.defines.defines` over the level's output.
  This ordering means `--define DEBUG=false` composes naturally with `--compilation_level WHITESPACE_ONLY` (or with any future level transform).
- **Fast path:** when `cfg.defines.defines` is empty, `apply_defines` is a string-copy no-op (skips tokenization entirely).
- **New `CompilerError::Define(defines::DefineError)`** variant for substitution failures (currently only "tokenizer rejected the source").
- **Diff fixture `tests/diff/define/`** per CLOC11 §3.
- **New integration test `tests/diff_define.rs`** drives the actual binary against the fixture.
- **17 new unit tests in `defines::tests`** covering: empty defines passthrough, every DefineValue variant (bool/integer/fractional/string/null), case-sensitive identifier matching (`DEBUG` doesn't match `debug`), string-literal content protection, word-boundary preservation (`return DEBUG` → `return false`, not `returnfalse`), no-space-around-punctuation, multiple defines, keyword non-substitution, NaN/Infinity sentinels, embedded-quote re-escape, error display.

### Looseness vs. real CC

This is v1: we substitute *every* reference to a `--define` name, not only references to `goog.define`-annotated variables. In practice this matches what users expect when they pass `--define FLAG_DEBUG=false` for a flag they own. The cases where CC would NOT substitute (e.g. a `var FLAG_DEBUG` shadowing the same name) are rare in real builds. CLOC11.21+ will tighten the rule once we have JSDoc `@define`-aware metadata.

### Behavior changes (user-visible)

- `closurec --define DEBUG=false --js app.js` now actually substitutes `DEBUG` references in the output. Previously the flag was parsed but ignored.
- As a side-effect of routing through the tokenizer, the substitution output is already minified (single-space gap between word-like tokens, no spaces elsewhere). CC's WHITESPACE_ONLY does the same thing; we just get it for free.

### Tests

95 unit + 5 integration tests passing. Clippy clean.

### Version

Bumps closurec 0.5.0 → 0.6.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.5.0] - 2026-05-25

### Added — CLOC11.06: `--compilation_level WHITESPACE_ONLY` wired

First *behavioral* compilation-level slice of [CLOC11]. CLOC11.01–03 wired the I/O layer; CLOC11.06 starts actually transforming JavaScript. The closurec binary now honors `--compilation_level WHITESPACE_ONLY` end-to-end, matching Closure's documented behavior at this level.

- **New `whitespace_only` module.** Token-level minifier: tokenize via `javascript-lexer::tokenize_javascript_typed`, drop trivia (comments / whitespace / newlines), re-stitch survivors with the minimum-necessary inter-token whitespace. Conservative space-insertion rule: a single space goes between two adjacent *word-like* tokens (identifier, number, keyword, regex, template, BigInt, private name); other adjacencies emit back-to-back.
- **String-literal re-quoting.** The lexer's `Token.value` is *unescaped* content (escape sequences resolved), so emitting it raw would corrupt `var s = "a\"b"`. The minifier re-quotes string tokens with double quotes and re-escapes `"`, `\`, LF, CR, TAB. Matches CC's WHITESPACE_ONLY canonicalization.
- **`transform_source(source, config)` dispatch added to `run.rs`.** New per-level matrix:
  - `WhitespaceOnly` → call into `whitespace_only::whitespace_only_minify`.
  - `Simple` / `Advanced` / `Bundle` / `TranspileOnly` → identity for now; CLOC11.07–10 land their real bodies.
- **`map_language_in_to_es_version`** projects `LanguageVersion` enum → `EsVersion` for the lexer. `Stable` / `EcmascriptNext` / `Unstable` / `NoTranspile` shortcuts all resolve to `EsVersion::latest()` so modern JS isn't silently downgraded.
- **`CompilerError::Minify(MinifyError)`** new variant; carries the underlying tokenizer error message with the offending source context.
- **Two new crate dependencies**: `coding-adventures-javascript-lexer` (the `tokenize_javascript_typed` entry point) and `lexer` (the underlying `Token` / `TokenType` types from the grammar-driven lexer).
- **Diff fixture `tests/diff/whitespace-only/`** per CLOC11 §3: a JS input with line comments, block comments, mixed whitespace, function bodies — `expected.stdout` is the canonical compact emission.
- **New integration test `tests/diff_whitespace_only.rs`** drives the built binary against the fixture and asserts byte-equal stdout.
- **11 new unit tests in `whitespace_only::tests`**: empty input, line-comment stripping, block-comment stripping, whitespace collapsing around punctuation, space-between-keywords (`return typeof x`), space-between-keyword-and-number (`return 1`, must not become `return1`), no-space-around-punctuation, string-literal content preservation through re-quoting, multiline-to-single-line, mixed-comments-and-whitespace, error display.

### Behavior changes (user-visible)

- **`closurec --compilation_level WHITESPACE_ONLY --js foo.js`** now actually minifies. Previously this invocation ran the identity pipeline.
- All other levels still run identity (concatenation) — that changes in CLOC11.07+.

### Implementation note — operating at the token level (not the AST)

The CLOC09 typed AST (`javascript_ast::Program`) and the parser (which produces `GrammarASTNode`) don't yet have a bridge. WHITESPACE_ONLY doesn't need the AST — it operates on tokens — so this PR skips the bridge and uses the lexer directly. Building the AST bridge is on the critical path for CLOC11.07 (`--compilation_level SIMPLE`) and will land then.

### Tests

78 unit + 4 integration tests passing. Clippy clean.

### Version

Bumps closurec 0.4.0 → 0.5.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.4.0] - 2026-05-25

### Added — CLOC11.03: `--js_output_file` write semantics

Third implementation slice of [CLOC11] (drop-in Closure Compiler compatibility). CLOC11.01 wired `--js_output_file` to a simple `fs::write` call; this release brings the disk-write side to behavioral parity with the upstream Java tool.

- **Auto-create parent directories.** A `--js_output_file build/dist/app.min.js` no longer requires a preceding `mkdir -p build/dist`. The upstream `closure-compiler.jar` creates the parent tree automatically; we now match. Implemented as `fs::create_dir_all` on the resolved parent path, gated on `path.parent().is_some()` && `parent.exists() == false` so a bare filename in CWD doesn't try to `create_dir_all("")`.
- **`write_output_file(path, contents)` extracted as its own pub function.** Mirrors the CLOC11.02 pattern of splitting concerns into independently-testable units. The full pipeline (`run_compiler`) now calls it; unit tests can also call it directly.
- **Typed error on parent-create failure.** When `fs::create_dir_all` fails (e.g. the path collides with an existing regular file), we surface `CompilerError::OutputWriteError { path: <parent>, kind, message }` — the path field points at the parent so the user can fix the right thing.
- **Diff fixture `tests/diff/js-output-file/`** per CLOC11 §3: two .js inputs + flags.txt + expected.stdout.
- **Two new integration tests in `tests/diff_output_file.rs`**:
  - `js_output_file_writes_to_disk_with_auto_create_parents` — invokes the real binary with `--js_output_file <fresh-nested-path>`, asserts the file lands with the expected content and stdout stays empty.
  - `omitting_js_output_file_falls_back_to_stdout` — same fixture without the flag, asserts content lands on stdout.
- **Five new unit tests in `run::tests`**:
  - `write_output_file_creates_missing_parent_directories`
  - `write_output_file_bare_filename_does_not_create_dot` (regression: `parent()` of bare filename is `Some("")`; we must skip the `create_dir_all` rather than ask the OS to create an empty path)
  - `write_output_file_reports_create_dir_failure_as_typed_error` (file-where-directory-expected)
  - `run_compiler_autocreates_output_parent_dirs` (end-to-end)
  - `run_compiler_stdout_fallback_when_output_file_absent` (regression pin on the CLOC11.01 behavior)

### Known gap deferred to a follow-up

- **Empty-string value (`--js_output_file ""`) still rejected** by cli-builder's string validator at parse time (per `positional_resolver.rs`). The upstream Closure tool accepts it as a synonym for stdout. Closing this gap requires either (a) a cli-builder change to support `allow_empty: true` per-flag, or (b) a closurec-side argv preprocessor that special-cases the empty value. Both are out of scope for CLOC11.03 — tracked for a separate small PR. Workaround today: simply omit the flag to get stdout.

### What's NOT new

- v0.4.0 does not lex, parse, optimise, or emit JavaScript yet — the pipeline body remains "concatenate inputs". That work begins with CLOC11.06 (`--compilation_level WHITESPACE_ONLY`). CLOC11.03's value is making the I/O layer trustworthy for every later PR to build on.

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.3.0] - 2026-05-25

### Added — CLOC11.02: `--js` glob expansion + `!` exclusion

Second implementation slice of [CLOC11] (drop-in Closure Compiler compatibility). CLOC11.01 read `--js` values as literal file paths; this release replaces that with a real glob expander matching Closure's documented semantics.

- **New module `globs`.** Hand-rolled (zero-dep) glob matcher supporting:
  - `*` — matches any sequence within a single path segment.
  - `**` — matches zero or more whole path segments. Only special as a full segment per CC's docs; `src/**.js` is literal.
  - `?` — exactly one character within a segment.
  - `[abc]` / `[a-z]` / `[!abc]` — character classes with range and negation.
  - Literal text otherwise.
- **`!` exclusion.** A `--js` value starting with `!` removes everything it matches from the running included set. Mirrors Closure's behavior: `--js 'src/**/*.js' --js '!src/legacy/**'` includes all `src/` JS then drops the legacy subtree.
- **Walk strategy.** For each inclusion pattern we identify the longest fixed (glob-free) prefix and walk under it only — same optimisation as upstream `CommandLineRunner.findJsFiles`. Directory entries are sorted lexicographically before recursion so expansion is deterministic.
- **`resolve_inputs(config)`** extracted as its own pub function so glob behavior is unit-testable without going through full `run_compiler`. Result: `run_compiler` calls `resolve_inputs` first, then reads the resolved paths.
- **New `CompilerError::GlobExpansion(globs::GlobError)` variant** carrying the typed glob failure (NoMatches / InvalidPattern / WalkError) with the offending pattern.
- **Diff fixture `tests/diff/js-glob/`** per CLOC11 §3:
  - `input/` directory tree with 4 .js files including one excluded subtree.
  - `flags.txt` invoking `--js 'tests/diff/js-glob/input/**/*.js' --js '!tests/diff/js-glob/input/excluded/**'`.
  - `expected.stdout` with the concatenated content of the surviving 3 files in lex order.
- **`tests/diff_glob.rs`** integration test that runs the actual built binary against the fixture and asserts byte-equal output.

### Behavior changes (potentially user-visible)

- **Missing literal paths now error with `GlobExpansion(NoMatches)` instead of `InputReadError(NotFound)`**. Matches Closure's behavior (it emits `JSC_NO_JS_FILES_FOUND_FOR_PATTERN` regardless of whether the input was a glob or a literal). The `missing_input_returns_typed_error` test was updated to assert the new variant.
- **A `--js` invocation that produces zero matches is now a hard error** (exit code 2), even for literal paths. Closure does the same.

### Tests

21 new unit tests in `globs::tests`:
- 6 pure-function tests: literal vs glob detection, fixed-prefix splitting (including absolute paths), segment-matcher behavior for literals, `*`, `**`, `?`, char classes (positive, range, negative), invalid char class, error display.
- 9 filesystem-backed tests: literal-path passthrough, missing literal errors, `*.js`, `**/*.js` recursion, exclusion, no-matches error, invalid-pattern error, dedupe across overlapping inclusions, order preservation across patterns, subtree exclusion via `**`.

Plus the integration diff test brings the binary's total to 60 tests passing.

### Architecture

`globs.rs` is a single self-contained module under `code/programs/rust/closurec/src/`. No new crate dependencies. Per the repo's zero-dep working principle, this implements just enough of POSIX glob to match Closure's documented surface. Brace expansion (`{a,b}`), capture groups, and other features beyond Closure's surface are not supported and are not part of the v1 scope.

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.2.0] - 2026-05-24

### Added — CLOC11.01: CompilerConfig + identity build wiring

First implementation slice of [CLOC11] (drop-in Closure Compiler compatibility). Previously `closurec` validated argv, then printed `"closurec v0.1.0 - identity pipeline\n"` and exited — flag values were dropped on the floor. This release threads them through.

- **New module `config`.** A typed `CompilerConfig` struct with 18 per-feature sub-structs (`IoConfig`, `CompilationConfig`, `LanguageConfig`, `FormattingConfig`, `SourceMapConfig`, `DiagnosticsConfig`, `DefinesConfig`, `DependenciesConfig`, `ChunksConfig`, `PolyfillsConfig`, `RenamingReportsConfig`, `ExportsConfig`, `ConformanceConfig`, `InstrumentationConfig`, `SpecialModesConfig`, `SpecialPassesConfig`, `TranslationsConfig`, `JsonStreamsMode`). One sub-struct per row in CLOC11 §4's flag inventory, so later CLOC11.* PRs add lines, never new architecture.
- **New module `wire`.** `pub fn config_from_parsed(parsed: &ParseResult) -> Result<CompilerConfig, ConfigError>` translates cli-builder's `HashMap<String, serde_json::Value>` into the typed config. Every one of the 100 declared Closure Compiler flags gets read here; v1 of this PR only *uses* the I/O fields downstream, but all 100 flag slots are populated and tested.
  - `ConfigError::SpecMismatch` for "cli.spec.json says string but runtime got integer" — catches spec/wire drift loudly.
  - `ConfigError::InvalidDefine` for `--define NAME=value` values that aren't valid JS literals. Closure-strict semantics: bare unquoted strings rejected.
  - `ConfigError::Conflict` reserved for incompatible flag combinations in later PRs.
- **New module `run`.** `pub fn run_compiler(config: &CompilerConfig) -> Result<CompilerOutput, CompilerError>` executes the compiler. v1 = identity pipeline: read every `--js` literal path, concatenate with newline separators in input order, write to `--js_output_file` or stdout. CLOC11.02 will replace literal-path reads with glob expansion.
  - `CompilerError::InputReadError` / `OutputWriteError` carry the `io::ErrorKind` so callers format meaningfully without losing the underlying cause.
- **`main::parse_and_run` rewired.** The `ParserOutput::Parse` branch now calls `wire::config_from_parsed` → `run::run_compiler` and surfaces their results. Exit codes:
  - `0` — success (clean parse + successful compile).
  - `1` — argv parse error (unchanged).
  - `2` — compilation error (new; covers I/O failures and config validation).
- **23 new tests** across the three modules (config: 3, wire: 12, run: 7) plus updated existing CLI tests.

### Changed

- The "identity pipeline" banner now appears only when `--js` is absent. With `--js` inputs the binary reads + writes them.
- Pre-existing CLI-surface tests that fed nonexistent `--js` paths and pinned the banner string now assert "parses cleanly" (no `unknown`/`invalid` markers) rather than pinning the banner. The CLI *surface* contract is unchanged.

### Architecture notes

Per [CLOC11 §5], the bridge between cli-builder's untyped flag map and the compiler pipeline is one typed `CompilerConfig` with per-feature sub-structs. Adding a flag in any later CLOC11.* PR follows a fixed recipe:

1. Add a field to the appropriate sub-struct in `config.rs`.
2. Map it in the corresponding `read_*` function in `wire.rs`.
3. Consume it in `run.rs`.
4. Add a diff test under `tests/diff/<feature>/` (CLOC11 §3).

No new architectural pieces are needed per flag.

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.1.0] - 2026-05-23

### Added
- New program per CLOC08 — the CLI driver that ties together every crate in Stages 1–4 (lexer, parser, type sidecar, JSDoc extractor, type-checker, pass pipeline + every canonical pass per CLOC06, emitter, source-map generator).
- **Drop-in compatibility with the upstream Java Closure Compiler at the command-line surface.** A script written against `java -jar closure-compiler.jar --js foo.js --js_output_file out.js --compilation_level ADVANCED` works unchanged when the `java -jar …` invocation is swapped for `closurec`.
- All ~100 flags from `CommandLineRunner.java` declared in [`cli.spec.json`](./cli.spec.json), a [cli-builder](../../../packages/rust/cli-builder) JSON spec embedded into the binary via `include_str!`:
  - inputs/outputs: `--js`, `--externs`, `--js_output_file`, `--chunk`, `--chunk_output_path_prefix`, `--chunk_wrapper`;
  - compilation control: `--compilation_level` (`BUNDLE`/`WHITESPACE_ONLY`/`SIMPLE`/`TRANSPILE_ONLY`/`ADVANCED`), `--checks_only`, `--continue_after_errors`, `--use_types_for_optimization`;
  - language: `--language_in`/`--language_out` with the full ECMAScript-3-through-2021 + `STABLE`/`NEXT`/`UNSTABLE` enumeration;
  - source maps: `--create_source_map`, `--source_map_format`, `--source_map_location_mapping`, `--source_map_input`, `--apply_input_source_maps`, `--source_map_include_content`, `--parse_inline_source_maps`;
  - modules: `--module_resolution`, `--js_module_root`, `--process_common_js_modules`, `--rewrite_polyfills`, `--isolate_polyfills`, `--inject_libraries`, `--force_inject_library`;
  - warnings: `--warning_level`, `--jscomp_error`/`--jscomp_warning`/`--jscomp_off`, `--hide_warnings_for`, `--warnings_allowlist_file`, `--extra_annotation_name`;
  - renaming + reports: `--variable_renaming_report`, `--property_renaming_report`, `--rename_variable_prefix`, `--rename_prefix_namespace`, `--variable_map_input_file`, `--property_map_input_file`;
  - output shape: `--isolation_mode`, `--output_wrapper`/`--output_wrapper_file`, `--chunk_output_type`;
  - formatting: `--formatting` (repeatable enum: `PRETTY_PRINT`/`PRINT_INPUT_DELIMITER`/`SINGLE_QUOTES`), `--charset`, `--emit_use_strict`;
  - conformance + framework hooks: `--conformance_configs`, `--angular_pass`, `--polymer_version`, `--chrome_pass`, `--j2cl_pass`, `--remove_j2cl_asserts`;
  - defines: `--define name[=val]` (short `-D`);
  - coverage: `--instrument_for_coverage_option`, `--production_instrumentation_array_name`, `--instrument_mapping_report`;
  - dependency management: `--dependency_mode`, `--entry_point`;
  - tracing + debugging: `--debug`, `--print_tree`/`--print_tree_json`/`--print_ast`, `--print_source_after_each_pass`, `--tracer_mode`, `--logging_level`, `--summary_detail_level`, `--output_manifest`, `--output_chunk_dependencies`, `--help_markdown`;
  - dynamic imports: `--allow_dynamic_import`, `--dynamic_import_alias`;
  - JSON streams: `--json_streams` (`NONE`/`IN`/`OUT`/`BOTH`);
  - misc: `--browser_featureset_year`, `--env`, `--third_party`, `--flagfile`, `--num_parallel_threads`, `--continue_after_errors`, `--assume_function_wrapper`, `--assume_static_inheritance_is_not_used`, `--assume_no_prototype_method_enumeration`, `--renaming`, `--error_format`, `--expected_diagnostics`.
- Short aliases honored: `-O` → `--compilation_level`, `-W` → `--warning_level`, `-D` → `--define`.
- `--help` / `-h` and `--version` injected automatically by cli-builder; version sourced from `Cargo.toml`.
- `parse_and_run(&[String]) -> (String, ExitCode)` is a **pure function** with no I/O — tests drive it directly without spawning the binary.
- Exit codes: `0` success, `1` parse error, `70` internal error (`EX_SOFTWARE`).
- 15 tests covering: `cli.spec.json` loads cleanly (90+ flags), `--help` long + short produce help text, `--version` returns the crate version, canonical Closure invocations parse (`--js`/`--js_output_file`/`--compilation_level`/`--create_source_map`), `--js` is repeatable, unknown flag returns error mentioning the bad flag, invalid enum value returns error, short aliases (`-O`, `-W`, `-D`) work, `--formatting` is a repeatable enum, deprecated hyphenated alias `--checks-only` is rejected (known v0.1.0 gap — see notes), empty argv parses cleanly with defaults, `version_string_matches_crate_version` locks the Cargo.toml ↔ spec sync.

### Changed from the (unmerged) earlier draft
- The earlier `feat/scaffold-closurec` revision used a hand-rolled `std::env::args` parser and a custom flag surface (`--input`, `--output`, `--source-map BOOL`, `--ascii-only BOOL`, `--pretty BOOL`, `--disable NAME`). It was reworked **before merge** at user direction to (a) use `cli-builder` declaratively and (b) be drop-in compatible with the Java Closure Compiler. The custom flag surface is retired.

### Notes
- **Known compatibility gaps in v0.1.0**: cli-builder doesn't currently support multiple long-form aliases per flag, so a handful of deprecated upstream aliases are not implemented. Use the canonical name instead:
  - `--checks-only` → `--checks_only`
  - `--dev_mode` → `--jscomp_dev_mode`
  - `--warnings_whitelist_file` → `--warnings_allowlist_file`
  - `--D` (long form) → `--define` or `-D`
  Real-world Closure invocations use the canonical underscored names; these deprecated forms are rarely seen. Adding alias support to cli-builder is tracked as a v0.2 enhancement.
- v1 is scaffolding. The whole pipeline is identity today (`javascript-ast` ships only `Program` / `SourceType` per CLOC02 Phase 1), so a successful compile prints `closurec v0.1.0 - identity pipeline\n` and exits 0. Real wiring lands when the AST grows nodes. Pinning the Closure-compatible CLI surface now means scripts that invoke the Java tool today can target `closurec` with no flag changes when the body fills in.
- Dependencies: `cli-builder`; every crate scaffolded in Stages 1–4; `serde`/`serde_json`.
- Required capabilities: `fs.read` + `fs.write`. v1 doesn't actually touch the filesystem yet (identity body skips it) but the manifest declares the future surface.
- Source of truth: when upstream Closure Compiler adds a flag, `cli.spec.json` is updated and the binary picks it up via `include_str!`; no Rust code changes are required.
