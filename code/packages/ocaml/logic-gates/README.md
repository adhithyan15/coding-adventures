# logic-gates

An OCaml implementation of the portable logic-gates contract. It provides
validated primitive and NAND-derived gates, n-ary gates, multiplexers,
encoders, decoders, tri-state output, latches, edge-triggered flip-flops,
registers, shift registers, and wrapping counters.

Selectors and register values are least-significant-bit first. Stateful
operations return the next explicit state instead of retaining hidden global
state, which keeps simulations deterministic and easy to test. Decoder width
is capped at 16 bits so oversized output allocation fails deterministically
before memory is reserved.

## Installation

Install a released package with `opam install coding-adventures-logic-gates`.
From this source directory, use `opam install .`.

## Usage

```ocaml
open Coding_adventures_logic_gates

let () =
  match Basic.and_gate 1 0 with
  | Ok bit -> Printf.printf "1 AND 0 = %d\n" bit
  | Error (Invalid_bit { name; value }) ->
      Printf.eprintf "invalid %s bit: %d\n" name value
  | Error _ -> prerr_endline "invalid gate input"
```

Compile and run the example with:

```bash
opam exec -- ocamlfind ocamlc -linkpkg -package coding-adventures-logic-gates example.ml -o example.bc
opam exec -- ocamlrun example.bc
```

## Input validation

Every gate (`not_gate`, `and_gate`, `or_gate`, `xor_gate`, `nand_gate`,
`nor_gate`, `xnor_gate`, `nand_not`, `nand_and`, `nand_or`, `nand_xor`)
validates each of its own arguments and returns
`Error (Invalid_bit { name; value })` for the first argument that is not
exactly `0` or `1`. `name` is the gate's own parameter name (`"a"` or `"b"`),
even for the NAND-built gates, so `nand_or 1 2` blames `"b"`. When both
arguments are invalid, `"a"` is reported.

`bit` is `int`; OCaml has no implicit numeric conversions, so
`and_gate 1.0 0`, `and_gate true 0` and `and_gate '1' 0` are rejected at
compile time rather than at run time.

## Development

```bash
# Run tests
bash -e BUILD
```

The build runs ocamlformat checks, Alcotest, and verifies that bisect_ppx
measures the production source. The package's current measured line coverage
is above 95%; numeric threshold enforcement belongs to the cross-platform
package CI contract.
