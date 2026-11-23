# KILN01 — The Kiln Language, Version 1

**Status:** Draft, for review. **Implementation not started.** This spec is the
one KILN00 §6 says is missing: KILN00 fixes *what a target means*, and KILN02
fixes *how an action becomes 32 bytes*, but neither fixes the text between
them. Until this document exists there is nothing for a parser, a linter or a
conformance port to agree on. It decides no new policy; every construct here
is one KILN00 §6 already sketches, written down precisely enough to implement.

**Layer:** Language definition for the build system (KILN00).

**Builds on:** [KILN00](KILN00-kiln-build-system.md) §4–§8 (vocabulary, file
sets, the language, phases, keys), [KILN02](KILN02-key-encoding.md) (the
canonical bytes), KILN03/KILN04 (legacy and traced reuse).

**Version:** the language is `kiln 1`. `key.SemanticsVersion == 1` is *this*
expansion semantics; version 0 is the legacy static key of KILN03 and version
2 is the traced key of KILN04. KILN02 §… reserves 1 for exactly this.

---

## 1. Scope and non-goals

This spec defines:

1. **The lexical layer (§3)** — bytes to tokens, including the indentation
   rule. This is the part every implementation must share to the byte, because
   a disagreement here becomes a different recipe and therefore a different
   key.
2. **The syntax (§4, §5)** — statements, fields, phases, steps, variables.
3. **Expansion (§6)** — how a BUILD file plus the rules it uses becomes a set
   of canonical actions, and what the canonical form is.
4. **The checks (§7)** — the properties KILN00 §6.1 requires (termination,
   determinism, settled inputs) and the diagnostic each violation gets.
5. **Two pure functions (§8)** — the conformance surface KILN00 §16 defines,
   with the golden vectors a port must reproduce.

It does **not** define: how a step is executed or isolated (KILN00 §9), how a
key is encoded (KILN02), how the ledger decides reuse (KILN00 §10), or which
rules exist (part of the repository, not of the language). A rule file is
data in this language; adding a language to Kiln is adding a `.rule` file.

## 2. The two kinds of file

| Kind | Path | Marker | Evaluated by |
|---|---|---|---|
| **BUILD** | any `BUILD` | `kiln 1` | every command; the planner |
| **rule** | `code/kiln/rules/<name>.rule` | `kiln-rule 1` | the planner, on demand |

A file's kind is decided by its **marker**, never by its path alone: the first
line that is neither blank nor a `#` comment must be exactly `kiln 1`
(a BUILD) or `kiln-rule 1` (a rule) for this version. Anything else is a
legacy shell BUILD, which the legacy tool runs and Kiln does not
(KILN00 §6.2).

**Why rules carry a marker KILN00 does not show.** KILN00 §6.3's rule
examples have no version line. That is a gap and not a decision: a rule is as
much a language artefact as a BUILD, and an unversioned rule file would be
the one file whose meaning could change under a commit that did not change it.
Adding the marker is therefore proposed here, and BUILD and rule version
numbers are independent so either can be bumped alone. *(Decision required:
whether to accept `kiln-rule 1`, or to leave rule files unversioned and
version them through the directory.)*

**Coexistence.** The legacy tool and its fixtures must skip a `kiln N` BUILD
(KILN00 §6.2); a Kiln `BUILD` beside a legacy `BUILD_windows` is a validator
error; the educational ports stay at the legacy level.

## 3. The lexical layer

### 3.1 Bytes and lines

A file is UTF-8. A byte order mark is refused, not stripped: two files that
differ only by a BOM would otherwise produce the same recipe. A NUL is
refused. A lone `\r` is refused; `\r\n` is accepted and equivalent to `\n`, so
a Windows checkout and a Linux one agree on the recipe (KILN00 §8 item 6 makes
the same argument for `.gitattributes`). A file that does not end in a newline
is accepted; the last line is still a line.

### 3.2 Indentation

