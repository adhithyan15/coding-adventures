(* ===========================================================================
   Constant-time comparison and selection (CT01), OCaml lane
   ===========================================================================

   Why this exists
   ---------------
   A secret comparison that stops at the first differing byte tells an attacker
   how long the matching prefix was.  Controlling one operand (a MAC tag, a
   session token) they can recover the other a byte at a time by timing guesses.
   The defence is to read EVERY byte whatever the data is, fold the differences
   into one accumulator, and look at the accumulator once at the very end:

       a   = 41 42 43 44          a   = 41 42 43 44
       b   = 41 42 43 44          b   = 41 FF 43 44
       xor = 00 00 00 00          xor = 00 BD 00 00   (all four bytes were read)
       acc = 0x00 -> equal        acc = 0xBD          -> different

   What this module promises and what it does not
   ----------------------------------------------
   The SOURCE has no content-dependent early return, branch, index or loop
   bound; the loops run to the public length.  It does NOT promise wall-clock
   constant time: ocamlopt may rewrite the loops, the garbage collector can run
   at any allocation and copies live data (leaving secret bytes behind), and
   nothing in OCaml lets the source pin machine code or inspect it.  Bounds
   checks on [String.get] are branches on the PUBLIC index, never on data.

   The selection bit
   -----------------
   OCaml has no branch-free way to turn a bool into a number ([if b then 1
   else 0] is a branch on the secret), so the bit is an [int] that must be
   exactly 0 (pick right) or 1 (pick left), and each equality has a
   bit-returning variant so that no caller converts a boolean:

       ct_select_bytes a b (ct_eq_bit x y)

   Native [int] is 63 bits wide, so the byte-equality bit uses a 62-bit shift;
   unsigned 64-bit values use [Int64] so all 64 bits survive.
   =========================================================================== *)

type error = Select_length_mismatch | Invalid_choice

(* OR together the XOR of every byte pair.  Both strings have the same length.
   The loop bound [n] is the public length. *)
let difference left right =
  let acc = ref 0 in
  for i = 0 to String.length left - 1 do
    acc :=
      !acc
      lor (Char.code (String.get left i) lxor Char.code (String.get right i))
  done;
  !acc

(* For acc in 0..255, [acc - 1] is -1 (all 63 bits set) exactly when acc = 0 and
   is 0..254 (top bit clear) otherwise.  A LOGICAL shift right by 62 leaves the
   one bit that distinguishes them: 1 for "equal", 0 for "different".  No
   comparison, no conditional. *)
let ct_eq_bit left right =
  if String.length left <> String.length right then 0
  else (difference left right - 1) lsr 62

let ct_eq_fixed_bit = ct_eq_bit
let ct_eq left right = ct_eq_bit left right = 1
let ct_eq_fixed = ct_eq

(* mask = 0xFF for bit 1, 0x00 for bit 0, so
     out = right xor ((left xor right) land mask)
   is [left] or [right] by the same instructions either way.  The validity
   test [bit land (lnot 1) <> 0] is one computation with the same answer for
   both valid values (and it also rejects negatives), so a valid call does
   identical work whichever bit is secret. *)
let ct_select_bytes left right bit =
  if String.length left <> String.length right then Error Select_length_mismatch
  else if bit land lnot 1 <> 0 then Error Invalid_choice
  else
    let mask = -bit land 0xFF in
    let out = Bytes.create (String.length left) in
    for i = 0 to String.length left - 1 do
      let l = Char.code (String.get left i)
      and r = Char.code (String.get right i) in
      Bytes.set out i (Char.chr (r lxor (l lxor r land mask)))
    done;
    Ok (Bytes.unsafe_to_string out)

(* d = x xor y is zero exactly when the values are equal.  (d lor -d) has its
   top bit set exactly when d <> 0 (for a non-zero d either d or its two's
   complement is negative), so shifting that bit down logically and flipping it
   gives the equality bit.  One fixed operation sequence folds all 64 bits. *)
let ct_eq_u64_bit x y =
  let d = Int64.logxor x y in
  let folded = Int64.shift_right_logical (Int64.logor d (Int64.neg d)) 63 in
  Int64.to_int (Int64.logxor folded 1L)

let ct_eq_u64 x y = ct_eq_u64_bit x y = 1
