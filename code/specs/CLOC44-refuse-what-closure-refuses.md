# CLOC44 — refuse what Closure refuses (CCR-075)

**Status:** slice 1 implemented (private elements). The remaining slices are
listed below.
**Decision:** owner, 2026-10-19 (#15860, option a).
**Crate:** `closurec` (`src/refusals.rs`, `run.rs` step 1b, `tests/ladder.rs`).

## The rule

When the pinned reference compiler (`closure-compiler-v20260915`) rejects an
input, closurec rejects it the same way, at every compilation level:

- the same diagnostic key and message, location and stderr text, byte for byte;
- the same exit status;
- no JavaScript on stdout, no `--js_output_file`, no sidecars.

closurec does not compile a superset of what Closure compiles. If a newer pin
implements a feature, closurec follows that pin.

## Upstream's diagnostic format

Measured against the jar:

```text
a.js:12:10: ERROR - [JSC_UNSUPPORTED_LANGUAGE_FEATURE] This language feature is not currently supported by the compiler: Private elements.
  12| class B { static #s = 1; #m() { return #s in this; } get #g() { return 1; } }
                ^

…one block per error…
6 error(s), 0 warning(s)
```

| Element | Rule |
|---|---|
| file | as given on the command line, with runs of `/` collapsed (Java `File`) |
| line / column | 1-based line, 0-based column |
| excerpt | `"  " + line + "| "` then the source line verbatim |
| caret | indented by the excerpt prefix's width plus the column |
| separator | a blank line after each error |
| summary | `N error(s), 0 warning(s)` |
| order | input files in order, then source position |
| exit status | `min(N, 127)` |

### Bounded output, a deliberate divergence

Upstream prints every error with its full excerpt, so a hostile single line
(`#a;` repeated) makes its output grow with errors × line length. 30 KB of
input gives about 450 MB. closurec matches upstream byte for byte up to
`MAX_DIAGNOSTIC_BYTES` (16 MiB) of diagnostics. Past that, it omits the
remaining error blocks but still prints the true `N error(s)` summary and exit
status. A refusal stores only positions, and excerpts are read from each
source once, so memory stays proportional to the input. That same 30 KB input
peaks at about 48 MB, where an unbounded render took 1.25 GB.

Excerpts echo source lines verbatim, control characters included, as upstream
(and rustc, and gcc) do. That is parity, and is left as is.

## Slice 1: private elements

Upstream reports `JSC_UNSUPPORTED_LANGUAGE_FEATURE` ("Private elements.") for
**every** private name token. That covers declarations of fields, methods,
accessors, generators and async methods, every `this.#x` use, and `#x in o`.
A `static` private **field**, but not a method, gets a second error at the
`static` keyword:

| input | upstream columns |
|---|---|
| `class A { #x = 1; }` | 10 |
| `class A { static #x = 1; }` | 10, 17 |
| `class A { static #m() {} }` | 17 |
| `class A { get #g() {} }` | 14 |
| `class A { m() { return this.#x; } #x; }` | 28, 34 |

`refusals::scan` tokenizes each input before any compilation step and emits
one refusal per private name, plus the static-field extra. `run_compiler`
returns `CompilerError::Refused` before reading inputs for compilation, so
nothing is written.

## The ladder's third verdict

A rung upstream refuses records that refusal in its own fixture:
`expected.exit` and `expected.stderr` (captured from the jar), with an empty
`expected.stdout`. `declined_verdict` counts it as agreement
(`DeclinedIdentically`) when closurec exits with the same status, prints no
stdout, and prints the same stderr byte for byte. A ledger entry for such a
rung is then stale and fails the gate.

The refused cohort is pinned by name (`UPSTREAM_REFUSED`).

The three `ladder_t7_private_field_*` rungs move from recorded divergences to
declining identically. The ledger drops from 59 to 56 entries, and parity goes
to WS 55/57, SIMPLE 35/57 and ADVANCED 25/57. The four CCR-081 `simple-private-*`
fixtures assert the identical refusal too.

## Later slices (one PR each)

The refusals already captured in `tests/oracle/upstream-refusals-v20260915.json`:

| Fixture | Upstream diagnostic |
|---|---|
| `simple-import`, `simple-export`, `simple-importexpr` | `JSC_INVALID_MODULE_PATH` (module resolution) |
| `simple-importmeta` | `JSC_CANNOT_CONVERT` (`import.meta`) |
| `simple-newtarget` | `JSC_PARSE_ERROR` "new.target must be inside a function" |
| `simple-super` | `JSC_INVALID_SUPER_ACCESS` |
| `simple-try-catch` | `JSC_PARSE_ERROR` "return must be inside function" |
| `simple-with` | `JSC_USE_OF_WITH` |

Found since, and not yet captured as fixtures:

| Shape | Upstream |
|---|---|
| ADVANCED with an undeclared global (`f();throw 1;g()`) | `JSC_UNDEFINED_VARIABLE` "variable f is undeclared", once per undeclared name. Exit status is the error count (3 there). Default externs (`console`, …) count as declared. |
| `throw 1;class C{}` at SIMPLE/ADVANCED | `INTERNAL COMPILER ERROR`, exit 254. That is an upstream crash, not a refusal, so mirroring it is out of scope. |

Each later slice captures upstream's stderr and exit status into the fixture,
then implements the check. The undeclared-global slice needs the default
externs list, so it comes after the externs work.
