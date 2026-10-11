# closurec

`closurec` is the CLI driver for the Closure Compiler clone.
It tracks the upstream Java Closure Compiler's command-line flag names per
[CLOC08](../../../specs/CLOC08-closurec-cli-surface.md). All canonical options
and all seven upstream aliases in the pinned surface audit are accepted.
Semantic support still varies by flag, so accepting an invocation does not yet
mean its output is a drop-in replacement for the Java compiler.

The binary ties together every crate in Stages 1–4: lexer,
parser, type sidecar, JSDoc extractor, type-checker, pass
pipeline + every canonical pass per CLOC06, emitter, and
source-map generator. This crate is just the glue.

## How the CLI is built

`closurec` uses [`cli-builder`](../../../packages/rust/cli-builder)
for argument parsing. The flag surface is declared **declaratively**
in [`cli.spec.json`](./cli.spec.json), a cli-builder JSON spec
that mirrors `CommandLineRunner.java` upstream. The spec is
embedded into the binary via `include_str!` at compile time —
no runtime file lookup is required for the parser to come up.

```rust
const CLI_SPEC_JSON: &str = include_str!("../cli.spec.json");
let spec = load_spec_from_str(CLI_SPEC_JSON)?;
let parser = Parser::new(spec);
let output = parser.parse(&argv)?;
```

cli-builder handles:

- multi-token flags (`--js file.js`),
- enum-value validation (`--compilation_level ADVANCED`),
- repeatable flags (`--js a.js --js b.js --js c.js`),
- short aliases (`-O ADVANCED` ↔ `--compilation_level ADVANCED`),
- alternate long spellings (`--checks-only` ↔ `--checks_only`),
- type validation (integers, booleans, paths, enums),
- conflict checking,
- fuzzy "did you mean?" suggestions on unknown flags,
- `--help` and `--version` auto-injection.

When upstream Closure Compiler adds a flag, we update `cli.spec.json` and the
binary picks it up. The pinned surface audit below makes an unclassified local
or upstream change fail CI rather than relying on this prose.

## CLI surface

The full ~100-flag surface is in `cli.spec.json`. Highlights:

```
closurec --js src/foo.js --js src/bar.js \
         --js_output_file out/bundle.js \
         --compilation_level ADVANCED \
         --language_in ECMASCRIPT_2021 \
         --language_out ECMASCRIPT_2015 \
         --create_source_map out/bundle.js.map \
         --warning_level VERBOSE \
         --jscomp_off lintChecks \
         --define DEBUG=false \
         --formatting PRETTY_PRINT \
         --formatting SINGLE_QUOTES
```

`--js` and `--externs` accept literal files, `*`, `**`, `?`, character
classes, and leading-`!` exclusions. Paths use the host filesystem's native
component rules: Windows callers may use backslashes, forward slashes, or a
mixture after a drive or UNC root, while Unix keeps backslash as an ordinary
filename character. Matches are sorted deterministically within each inclusion
and duplicate files retain their first command-line position.

Short aliases the Java tool ships:

| Short | Long |
|-------|------|
| `-O`  | `--compilation_level` |
| `-W`  | `--warning_level` |
| `-D`  | `--define` |

Alternate long spellings retained by the Java tool are also accepted:

| Alternate long spelling | Canonical spelling |
|-------------------------|--------------------|
| `--D` | `--define` |
| `--checks-only` | `--checks_only` |
| `--dev_mode` | `--jscomp_dev_mode` |
| `--warnings_whitelist_file` | `--warnings_allowlist_file` |

`--typed_ast_output_file` is the current canonical upstream spelling.
`closurec` also accepts its historical misspelling
`--typed_ast_output_file__INTENRNAL_USE_ONLY` as a deprecated compatibility
alias; both spellings populate the same runtime configuration field, and
conflicting values fail closed.

### Machine-audited surface

[`tests/cli-surface/v20260915-audit.json`](./tests/cli-surface/v20260915-audit.json)
pins the 87,726-byte `CommandLineRunner.java` blob at upstream commit
`10ca677aff381d2c2e6e1b254ba32861e503173d`, including its Git blob ID and
SHA-256. The deterministic report classifies 102 upstream options, seven
upstream aliases, 112 local spec flags, two cli-builder-generated flags,
eleven local correlation-vector extensions, one deprecated compatibility
alias, and zero unsupported upstream aliases.

