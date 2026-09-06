# state-machine

Bounded, deterministic DFA, NFA, PDA, and modal state machines for the OCaml
implementation lane. Every mutable runtime has an immutable definition,
typed construction and processing errors, deterministic snapshots, persistent
trace records, and explicit resource ceilings.

The public API uses string states and events. NFA and PDA epsilon transitions
use None; Some "" remains an ordinary named event. DFA reachability and
modal-mode structure exercise the direct directed-graph dependency.

## Highlights

- DFA actions receive exact source, event, and target values. Acceptance is
  non-mutating, while sequence processing validates the complete request before
  running actions or changing state.
- NFA execution performs iterative epsilon closure and bounded subset
  construction. Trace entry and state-cell limits are checked before mutation.
- DFA minimization removes unreachable states and merges language-equivalent
  partitions without changing the source machine.
- PDA stacks are exposed bottom-to-top; the last pushed symbol becomes the new
  top. Named processing never silently consumes epsilon transitions.
- Modal machines switch only through switch_mode. Entering a mode resets its
  contained DFA before activation.

## Dependencies

- coding-adventures-directed-graph 0.1.0

## Installation

Install a released package with `opam install coding-adventures-state-machine`.
From this source directory, pin the local dependency chain leaf-first and
install the package:

```bash
opam pin add --no-action --working-dir --no-checksums -y coding-adventures-graph ../graph
opam pin add --no-action --working-dir --no-checksums -y coding-adventures-directed-graph ../directed-graph
opam install .
```

## Usage

```ocaml
open Coding_adventures_state_machine

let () =
  let transitions =
    [ { source = "locked"; event = "coin"; target = "unlocked" } ]
  in
  match
    Dfa.create ~states:[ "locked"; "unlocked" ] ~alphabet:[ "coin" ]
      ~transitions ~initial:"locked" ~accepting:[ "unlocked" ] ()
  with
  | Error _ -> prerr_endline "invalid machine definition"
  | Ok machine -> (
      match Dfa.process machine "coin" with
      | Ok state -> Printf.printf "current state: %s\n" state
      | Error (Unknown_event event) ->
          Printf.eprintf "unknown event: %s\n" event
      | Error _ -> prerr_endline "transition failed")
```

Compile and run the example with:

```bash
opam exec -- ocamlfind ocamlc -linkpkg -package coding-adventures-state-machine example.ml -o example.bc
opam exec -- ocamlrun example.bc
```

## Development

The build recipe pins the complete leaf-first local closure, installs exact
development dependencies, checks formatting, runs Alcotest with bisect_ppx,
and fails if the production source is absent from measured coverage. Numeric
95% threshold enforcement belongs to the cross-platform package CI contract.

    bash -e BUILD

The package targets OCaml 5.2.1, Dune 3.17.2, Alcotest 1.9.0,
bisect_ppx 2.8.3, and ocamlformat 0.27.0.
