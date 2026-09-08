# OCaml capability analyzer

`coding-adventures-capability-analyzer` checks one OCaml package against the
capability-security contract in [Spec 13](../../../specs/13-capability-security.md)
and [OCAML06](../../../specs/OCAML06-capability-analyzer.md).

It parses `.ml` and `.mli` files with the OCaml 5.2.1 compiler AST, reports
direct filesystem, network, process, environment, FFI, clock, and standard-
stream access, and compares those findings with `required_capabilities.json`.
It also blocks unsafe constructs such as `Obj.magic`, unchecked Marshal
deserialization, closure marshaling, native Dynlink loading, and `external`
declarations.

The analyzer works on syntax, not strings. Module aliases, supported `open`
forms, lexical value shadowing, interfaces, and nested source directories are
handled explicitly. A malformed source file or manifest is a tool error rather
than a warning because silently skipping input could create a false-clean
result.

## Command line

From this package directory:

```bash
opam install . --deps-only --with-test --with-dev-setup --yes --require-checksums
opam exec -- dune exec coding-adventures-capability-analyzer -- --dir ../logic-gates
```

Use `--verbose` to print every detection even when the manifest covers it:

```bash
opam exec -- dune exec coding-adventures-capability-analyzer -- \
  --dir . --verbose
```

Exit status is `0` for a clean package, `1` for capability or banned-construct
violations, and `2` for an input, parse, manifest, traversal, or internal error.

Example violation:

```text
src/main.ml:12:4: [CAP001] undeclared capability: env:read:* (Sys.getenv call) — add it to required_capabilities.json
```

## Policy model

Detections are action-level wildcards such as `fs:read:*`. Any declaration
with the same category and action covers the static finding; a runtime cage is
responsible for enforcing its narrower target. Cross-category or cross-action
declarations never match.

`Obj` and unsafe Marshal use are hard failures. An `external` declaration is
allowed only when the manifest contains both an exact OCaml exception for
`external` and an `ffi:call` capability. `Dynlink.loadfile` and
`Dynlink.loadfile_private` similarly require an exact exception plus
`ffi:load`. Those declarations remain security-sensitive review events; the
analyzer does not approve them.

## Library

The `Coding_adventures_capability_analyzer` module exposes pure source and
manifest parsers as well as directory analysis:

```ocaml
open Coding_adventures_capability_analyzer

match analyze_directory "code/packages/ocaml/logic-gates" with
| Error message -> prerr_endline message
| Ok result when passed result -> print_endline "clean"
| Ok result -> List.iter (fun violation -> print_endline (format_violation violation)) result.violations
```

`analyze_source` is useful for editors and fixture tests because it accepts
source text directly and performs no filesystem access. `evaluate` applies one
already parsed manifest to collected detections and banned constructs.

## Tests

The JSON behavior corpus under
`code/specs/fixtures/ocaml-capability-analyzer-v1/` is the executable contract;
an equality-checked mirror under `test/fixtures/` keeps source archives
self-contained. The package tests also cover strict manifest loading, identity
and regular-file checks, fixed traversal and input ceilings, the closed 19-pair
taxonomy, declaration matching, dual FFI opt-in, hard bans, deterministic
recursive discovery, first-class sensitive references, constrained includes,
CLI output, and fail-closed parse, generated-source, attribute, and extension
handling.

```bash
opam exec -- dune build @fmt
BISECT_FILE="$PWD/bisect" opam exec -- \
  dune runtest --force --instrument-with bisect_ppx
opam exec -- bisect-ppx-report summary --per-file \
  --expect src/coding_adventures_capability_analyzer.ml bisect*.coverage \
  | tee _build/coverage-summary.txt
python test/check_coverage.py --summary _build/coverage-summary.txt \
  --source src/coding_adventures_capability_analyzer.ml --minimum 95
```

The BUILD contracts require at least 95% measured production coverage and an
installable release build.