The offline `cli_surface` integration test verifies that every upstream and
local name has exactly one reviewed disposition, the report still hashes the
current `cli.spec.json`, and every supported short or long alias maps to its
reviewed canonical flag. To regenerate from an
independently obtained pinned source file:

```sh
cargo run --example cli_surface_audit -- \
  --upstream-source /path/to/CommandLineRunner.java \
  --output tests/cli-surface/v20260915-audit.json
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0    | Success. |
| 1    | CLI parse error, or SIMPLE/ADVANCED compilation failed during parse, typed-AST bridging, an optimization pass, or emission. |
| 2    | Execution failure outside the typed pipeline, including input/output and glob errors. |
| 70   | Internal error (`cli.spec.json` malformed — bug in *us*). `EX_SOFTWARE` per `sysexits.h`. |

Compiler diagnostics are written to stderr. A failed SIMPLE/ADVANCED compile
writes no JavaScript output and does not create the requested output file.

## Current scope

`closurec` reads input files, expands `--js` globs, and emits JavaScript.
`WHITESPACE_ONLY` uses a token path; `SIMPLE` and `ADVANCED` parse into the
typed JavaScript AST, run the registered optimization passes, and emit from that
AST. Source maps and correlation-vector traces are available. The typed levels
fail with a stage-specific error when parsing, bridging, a pass, or emission
cannot handle an input.

`BUNDLE` and `TRANSPILE_ONLY` remain identity passthroughs. The CLI surface is
broader than the implemented semantics, and the [measured parity](#measured-parity)
below shows substantial SIMPLE and ADVANCED gaps. Use the differential fixtures
and divergence ledger to evaluate a specific feature before relying on it in a
build.

## Compilation levels

`--compilation_level` (`-O`) selects how hard the compiler works:

| Level | What it does |
|-------|--------------|
| `WHITESPACE_ONLY` | Strips comments and inter-token whitespace only. Token-level; never parses to a typed AST. |
| `SIMPLE` | Runs `parse → bridge → <optimization passes> → emit`. It preserves top-level declarations under SIMPLE's open-world contract. The passes are `constant-fold`, `fold-control-flow`, `dce`, `inline-variables`, and `rename`; see the note below on the order they actually execute in. |
| `ADVANCED` | Runs the SIMPLE passes plus closed-world `inline`, `remove-unused-vars`, `treeshake`, and `rename-globals`; `rename-properties` runs only with an explicit externs boundary. More advanced-only passes remain planned. |
| `BUNDLE` / `TRANSPILE_ONLY` | Identity passthrough for now — module bundling and language down-levelling are orthogonal to the optimization pipeline and land separately. |

Pass order, and what `--correlation_vector` reports, are the *scheduler's*.
`closure-pass-pipeline` topologically sorts on each pass's declared
`depends_on` and uses registration order to break ties among ready passes.
The trace records the schedule that actually ran.

The `passes` field of a correlation-vector trace is taken directly from
`PipelineOutput::execution_order`. A pass can appear there only by having run:
there is no second list that could disagree with the pipeline.

Per-node tracing is still partial. CLOC27 carries lexer identities onto leaves;
[CLOC31](../../../specs/CLOC31-primitive-fold-lineage.md) connects string-literal
`.length`, primitive binary and primitive unary results to their operand
identities and records the rewrite on the replacement. The trace tests follow
actual parent links, including nested folds, rather than treating token presence
as lineage. Composite node spans, other transformations, deletion/motion/inlining
events and output-byte/source-map joins remain required by
[CCR-065](https://github.com/adhithyan15/coding-adventures/issues/15830).

[CV01](../../../specs/CV01-compact-compiler-identities.md) uses compact-v1
identities in the shared compiler log: `cv1.` plus sixteen hex digits, with
ancestry carried by parent edges. JSON/pretty JSON and NDJSON preserve the
allocator version and last-issued watermark, so reload cannot reuse IDs issued
while storage was disabled. Filtered sidecars explicitly declare partial views
and are rejected as full reloadable compact logs. NONE still computes the log
and optional summary without writing a sidecar.

Compact IDs bound identity length, not graph size or query/serialization work.
Checked resource limits and deterministic serialization are implemented locally;
native CI/merge acceptance and actual pass/sweep event chronology remain in
[#16868](https://github.com/adhithyan15/coding-adventures/issues/16868).
Allocation sequence and the scheduled `passes` list do not supply that chronology.

### Checked provenance limits

Tracing uses the checked compact graph with the shared `GraphLimits` defaults.
Use partial overrides for a build:

```text
closurec --js input.js --js_output_file output.js --correlation_vector \
  --correlation_vector_limits max_nodes=10000,max_output_bytes=1048576
