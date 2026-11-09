# ct-compare (OCaml)

Constant-time byte comparison and selection: the OCaml lane of
[CT01](../../../specs/CT01-constant-time-compare.md).

A secret comparison that stops at the first differing byte tells an attacker how
long the matching prefix was. This package reads every byte regardless, folds
the differences into one accumulator and inspects it once at the end.

## Installation

`opam install coding-adventures-ct-compare`, or `opam install .` from this
directory.

## Usage

```ocaml
open Coding_adventures_ct_compare

let () =
  assert (ct_eq "tag" "tag");
  (* The selection bit is an int, produced without converting a bool. *)
  match ct_select_bytes "accept" "reject" (ct_eq_bit "tag" "tag") with
  | Ok picked -> print_endline picked
  | Error Select_length_mismatch -> prerr_endline "lengths differ"
  | Error Invalid_choice -> prerr_endline "bit must be 0 or 1"
```

## API

| Function | Result |
| --- | --- |
| `ct_eq : string -> string -> bool` | byte equality |
| `ct_eq_fixed : string -> string -> bool` | same operation (OCaml has no fixed-size byte type here) |
| `ct_select_bytes : string -> string -> int -> (string, error) result` | `left` when the bit is 1, `right` when 0 |
| `ct_eq_u64 : int64 -> int64 -> bool` | equality of two 64-bit patterns |
| `ct_eq_bit`, `ct_eq_fixed_bit : string -> string -> int` | `1` or `0` |
| `ct_eq_u64_bit : int64 -> int64 -> int` | `1` or `0` |

* `type error = Select_length_mismatch | Invalid_choice`. `Select_length_mismatch`
  is CT01's `CT_SELECT_LENGTH_MISMATCH`; both errors are decided before any
  content is read, and the length check comes first.
* A length mismatch makes `ct_eq` false (`ct_eq_bit` returns 0) without reading
  either input: lengths are public.
* The selection bit is an `int` that must be exactly `0` or `1`, validated by
  `bit land (lnot 1) <> 0` (one computation, identical for both valid values).
  OCaml has no branch-free bool-to-int conversion, so use the `_bit` variants
  rather than converting a boolean.
* The returned string is freshly allocated (it is never one of the inputs).
* `int64` carries all 64 bits: `-1L` is `0xFFFFFFFFFFFFFFFF`.

## What this does and does not guarantee

The source has no content-dependent early return, branch, index or loop bound.
The byte-equality bit is `(acc - 1) lsr 62` (native `int` is 63 bits wide), the
u64 bit is `((d lor neg d) lsr 63) lxor 1`, and selection uses
`r lxor ((l lxor r) land mask)`.

It does **not** guarantee wall-clock constant time:

* `ocamlopt` is an optimising compiler and may rewrite the loops; nothing in
  OCaml lets the source pin or inspect the generated machine code, and this
  package does not claim the assembly was reviewed for every compiler version.
  Under `ocamlc`/`ocaml` every instruction is dispatched through the bytecode
  interpreter instead.
* The runtime is garbage collected: the minor heap and compactor copy live
  data, so secret bytes may be duplicated and left behind in freed memory. The
  package cannot zero them.
* `String.get` bounds checks branch on the public index, never on data.
* Cache and speculation behaviour of the host are outside the model.

Treat it as protection against *algorithmic* timing leaks only.

## Tests

```bash
dune runtest --force
```

The Alcotest suite replays all 21 cases of the shared CT01 corpus and adds
every single-byte mismatch position, every single-bit difference of every byte
value, every accumulator value 1..255, all 256x256 byte pairs through selection
under both bits, length mismatches, invalid choices (including negatives,
`min_int` and `max_int`) and all 64 single-bit flips of an `int64` from five
bases.

Dune's workspace root is this package, so the canonical corpus outside it
cannot be a dune dependency. `test/fixtures/ct-compare-v1/cases.json` is a
copy, and `vendored copy is current` fails the build whenever the canonical
`code/specs/fixtures/ct-compare-v1/cases.json` is reachable and differs. Refresh
the copy with `cp` after any corpus change.

Coverage tooling (`bisect_ppx`) and `ocamlformat` are required by `BUILD` in CI;
neither was available where this port was written, so coverage was not measured
there (every function and branch of the 60-line implementation is exercised) and
the sources were formatted by hand.

## Where it fits

A leaf utility with no dependencies and no capabilities (see
`required_capabilities.json`).