Indentation is **spaces only** and a tab anywhere in the leading whitespace is
a parse error (KILN00 §6.2). Indentation is significant: it nests a block
under the statement above it.

The rule is the one Go used before gofmt made it moot, and it is chosen so an
editor cannot silently change meaning:

- A block is introduced by a statement ending in `:` and is the run of
  following lines whose indentation is **greater** than the introducer's.
- All lines directly inside one block must use the **same** indent width, and
  it must be greater than the parent's. A line that is deeper than its
  siblings but not opened by a `:` is `indent-unexpected`.
- The width itself is not fixed — two or four or eight are all legal — but it
  must be **consistent within a file**. A file that uses two spaces in one
  block and four in another is `indent-inconsistent`. This is deliberate: the
  width is a style choice, the *consistency* is a correctness property,
  because an inconsistent file reads differently from how it parses.
- Blank lines and comment-only lines do not participate; a block continues
  across them at its own indentation.

### 3.3 Tokens

| Token | Grammar |
|---|---|
| comment | `#` to end of line |
| identifier | `[A-Za-z_][A-Za-z0-9_-]*` |
| field name | identifier, or an identifier followed by `+` (the append form) |
| integer | `[0-9]+` |
| duration | integer followed by `s`, `m` or `h` (`20m`) |
| bare word | one or more of `[A-Za-z0-9_./+*?=-]`, never starting with `#` or a digit-invalid context; `**` and `*` are ordinary characters to the lexer (they are glob syntax to the *planner*, §6.4) |
| quoted string | `"…"` with `\"`, `\\`, `\n`, `\t`, `$$` escapes; a raw newline is refused |
| triple string | `"""…"""`, newlines allowed, no escapes, used only for `shell:` |
| list | `(a b c)`, and `()` for empty |
| punctuation | `:`, `=`, `+`, `$`, `(`, `)`, `,`, `#`, `|` |

Whitespace separates tokens; there is no semicolon and no line continuation.
A step that needs several lines uses `( … )` (§5.4).

### 3.4 Interpolation

`$name` interpolates a bound variable anywhere a bare or quoted word is
allowed. `${name}` is the explicit form for a name followed by characters that
would otherwise extend it. `$name` in a **list** position expands to one
element per item (§5.5). `$$` is a literal `$`, so a step can contain a shell
`$` without the `shell:` escape hatch.

There is no other expansion: no environment lookup, no command substitution,
no path expansion. A `$` followed by anything else is `interp-invalid`.

## 4. Statements

A file is a sequence of statements at indent zero (plus, in a rule, the
statements of the rule body).

| Statement | Where | Meaning |
|---|---|---|
| `kiln N` / `kiln-rule N` | first non-comment line | the marker (§2) |
| `load("path", "name")` | BUILD | import a rule by name; the path is repository-relative and must resolve to a `.rule` file |
| `let name = value` | both | bind an immutable value (§5.5) |
| `name: rule()` | BUILD | define a target (tier 1) |
| `name:` | BUILD | define a one-off target with no rule (tier 1) |
| `rule name(params):` | rule | define a rule (tier 2) |
| `<field>: value` | inside a target or rule body | a field (§5.1) |
| `<field> +: value` | inside a target or rule body | append to a field (§5.2) |
| `<phase>:` | inside a target or rule body | open a phase block (§5.3) |
| `<phase> +:` | inside a target or rule body | append steps to a phase (§5.3) |

Two statements `load`ing the same name, or two targets of the same name in one
file, are `duplicate-name`.

## 5. Values, fields, steps

### 5.1 Fields

The field names and their meanings are KILN00 §6.2's table and are not
repeated here; the language fixes their **value shape**:

| Field | Value shape |
|---|---|
| `inputs`, `test-inputs`, `reads`, `deps`, `test-deps`, `tools`, `outputs`, `provides` | a list, or one bare word as a one-element list |
| `env` | `NAME = "value"` pairs, comma-separated on one line or one per line |
| `pass` | a list of variable names |
| `state` | a list of names; `state: cargo-target shared-by toolchain profile` is four names |
| `cache` | `on` or `off` |
| `timeout` | a duration (§3.3) |