```

The nine fields are `max_nodes` (1,000,000), `max_edges` (4,000,000),
`max_events` (4,000,000), `max_metadata_values` (1,000,000),
`max_metadata_depth` (64), `max_metadata_bytes` (134,217,728),
`max_input_bytes` (134,217,728), `max_work` (64,000,000), and
`max_output_bytes` (536,870,912). Byte fields count encoded provenance;
`max_input_bytes` applies to JSON snapshot import, not JavaScript source files.
Work allowances apply per graph operation. Omit the flag for defaults.
Overrides require unique known names and unsigned ASCII decimal values; zero
is allowed and metadata depth cannot exceed 64. CLI and programmatic settings
are validated before inputs. Provenance recording failures identify the stage
and return exit status 1 on stderr with empty success stdout. Snapshot and summary exports validate the complete graph and enforce shared
work/output allowances; pretty whitespace and NDJSON newlines count too. NONE
validates evidence without applying output-byte caps unless a summary is
materialized. Payloads are prepared before any JS/map/manifest/sidecar write.
The compiler preflights destinations, stages every complete file and installs
JS/map/manifest outputs before the sidecar. Normalized, ancestor, Windows case
and existing hard-link aliases reject; final symlinks, nonregular files and
read-only destinations reject. Reported failures before commit restore original
files in reverse order. Backup creation also refuses an occupied path; cleanup
owns only backup links it created, regardless of matching file identities.
Obstructed recovery retains original copies and names
their paths; cleanup failures after commit warn on stderr while the complete
output set remains successful. This requires filesystem object identities and
hard links. It provides complete individual files, without promising simultaneous
visibility of the whole set or recovery from a process crash.

Source-root origins record `content_sha256` for consumed UTF-8 bytes. JS/map/
manifest `wrote` events record the digest and byte length of final encoded bytes.
These hashes identify artifacts; they do not establish node-level output joins
or correct source-map mappings. Exact final security review and native-platform
CI/merge verification remain required before CV02 acceptance.

The Windows repair creates staging directories and files atomically with a
protected current-user DACL. It captures and rechecks owner/group/DACL/protection,
proves the intended policy and production fresh verification open on a distinct
always-empty probe, and verifies the installed policy exactly. Explicit legacy
and inherited policies use separately
verified handle routes; unsupported or stale inherited policies reject before
originals change. Pre-commit failure restores candidate privacy before rollback.
Native Windows regressions cover preparation, protected replacement, inherited
and new-file policy, ACL-only changes, rejection and rollback. PR #16905 remains
draft pending native-platform CI. Full-scope independent review passed repaired
head `3b7928e241`; native acceptance must also prove the compiler's test command
ran. Its first CI plan selected Closure on Windows but left the OS-suite gate
false, so the gate is now repaired to include the compiler and CV02 specification.
Independent review also reproduced a specification-only plan with that gate
enabled but no compiler or Rust toolchain. Exact specification-to-consumer
selection now supplies both on all three platforms and fails on a missing
consumer; production-plan regressions verify the actual native command.
The published candidate's Linux/macOS native compiler package commands passed;
the Windows command was skipped, so acceptance awaits refreshed native CI.
The updated head requires refreshed review and CI before acceptance.
The correctly gated `0e02820565` Windows job then failed its strict Clippy
preflight on constant-size SID chunking before compiler tests ran. The decoder
now uses fixed four-byte arrays after the same bounded SID checks, preserving
policy identity and word order. Matching-toolchain validation, fresh review
and another exact-head native run are required before acceptance.
Audit SACLs/integrity claims are unproven.

The next native Windows run at `c76de302f8` passed lint but failed the promised
OWNER RIGHTS policy rejection test. A deterministic native regression reproduced
successful fresh opening and publication with an earlier FullControl allow and
a later OWNER RIGHTS denial. The repair now checks structural admission before
any empty probe or original mutation: effective OWNER RIGHTS denied READ_CONTROL
or file generic rights reject across ordinary/object/callback denial encodings,
independently of ACE order and token capabilities. Unknown or malformed effective
encodings reject; inheritance-only ACEs are inert. Supported policies retain exact
ACE order and all existing fresh-open/identity/policy proofs. Diagnostics record
the inherited fixture's actual token, ACL and open outcome. Repaired-head review
and successful native compiler execution on all three platforms remain required.

The `0dfbcf7b10` Windows run passed the structural-policy controls but its inherited
fixture's optional token diagnostic found Git's Unix `whoami.exe` through PATH.
That utility rejected `/all` before publication was tested. The fixture now
resolves Windows' utility from the OS system directory and isolates optional
diagnostic status from strict ACL setup. A shadow-PATH regression verifies native
selection and intended policy; invalid setup paths still fail. The production
publication protocol and unconditional rejection requirement are unchanged.
Another reviewed-head native run is required before acceptance.

The Linux/macOS implementation supports ordinary owner/group/mode policies.
Descriptor-bound extended access/default ACL queries reject unsupported ACLs
before staging or original mutation. Distinct empty creation probes measure the
actual parent's group/setgid and umask behavior, prove group-assignment capability,
and permit fresh verification opens. Data-bearing files stay owner-only until
installation; final ownership precedes mode application and exact verification.
Failure restores candidate owner-only mode before cleanup, without requiring an
unnecessary group reset. Other Unix targets fail explicitly. Seven native cases
per platform are committed and strict Linux/macOS cross-checks pass; native Unix
runtime validation remains pending CI. Cross-compilation is not ACL runtime evidence.

### Measured parity

`tests/diff/ladder_*` is an ordered complexity ladder compiled by both
`closurec` and the pinned Closure `v20260915` oracle. The fixture and
divergence-ledger checks on 2026-09-26 cover 57 rungs:

| Level | Rungs agreeing | |
|-------|---------------:|---|
| `WHITESPACE_ONLY` | 54 / 57 | 95% |
| `SIMPLE` | 34 / 57 | 60% |
| `ADVANCED` | 24 / 57 | 42% |

All 57 rungs are committed as fixtures under `tests/diff/ladder_*`. Agreement falls as the amount of claimed optimization rises. The `ADVANCED`
figure flatters it: most of its agreements are rungs with nothing to optimize.
Upstream `ADVANCED` reduces whole programs to their observable effect —
`var o={a:1,b:2};console.log(o.a)` becomes `console.log(1)` — where `closurec`
emits approximately the input. Treat `ADVANCED` as scaffolded rather than
implemented.

Known divergences are recorded in `tests/ladder/divergences.json`, each with a
tracking issue, and are machine-checked: closing one fails the harness.

The SIMPLE pipeline:

```text
source ──parse──▶ grammar AST ──bridge──▶ typed Program
       ──passes──▶ optimized Program ──emit──▶ JS text
