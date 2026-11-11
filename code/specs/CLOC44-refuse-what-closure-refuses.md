# CLOC44 — refuse what Closure refuses (CCR-075)

**Status:** slices 1–4 implemented (private elements, return/new.target outside function, super outside method, with statement).
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
| line / column | 1-based line, 0-based column in UTF-16 code units |
| excerpt | `"  " + line + "| "` then the source line verbatim (including CRLF `\r`) |
| caret | indented by the excerpt prefix's width plus the column (tabs copied from excerpt) |
| separator | a blank line after each error |
| summary | `N error(s), 0 warning(s)` |
| order | source name (Java `String.compareTo`), then source position |
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

## Slice 2: `return` and `new.target` outside a function

Both are `JSC_PARSE_ERROR`s, reported at the statement's or expression's
first token, for every occurrence and at every level. Upstream's rule is
Closure's, not quite ECMA-262's, and it treats the two keywords
differently. An arrow counts as a function even for `new.target` at top
level. A class static block clears the context for `return` only. A static
block, a field initialiser and a method's computed key all see the context
*around* the class for `new.target`:

| input | upstream |
|---|---|
| `return;return;` | columns 0 and 7, "return must be inside function" |
| `if(a){return}`, `l:{return}` | refused |
| `x=()=>{return 1}`, `class A{m(){return new.target}}` | compiled |
| `()=>new.target` (top level) | compiled |
| `class A{static{return}}`, `class A{static{new.target}}` | refused |
| `class A{x=new.target}` | refused, "new.target must be inside a function" |
| `function f(){class A{x=new.target}}`, `function f(){class A{static{new.target}}}` | compiled |
| `function f(){class A{static{return}}}` | refused |
| `class A{static [new.target](){}}` | refused (the key is outside the method) |
| `class A{#x}return;` | private element at 8, then `return` at 11 |

These need structure that tokens can't give, so `refusals::scan` parses the
file with `parse_javascript_typed` and walks the tree iteratively
(`outside_function`). It parses only when a `return` keyword, or a `new`
keyword followed by `. target`, is present (not a string, not after `.`),
so other files pay nothing. Function-like nodes (declarations,
expressions, methods, accessors, generators, async forms and arrows) turn
function context on, which covers their parameters as well as their
bodies. Two flags travel down the tree, `return_ok` and `new_target_ok`:
`static_block` clears `return_ok` only, and a method's `property_name` child
keeps the flags from outside the method. A file
that doesn't parse yields no slice-2 refusals. SIMPLE and ADVANCED then
report their own parse failure; WHITESPACE_ONLY, which never parses,
compiles the file as before. That is fail-open, but only for inputs the
parser can't handle anyway, such as nesting past its rule-depth limit. All refusals are then sorted by position, so the kinds
interleave the way upstream prints them.

`simple-newtarget` and `simple-try-catch` now assert the identical refusal.
No ladder rung changes, because the ladder's `new_target` and `try_catch`
rungs use these constructs inside functions.

## Slice 3: `super` outside a method

Upstream reports `JSC_INVALID_SUPER_ACCESS` ("'super' may only be accessed within a method")
at the `super` keyword token when `super` is accessed outside a method.

| Where                                       | Upstream |
|---------------------------------------------|----------|
| `super.f(1 + 2);` (top level)               | refused, exit 1 |
| `x = () => super.x;` (top level)            | refused |
| `function f() { super.x; }`                 | refused |
| `class A { [super.x]() {} }`                | refused (computed key is outside method) |
| `class A { m() { super.x; } }`              | compiled |
| `class A { constructor() { super(); } }`    | compiled |
| `class A { m() { () => super.x; } }`        | compiled |
| `({ m() { super.x; } })`                    | compiled |
| `class A { x = super.y; }`                  | compiled |
| `class A { static { super.y; } }`           | compiled |

`refusals::scan` parses when a `super` keyword token is present (not after `.` or `?.`),
and checks `super_ok` during the AST walk. `simple-super` now asserts the identical
refusal (`expected.stderr`/`expected.exit`, empty `expected.stdout`).

## Slice 4: `with` statement

Upstream reports `JSC_USE_OF_WITH` ("The with statement cannot be used in strict mode.")
at the `with` keyword token at SIMPLE and ADVANCED compilation levels, exit status 1.
At WHITESPACE_ONLY, strict-mode validation is not enforced on scripts, matching upstream's
acceptance and emission of `with` in verified goldens like `minify_with_body_flatten`.

| Where | Upstream |
|---|---|
| `with (o) { x(1 + 2); }` at SIMPLE | refused, exit 1 |
| `function f() { with (o) {} }` at ADVANCED | refused, exit 1 |
| `with(o){a()}` at WHITESPACE_ONLY | compiled (`with(o)a();`), exit 0 |

`refusals::scan` parses when a `with` keyword token is present (not after `.` or `?.`),
and reports `with_statement` at SIMPLE and ADVANCED. `simple-with` now asserts the identical
refusal (`expected.stderr`/`expected.exit`, empty `expected.stdout`).

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
to WS 56/58, SIMPLE 37/58 and ADVANCED 25/58. The four CCR-081 `simple-private-*`
fixtures assert the identical refusal too.

## Later slices (one PR each)

The refusals already captured in `tests/oracle/upstream-refusals-v20260915.json`:

| Fixture | Upstream diagnostic |
|---|---|
| `simple-import`, `simple-export`, `simple-importexpr` | `JSC_INVALID_MODULE_PATH` (module resolution) |
| `simple-importmeta` | `JSC_CANNOT_CONVERT` (`import.meta`) |

Found since, and not yet captured as fixtures:

| Shape | Upstream |
|---|---|
| ADVANCED with an undeclared global (`f();throw 1;g()`) | `JSC_UNDEFINED_VARIABLE` "variable f is undeclared", once per undeclared name. Exit status is the error count (3 there). Default externs (`console`, …) count as declared. |
| `throw 1;class C{}` at SIMPLE/ADVANCED | `INTERNAL COMPILER ERROR`, exit 254. That is an upstream crash, not a refusal, so mirroring it is out of scope. |

Each later slice captures upstream's stderr and exit status into the fixture,
then implements the check. The undeclared-global slice needs the default
externs list, so it comes after the externs work.
