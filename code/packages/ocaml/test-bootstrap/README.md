# test-bootstrap (OCaml)

The OCaml lane of the Level 2 property-based testing engine
([VER05q](../../../specs/VER05q-ocaml-pbt-engine.md), a child of
[VER05](../../../specs/VER05-property-engine.md)). SplitMix64 generators, integrated shrinking,
a hierarchical shrink budget (VER05e section 4.1), honest verdicts with CI redaction, and the
runner for the shared `pbt-v1` conformance corpus (25 cases, exact outputs, minimal witnesses
and shrink step counts).

**Zero runtime dependencies beyond the OCaml 5 standard library.** Test and dev dependencies are
exactly those of `ocaml/ct-compare` (`alcotest`, `bisect_ppx`, `ocamlformat`).

## Where it fits

```text
warrants (VER04) -> generators -> this engine -> verdict (property-counterexample, ...)
                                   |
                                   +-- pbt-v1 corpus (code/specs/fixtures/pbt-v1) pins every lane
```

## Usage

```ocaml
open Coding_adventures_test_bootstrap

let () =
  let gen = Gen.list ~max_len:20 (Gen.int 0 1000) in
  let sum l = List.fold_left ( + ) 0 l in
  let opts = { Types.default_options with ci = Types.Ci_off } in
  let v = Runner.check ~seed:42L gen opts (fun l -> sum l <= 1000) in
  match v.failure_id with
  | None -> print_endline "held"
  | Some id -> Printf.printf "%s: %s\n" id (Option.value v.message ~default:"")
```

A property is `'a -> bool` and holds iff it returns exactly `true`. Every generator carries a
`canon` (an explicit `Val.t` description), so the engine renders and measures values without
ever applying polymorphic compare, equality or `Obj` to your data. `Gen.map ~canon` and
`Gen.custom ~canon` take your own.

## OCaml specifics (see the spec for the reasoning)

- **Integers.** SplitMix64 runs on `int64` with the unsigned `compare`/`div`/`rem`; every
  generated or fixture integer is a hand-written `Bigint.t` (no Zarith). Fixture digits are
  capped at 1,000.
- **Exceptions.** Classified by constructor name only (`Printexc.exn_slot_name`); `to_string` is
  never called, so payloads and registered printers never run. `Sys.Break` and `Out_of_memory`
  are never swallowed; `Stack_overflow` and `Effect.Unhandled` are `property-panic`.
- **Domains.** The budget scope is ambient in `Domain.DLS` (inherited by spawned domains) and can
  be overridden with `?parent`. Counters are shared under one mutex per tree.
- **Scope.** The ambient scope is per domain, not per thread or fibre: concurrent checks in
  different threads or fibres of one domain should pass `?parent`; without it they may be charged to
  another check's tree and get `property-exhausted` if it closes first (fail-closed, never a pass). A
  domain that outlives its scope holds a spent handle: a plain `check` there is `property-exhausted`. Generation is length-bounded per
  level, not in total (nested lists multiply); shrinking shares one budget.
- **Redaction.** The redacted `repro-fnv` is a non-secret fingerprint (an unkeyed hash), not a redaction
  guarantee for small witness spaces.
- **CI.** `Ci_unset` (the default) redacts like CI. Read the environment yourself with
  `Types.ci_of_env (Sys.getenv_opt "CI")`.

## Tests and build

`BUILD` mirrors `ocaml/ct-compare`: `opam install`, a `cmp` that checks `test/corpus_data.ml`
against `code/specs/fixtures/pbt-v1/cases.json`, `dune build @fmt`, `dune runtest` under
`bisect_ppx`, and `bisect-ppx-report summary --expect`. After editing the corpus, regenerate the
embedded module with

```sh
{ printf 'let json =\n  {pbt|'; cat ../../../specs/fixtures/pbt-v1/cases.json; printf '|pbt}\n'; } > test/corpus_data.ml
```