```

```sh
# SIMPLE evaluates constant expressions at compile time:
closurec --compilation_level SIMPLE --js in.js
#   var x = 1 + 2;   ⇒   var x=3;
```

SIMPLE and ADVANCED are fail-closed. If closurec's typed AST cannot represent a
valid construct yet, the command reports the `typed AST bridge` stage and the
unsupported grammar rule instead of silently returning WHITESPACE_ONLY bytes.
Malformed JavaScript likewise reports the `parse` stage and exits 1. This makes
an exit-0 result trustworthy: the requested typed pipeline actually completed.

## Architecture

```text
  args ─► cli_builder::Parser::parse ──► ParserOutput
                                            │
                                            ├─ Parse(r)   ──► wire config ──► run_compiler
                                            ├─ Help(h)    ──► h.text
                                            └─ Version(v) ──► v.version
```

`parse_and_run_with_streams(args)` returns `(stdout, stderr, ExitCode)` so tests
can exercise routing without spawning the binary. `parse_and_run(args)` is the
combined-stream compatibility helper; `main` only writes the returned streams.

## Reproducible upstream oracle

[`tests/oracle/manifest.json`](./tests/oracle/manifest.json) is the
machine-readable provenance registry for all 626 differential fixture
directories. It pins Google Closure Compiler `v20260915`, its annotated tag and
release commit, the exact Maven JAR URL, byte length and SHA-256, Java 21.0.12,
licensing, deterministic capture settings, and normalized command templates.

The 462 `minify_*` goldens have now been re-executed against that exact artifact
and Java 21.0.12. All 462 stdout captures are byte-identical to the checked-in
goldens, with zero changed and zero declined. The strict
[`minify-v20260915-report.json`](./tests/oracle/minify-v20260915-report.json)
preserves hashes, exit status, and complete stdout/stderr bytes for every case.
It also records the three successful legacy-octal compilations where upstream
emits warnings even though stdout matches. Local correlation-vector,
help/version, and transitional print-tree contracts still do not pretend to be
upstream bytes.

Oracle execution is an explicit maintainer action. After independently
obtaining and verifying the pinned artifact, run:

```sh
cargo run --example oracle_refresh -- \
  --oracle-jar /absolute/path/to/closure-compiler-v20260915.jar \
  --java /absolute/path/to/java-21.0.12