An unknown field is `field-unknown`, never ignored: a typo that silently did
nothing is the failure this repository has paid for most often (a rule that
does not fire, a check that does not run). A known field with the wrong shape
is `field-shape`.

### 5.2 Define and append

`field: value` **replaces** what the rule set. `field +: value` **appends**:
lists concatenate, `env` merges (a repeated name replaces), and a phase block
appends steps. Append to a field that the rule did not set is not an error; it
is an append to the empty value. `+` is not allowed on `cache`, `timeout` or
`env`'s inner `=`, which have one value by construction: `append-scalar`.

When a rule `use`s another rule (KILN00 §6.2, "the innermost rule applies
first"), the effect is that the whole document is evaluated with the rules
applied from the outermost inward, each layer's `:` replacing and each `+`
appending to what the layer beneath produced.

### 5.3 Phases

Exactly three phase names exist: `fetch`, `build`, `test` (KILN00 §7.1).
Their semantics — which may use the network, which runs for a dependency — are
KILN00's; the language's part is:

- A phase block's body is a list of steps (§5.4).
- `fetch` accepts two settings no other phase does: `network: on` and
  `pinned-by: <path>`. Either in another phase is `phase-setting`.
- A target with no `build` block and no `build` steps from its rule has an
  empty `build` action. That is legal: a package with nothing to compile
  still has a key, and a `test` may depend on it.
- A phase whose steps are all comments is **empty**, and an empty action is
  vacuously complete (KILN04 §7.1's treatment of an empty recipe).

### 5.4 Steps

One step per line inside a phase:

| Form | Meaning |
|---|---|
| `<tool> <args…>` | run a declared tool; words are bare or quoted |
| `in <dir>: <tool> <args…>` | the same, working in `<dir>` inside the work root |
| `copy <from> <to>` | built-in copy |
| `mkdir <dir>` | built-in |
| `remove <path>` | built-in |
| `write <path> "<text>"` | built-in |
| `( … )` | a step that spans lines; the body is one step |
| `shell: """…"""` | an explicitly non-portable escape hatch, counted by `kiln lint` |

A step runs **no shell**. Pipes, `&&`, `;`, redirection, globbing and `$VAR`
are not step syntax; a step containing one of them is `step-shell-syntax`,
and the diagnostic names the character, because the fix is always to use two
steps, `in`, or the built-in that replaces the shell idiom.

`in <dir>:` requires `<dir>` to be inside the work root: a leading `/`, a
component `..`, or a Windows drive or UNC prefix is `step-dir-escape`. The
built-ins' paths obey the same rule. A tool named in a step but absent from
`tools` is `tool-undeclared`; a built-in name is never also a tool name
(`builtin-shadow`).

### 5.5 Variables and loops

`let name = value` binds an immutable string, list, or list with a `for`
comprehension:

```
let targets = (cargo_archive_path array_ref_tracing beam_array_len)
    cargo test -p lang-aot --lib [for t in targets: --test $t]
```

- A value is a string, a list, or `[for v in <list>: <words…>]`. A
  comprehension's body is a word sequence that may interpolate `$v`; the whole
  comprehension expands to the concatenation of the bodies, in order.
- A list in a word position expands to one word per element. `(a b)` in
  argument position is therefore two arguments, not one.
- Rebinding a name in the same scope is `let-rebind`; there is no assignment.
- An unbound `$name` is `interp-unbound`. There is no implicit variable, with
  one exception: a rule's parameters are bound (below), and `$package` is
  bound in every BUILD to the package's repository-relative path.
- Scoping is lexical and file-local. A `let` at the top of a BUILD is visible
  to every target in it; a `let` inside a target or rule body is visible only
  there and in nested blocks.
- For a rule, `rule name(a, b = <default>)` binds parameters; a target's
  `name=value` arguments bind them by name. A missing argument with no
  default is `arg-missing`; an unknown argument name is `arg-unknown`.

**Not yet specified: `fn`, call syntax and `if`.** KILN00 §6.1 allows
functions in rule files (their call graph must be acyclic) and §6.6's closing
note allows an `if` inside a phase "so a conditional flag does not duplicate a
whole phase". This draft does not define either, and it does not pretend to:
they are open questions 7 and 8 below. Until they are settled, tier 1 and tier
2 are as written here, and a `fn` or an `if` is `construct-unknown` rather
than silently ignored.

**Termination.** The language has no recursion and no unbounded iteration: a
comprehension iterates a literal or a `let`-bound list, never a computed one,
and rule `use` graphs must be acyclic. The planner checks the graph before
evaluating any file (KILN00 §6.1 item 1) and refuses a cycle with
`rule-cycle`, naming the cycle.

## 6. Expansion

Expansion turns (BUILD + its rule, in force) into **canonical actions**.

### 6.1 What is fixed before anything runs

Evaluating any Kiln file must finish, be deterministic, and settle every input
pattern, output, tool and step before any step runs (KILN00 §6.1). In this
language that means an expression may not read a file, a clock, an
environment variable, the network, or the result of a glob. A construct that
would is not syntax: it is not expressible. Real computation belongs in a
**tool**, run as a step or a scanner, and hashed like any other input.

### 6.2 The canonical action

For a target `T` with phases `fetch`, `build`, `test`, the expansion produces
at most three actions. One action is:

```
target      T
phase       fetch | build | test
recipe      the phase's steps, in order, each as (tool, argv…)
inputs      sorted (path, kind, mode, sha256)   -- per KILN00 §8 item 6
reads       sorted (path, …)                    -- item 7
tools       sorted (name, pinned version, identity)
env         sorted NAME=value
pass        sorted name (value hashed, never stored)
state       the declared names
timeout     a duration or absent
deps        the (target, phase) pairs this action consumes
```

The inputs, reads, tools and deps follow KILN00 §8 items 6–10 exactly; this
spec adds nothing to them.

### 6.3 The canonical form

Two expansions are equal iff their canonical encodings are byte-equal:

- The recipe is the **ordered** list of `(tool, argv…)`. Argument order and
  step order are meaningful (KILN00 §8 item 5) and are never sorted.
- Every other list is sorted by its first component, and a duplicate in any
  of them is `dup-entry`.
- Paths are repository-relative, `/`-separated, with no `.` or `..` component
  and no trailing `/` outside a glob.
- Variadic interpolation has already happened: there is no bracket, no `$`,
  no `[for` and no whitespace-token boundary left in the canonical form.
- The encoding of this canonical form into key bytes is KILN02's job. The
  language's obligation is only that two files with the same meaning produce
  the same structure here.

### 6.4 Globs are the planner's, not the language's

`src/**`, `*.tex` and `**/*.log` are **data**. The lexer sees a word; the
planner resolves it through `Source` (KILN00 §5 item 1) after evaluation, and
the resolved `(path, …)` tuples are the action's inputs. `*` stays inside a
component and `**` as a whole component spans directories — the same language
as `.kilnignore`, pinned by `code/specs/fixtures/kiln/glob-cases-v1.json`.
A glob that resolves to nothing is legal and means an empty input set; whether
that is *useful* is the rule's problem, not the language's.

## 7. Checks and diagnostics

`kiln lint` and the planner apply these; each is a stable token with no path
text, and a file may carry several.

| Token | Condition |
|---|---|
| `marker-missing` / `marker-version` | no marker, or a version this Kiln does not implement |
| `tab-indent` | a tab in leading whitespace (§3.2) |
| `indent-unexpected` / `indent-inconsistent` | §3.2 |
| `bom` / `nul` / `cr` / `not-utf8` | §3.1 |
| `field-unknown` / `field-shape` / `append-scalar` | §5.1, §5.2 |
| `phase-unknown` / `phase-setting` | §5.3 |
| `step-shell-syntax` / `step-dir-escape` / `tool-undeclared` / `builtin-shadow` | §5.4 |
| `let-rebind` / `interp-unbound` / `interp-invalid` | §3.4, §5.5 |
| `arg-missing` / `arg-unknown` / `rule-cycle` | §5.5 |
| `duplicate-name` / `dup-entry` | §4, §6.3 |
| `construct-unknown` | a `fn` or an `if`, which this version does not define (§5.5, §9) |
| `test-runner-in-build` | a `build` step runs a test runner (KILN00 §7.1; the list is data in `code/kiln/lint.toml`, which no PR has written yet, exactly as in KILN00 §7.1) |
| `shell-escape` | a `shell:` step exists; reported, never fatal, and the count is a migration metric (KILN00 §6.2) |

`marker-missing` on a path named `BUILD` means the file is a **legacy** BUILD:
not an error, and not Kiln's to evaluate.

## 8. Conformance

KILN00 §16 defines the surface: two pure functions and golden vectors. As
functions of this language:

1. **Parse and expand:** `(BUILD bytes, rule bytes for each load, Source tree
   for glob resolution) → canonical actions`.
2. **Key:** `(canonical actions, file digests, tool identities) → 32 bytes`
   — already KILN02's `key.Key`, which is what makes a second implementation
   of this spec an independent check of the key algorithm.

The vectors live in `code/specs/fixtures/kiln/lang-cases-v1.json`, one case
per row of §3–§7, each with the input bytes, the expected canonical actions (or
the exact diagnostic token), and a SHA-256 of the canonical encoding. A case
that a port cannot reproduce is a bug in the port or a gap in this spec, and
which one is decided by reading the case, never by loosening it.

`key-vectors-v1.json` (KILN02) stays the authority for the *encoding*; this
spec's vectors are about the *structure* that encoding is applied to.

## 9. Open questions

1. **Rule versioning** (§2): accept `kiln-rule 1`, or version rules some other
   way? KILN00's examples have no marker.
2. **Indent width** (§3.2): this spec allows any consistent width. Should it
   instead fix one (two, or four) and refuse the others, as Go's formatter
   effectively did?
3. **`load` path form** (§4): the sketch shows
   `load("code/packages/starlark/library-rules/rust_library.star",
   "rust_library")` for the *legacy* Starlark rules, and Kiln's own rule path
   is `code/kiln/rules/<name>.rule`. Should Kiln's `load` take a rule name
   only (`load("rust_crate")`) and resolve the path itself, so a BUILD cannot
   name an arbitrary file?
4. **Rule parameters vs target fields** (§5.5): KILN00 uses both
   (`rust_crate(name = $package)` with `$package`, and
   `python_library(extra = "dev")`). Is `$package` a binding, a parameter with
   a default, or both?
5. **`provides` vs `outputs` for a consumer** (§6.2): KILN00 §7.2 defaults
   `provides` to build inputs plus outputs; the staging digests and the
   "verified when staged" rule are KILN00's, but the exact set a consumer
   receives is worth one worked example before M4.
6. **`fn` bodies, call syntax and the expression grammar** (KILN00 §6.6):
   KILN00 allows functions in rule files and requires their call graph to be
   acyclic. What may a function body contain — only the statements of §4, or
   expressions? Is a call `f(a, b)` or `$f(a, b)`? This draft defines no
   function, so no port can disagree yet.
7. **An `if` inside a phase** (KILN00 §6.6): "allowed above, so a conditional
   flag does not duplicate a whole phase". The condition's language (equality
   on strings and lists? membership?) and the diagnostic for a non-boolean are
   undecided. This draft refuses it as `construct-unknown`.
8. **A `-:` removal operator** (KILN00 §6.6): is removing an element a
   `field -: value`, and what does it mean for a phase's steps (drop a step by
   index?) or for parameters? This draft defines only `:` and `+:`.
9. **Error recovery**: does the parser report every diagnostic in a file, or
   stop at the first? Reporting all is more useful and more work; the choice
   affects the golden vectors.
