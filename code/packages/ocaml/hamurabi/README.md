# coding-adventures-hamurabi (OCaml)

Pure, deterministic simulation core for the seminal 1968 mainframe resource allocation game (**GAME01** / **HA01**).

## Historical Lineage

- **1964 — The Sumerian Game:** Designed by Mabel Addis (the first credited female video game designer and writer) and programmed by William McKay for an IBM 7090 mainframe at BOCES in Westchester County, New York.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the DEC PDP-8 in FOCAL-69.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published by DEC.

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER27`, `VER29`):
- **Zero ambient authority:** `"capabilities": []` declared in `required_capabilities.json`.
- **Pure state transitions:** `step_year : hamurabi_state -> turn_order -> (hamurabi_state, hamurabi_error) result`.
- **Explicit pseudorandomness:** All simulation rolls enter deterministically via explicit 64-bit PCG32 state embedded in `hamurabi_state` (`rng_state`, `rng_inc = 109L`).
- **Bit-exact integer arithmetic:** Handled with native 63-bit integers and unsigned 64-bit `Int64` with masking (`0xFFFFFFFFL`), completely avoiding signed overflow UB and float conversion inaccuracy.
- **Zero third-party dependencies:** Conformance tests read and validate against `code/specs/fixtures/hamurabi-v1/cases.txt` using solely stdlib (`Scanf`, `Printf`, `Buffer`) and `alcotest`.

## Usage

```ocaml
open Coding_adventures_hamurabi

let state0 = new_game 42L

let order = { acres_delta = 0; bushels_to_feed = 2000; acres_to_plant = 1000 }

let () =
  match step_year state0 order with
  | Error _ -> print_endline "Order rejected"
  | Ok state1 -> print_string (format_report state1)
```