```

The tool never downloads Java or the JAR. It requires an absolute Java path,
enforces independent hard-coded trust pins and exact JVM argv, fully preflights
the cohort, and executes private snapshots of the verified JAR/input bytes with
JVM injection variables removed. Streamed size and aggregate budgets,
per-process timeout/output caps, child cleanup, before/after Java identity
checks, and atomic non-symlink report replacement keep regeneration fail-closed.
It fails before execution on artifact hash/size, Java version, cohort,
flag-shape, or path-containment drift. See
[`tests/oracle/README.md`](./tests/oracle/README.md) for the complete review
procedure.

The verifier is offline and does not execute Java:

```sh
cargo test --test oracle_manifest
cargo test --test oracle_refresh_report
```

It rejects unclassified or multiply classified fixtures, unknown schema data,
bad pins, unsafe/stale paths, incomplete harness mappings, false upstream
claims, malformed hashes/review links, altered capture bytes, and unreviewed oracle drift. A new fixture
therefore cannot inherit provenance merely because its directory happens to
match a glob. Normal tests and CI never download or execute the JAR.

## What's coming

- Complete typed-AST coverage so every JavaScript construct accepted upstream
  can remain on the SIMPLE/ADVANCED pipeline.
- Route remaining flags to pass configuration —
  `--jscomp_off`/`--jscomp_warning`/`--jscomp_error` populate
  the warning level map; `--define` populates a value map the
  passes consult; `--compilation_level` selects a canonical
  pass preset.

## Dependency whitelist

- `cli-builder` — declarative CLI parsing.
- Front end: `javascript-tokens`, `javascript-ast`.
- Type checker: `closure-typechecker`.
- Pass pipeline: `closure-pass-pipeline` + every pass crate
  (`constant-fold`, `fold-control-flow`, `dce`, `inline`,
  `rename`, `treeshake`, `collapse-properties`,
  `remove-unused-vars`).
- Back end: `closure-emitter`, `closure-source-map`.
- Shared: `correlation-vector`, `type-sidecar`, `serde`,
  `serde_json`; the explicit oracle maintainer target uses the repository's
  `sha256` crate as a development-only dependency.

Traced runs now start full `chronology-v1` recording before input operations.
The journal references graph facts and actual pass/pipeline contexts, including
their sweep/slot, accepted schedule and typed terminal outcomes. It distinguishes
invocations from the existing distinct `passes`/`execution_order` inventory.
JSON/pretty JSON retain complete records; NDJSON emits `_event` frames and a
state-only `_meta` footer. Filtered journals preserve original refs/watermark and
declare partial coverage. Full journals validate through both CV import APIs.
Resource or encoding failure follows the existing artifact-preservation
contract. This adds operation/invocation chronology; exact source/composite/
output-byte ownership and nonempty source-map mappings remain separate work.
