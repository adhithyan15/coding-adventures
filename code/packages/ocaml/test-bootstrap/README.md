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

## Fuzz engine (VER06r)

The Level 2 coverage-guided fuzz engine ([VER06r](../../../specs/VER06r-ocaml-fuzz-engine.md), a
child of [VER06](../../../specs/VER06-coverage-guided-fuzz-engine.md)): the `Fuzz_*` modules of this
package. It conforms to the shared `fuzz-v1` corpus (25 cases) and replays the Go lane's 820
oracle vectors. Both fixtures are embedded as generated modules (`test/fuzz_corpus_data.ml`,
`test/go_vectors_data.ml`) that `BUILD` checks byte for byte.

```ocaml
open Coding_adventures_test_bootstrap

let () =
  let target data h =
    Fuzz_handle.edge h (Fuzz_handle.edge_id "entered");
    Fuzz_handle.tick h 1;
    if data = "\xff\xff" then failwith "boom"
  in
  let opts = { Fuzz.default_options with max_iterations = 500; seed = 7L; ci = Types.Ci_off } in
  let report = Fuzz.run [ "\000\000" ] opts target in
  let v = Fuzz.verdict_for target report opts "my_target" in
  List.iter print_endline v.lines (* not ok 1 - my_target [fuzz-crash:xxxxxxxx] *)
```

- **Coverage** is what the target reports through `Fuzz_handle.edge`; bisect_ppx is not used in the
  hot path (process-global counters, no per-execution read, needs an instrumented build).
- **Isolation.** Every execution runs in a fresh domain with a fresh, abstract, retired-after-use
  handle. OCaml cannot interrupt another domain: a target is bounded only by the cooperative
  `tick`/`reserve` and the wall-clock deadline (`Unix.gettimeofday`; the waiter sleeps, it does not spin). **A tight loop that never calls the handle cannot be
  stopped in-process**; its domain is leaked and counted, the campaign aborts after 4 leaks (on every
  path: mutation, minimisation, re-verification; 16 leaked workers per process is a typed
  `Error.Engine_error`), and the CI job timeout is the backstop (the
  out-of-process runner is PT-F14).
- **Memory** is a best-effort bound: `Gc.counters` deltas (bytes allocated, not residency) polled at
  `tick`/`reserve` (every 64th call, so the bookkeeping is not charged to the target) and once after
  the run, plus `reserve`d bytes. Default 256 MiB; `mem_budget` below 64 KiB is refused.
- **Verdicts are honest.** Zero executions is never a pass; crash witnesses are re-run; CI and unset
  modes carry no seed and no witness; the only public text is `not ok N - <target token>
  [<failure-id>:<hex8>]` with the VER06 9.3 byte grammar.
- `required_capabilities.json` declares `thread:spawn`, `time:read` and `time:sleep`. The library needs `unix`, which ships with the compiler.
