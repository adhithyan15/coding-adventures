(* The mutator algebra (VER06 section 3, VER06r section 2).

   Every mutator takes an immutable [string] and returns a NEW one, so "never
   modifies its argument" is free.  OCaml facts that shape the code:

   - offsets, counts, deltas and values arrive as [Bigint.t] (the Go oracle
     feeds 30-digit values, a hostile fixture a 1,000-digit one) and are reduced
     with [Bigint.rem_pos] against a NATIVE modulus before they index anything.
     [rem_pos] is a floored remainder, the wrap every other lane uses.
   - A flip never builds a big integer from the input bytes (the Elixir lane
     read the whole buffer as one integer and crashed on 2^(8 x 1 MiB)): a flip
     is a byte loop, and arithmetic works on at most 8 bytes in an [int64]. *)

type endian = Little | Big

let parse_endian = function
  | "little" -> Some Little
  | "big" -> Some Big
  | _ -> None

let interest8 = [ 0L; 1L; 127L; 128L; 255L ]
let interest16 = [ 0L; 1L; 255L; 256L; 32767L; 32768L; 65535L ]

let interest32 =
  [ 0L; 1L; 65535L; 65536L; 0x7FFFFFFFL; 0x80000000L; 0xFFFFFFFFL ]

(* The last three are unsigned words: 0x8000000000000000 and 0xFFFF... have the
   top bit set, so they are negative [int64]s that must be read unsigned. *)
let interest64 =
  [
    0L;
    1L;
    0xFFFFFFFFL;
    0x100000000L;
    0x7FFFFFFFFFFFFFFFL;
    0x8000000000000000L;
    0xFFFFFFFFFFFFFFFFL;
  ]

let interest_by_width =
  [ (1, interest8); (2, interest16); (4, interest32); (8, interest64) ]

let deltas = [ -35; -8; -1; 1; 8; 35 ]
let big = Bigint.of_int

(* [x mod m] for a native modulus m >= 1: the Bigint remainder is below m, so it
   always fits. *)
let mod_int x m =
  match Bigint.to_int_opt (Bigint.rem_pos x (big m)) with
  | Some i -> i
  | None -> 0

(* -------------------------------------------------------------------- *)
(* Flips                                                                 *)
(* -------------------------------------------------------------------- *)

(* Inverts bits [a, e) in place (a < e <= 8 * length).  Whole bytes are
   complemented at once. *)
let flip_bits b a e =
  let i = ref a in
  while !i < e do
    if !i land 7 = 0 && !i + 8 <= e then (
      let k = !i lsr 3 in
      Bytes.unsafe_set b k
        (Char.unsafe_chr (Char.code (Bytes.unsafe_get b k) lxor 0xFF));
      i := !i + 8)
    else (
      let k = !i lsr 3 in
      Bytes.unsafe_set b k
        (Char.unsafe_chr
           (Char.code (Bytes.unsafe_get b k) lxor (1 lsl (!i land 7))));
      incr i)
  done

(* Inverts [count] consecutive units (bits when unit = 1, bytes when unit = 8)
   from [offset mod units], wrapping around the buffer.  Only the PARITY of each
   unit's flips matters (XOR twice is a no-op): every unit is hit
   [count div units] times, and the first [count mod units] units from the
   wrapped start once more, so the work is O(len) for any count.

   Bit i is bit (i mod 8) of byte (i div 8), LSB first (VER06 3.1).

   Worked example, 2 bytes = 16 bits, [bit_flip 0 17]: 17 div 16 = 1 (every bit
   once) and 17 mod 16 = 1 (bit 0 once more, so twice, even): bits 1..15 end up
   inverted and bit 0 unchanged. *)
let parity_flip unit raw offset count =
  let size = String.length raw in
  if size = 0 || Bigint.sign count <= 0 then raw
  else
    let total = size * 8 in
    let units = total / unit in
    let start = mod_int offset units * unit in
    let q, r = Bigint.div_rem count (big units) in
    let full_odd = mod_int q 2 = 1 in
    let len = (match Bigint.to_int_opt r with Some i -> i | None -> 0) * unit in
    let b = Bytes.of_string raw in
    if full_odd then flip_bits b 0 total;
    if len > 0 then
      if start + len <= total then flip_bits b start (start + len)
      else (
        flip_bits b start total;
        flip_bits b 0 (start + len - total));
    Bytes.unsafe_to_string b

let bit_flip raw offset count = parity_flip 1 raw offset count
let byte_flip raw offset count = parity_flip 8 raw offset count

(* -------------------------------------------------------------------- *)
(* Arithmetic and boundary values                                        *)
(* -------------------------------------------------------------------- *)

(* The supported width of a Bigint argument, or 0 (the mutator is then the
   identity). *)
let width_of w =
  match Bigint.to_int_opt w with
  | Some (1 | 2 | 4 | 8 as k) -> k
  | _ -> 0

let width_ok w size = w <> 0 && size >= w

let mask_of w =
  if w = 8 then -1L else Int64.sub (Int64.shift_left 1L (8 * w)) 1L

let read_word endian raw at w =
  let acc = ref 0L in
  for i = 0 to w - 1 do
    let k = match endian with Little -> at + w - 1 - i | Big -> at + i in
    acc :=
      Int64.logor (Int64.shift_left !acc 8) (Int64.of_int (Char.code raw.[k]))
  done;
  !acc

let write_word endian b at w v =
  for i = 0 to w - 1 do
    let byte =
      Int64.to_int (Int64.logand (Int64.shift_right_logical v (8 * i)) 0xFFL)
    in
    let k = match endian with Little -> at + i | Big -> at + w - 1 - i in
    Bytes.unsafe_set b k (Char.unsafe_chr byte)
  done

(* Adds the signed [delta] modulo 2^(8 width) to the integer at the offset.  u16
   little-endian: [ff 00] is 0x00ff; +1 gives 0x0100, stored [00 01]. *)
let byte_arithmetic raw byte_offset delta width endian =
  let size = String.length raw in
  let w = width_of width in
  if not (width_ok w size) then raw
  else
    let at = mod_int byte_offset (size - w + 1) in
    let cur = read_word endian raw at w in
    let nv =
      Int64.logand (Int64.add cur (Bigint.to_int64_wrap delta)) (mask_of w)
    in
    let b = Bytes.of_string raw in
    write_word endian b at w nv;
    Bytes.unsafe_to_string b

(* Overwrites [width] bytes with [value mod 2^(8 width)]. *)
let boundary_insert raw byte_offset width value endian =
  let size = String.length raw in
  let w = width_of width in
  if not (width_ok w size) then raw
  else
    let at = mod_int byte_offset (size - w + 1) in
    let b = Bytes.of_string raw in
    write_word endian b at w (Int64.logand (Bigint.to_int64_wrap value) (mask_of w));
    Bytes.unsafe_to_string b

(* -------------------------------------------------------------------- *)
(* Blocks and splice                                                     *)
(* -------------------------------------------------------------------- *)

(* [min len avail] for a Bigint len and a native avail >= 0. *)
let clamp_len len avail =
  if Bigint.compare len (big avail) >= 0 then avail
  else match Bigint.to_int_opt len with Some i -> i | None -> avail

let block_delete raw offset len =
  let size = String.length raw in
  if size = 0 || Bigint.sign len <= 0 then raw
  else
    let at = mod_int offset size in
    let cut = clamp_len len (size - at) in
    String.sub raw 0 at ^ String.sub raw (at + cut) (size - at - cut)

(* Inserts at [offset mod (size + 1)]; an empty input becomes the insert. *)
let block_insert raw offset ins =
  let size = String.length raw in
  if size = 0 then ins
  else
    let at = mod_int offset (size + 1) in
    String.sub raw 0 at ^ ins ^ String.sub raw at (size - at)

(* Copies up to [len] bytes from [src mod size] to [dst mod (size + 1)] (the end
   is a valid insertion point). *)
let block_duplicate raw src_offset len dst_offset =
  let size = String.length raw in
  if size = 0 || Bigint.sign len <= 0 then raw
  else
    let src = mod_int src_offset size in
    let chunk = String.sub raw src (clamp_len len (size - src)) in
    let dst = mod_int dst_offset (size + 1) in
    String.sub raw 0 dst ^ chunk ^ String.sub raw dst (size - dst)

(* [target[0..ts) ^ donor[ds..]] with both splits clamped into their buffers. *)
let splice target donor target_split donor_split =
  let clamp size n =
    if Bigint.sign n <= 0 then 0
    else if Bigint.compare n (big size) >= 0 then size
    else match Bigint.to_int_opt n with Some i -> i | None -> size
  in
  let t = clamp (String.length target) target_split in
  let d = clamp (String.length donor) donor_split in
  String.sub target 0 t ^ String.sub donor d (String.length donor - d)

(* -------------------------------------------------------------------- *)
(* Sequences                                                             *)
(* -------------------------------------------------------------------- *)

type seq_result = Work_ok of string * int | Work_exhausted of int

exception Work_cap_exceeded of int * int

let nth_int64 l i = List.nth l i

(* [steps] seeded mutations, each chosen by one [bounded 7] draw:

     0 bit_flip, count 1            3 boundary_insert, width 1, interest8
     1 byte_flip, count 1           4 block_delete (len 1..n/2), skipped if n <= 1
     2 byte_arithmetic, deltas      5 block_insert of one random byte
                                    6 block_duplicate (len 1..n/2), skipped if n <= 1

   An empty buffer is refilled to [00] before every step.  The draw order is the
   cross-lane contract.  [max_len] is VER06 3.5's length budget: an insert or
   duplicate that would pass it is skipped AFTER its draws, so the stream stays
   aligned and memory stays bounded.

   Work is the bytes copied: the initial copy plus one buffer per step.  A call
   stops AT the step that would pass [cap] and reports what it had done (the
   caller still charges a failed call).  [steps] over [max_iterations] and inputs
   over [max_input_bytes] are an [Argument_error], raised before anything is
   copied.  Every sum is checked in the form [n > cap - work] (work <= cap
   always), so a 63-bit int cannot wrap. *)
let mutate_sequence_counted data seed steps max_len cap =
  if Bigint.compare steps (big Fuzz_limits.max_iterations) > 0 then
    raise
      (Error.Argument_error
         (Printf.sprintf "steps exceeds the maximum (%d)"
            Fuzz_limits.max_iterations));
  if String.length data > Fuzz_limits.max_input_bytes then
    raise
      (Error.Argument_error
         (Printf.sprintf "input exceeds %d bytes" Fuzz_limits.max_input_bytes));
  let work0 = String.length data in
  if work0 > cap then Work_exhausted 0
  else
    let rng = Prng.create (Bigint.to_int64_wrap seed) in
    let steps = match Bigint.to_int_opt steps with Some s -> max 0 s | None -> 0 in
    let step cur n =
      let b = Prng.bounded_int rng 7 in
      match b with
      | 0 ->
          let i = Prng.bounded_int rng (n * 8) in
          bit_flip cur (big i) Bigint.one
      | 1 ->
          let i = Prng.bounded_int rng n in
          byte_flip cur (big i) Bigint.one
      | 2 ->
          let di = Prng.bounded_int rng (List.length deltas) in
          let off = Prng.bounded_int rng n in
          byte_arithmetic cur (big off) (big (List.nth deltas di)) Bigint.one
            Little
      | 3 ->
          let vi = Prng.bounded_int rng (List.length interest8) in
          let off = Prng.bounded_int rng n in
          boundary_insert cur (big off) Bigint.one
            (Bigint.of_uint64 (nth_int64 interest8 vi))
            Little
      | 4 when n > 1 ->
          let l = Prng.bounded_int rng (max 1 (n / 2)) in
          let off = Prng.bounded_int rng n in
          block_delete cur (big off) (big (1 + l))
      | 5 ->
          let byte = Prng.bounded_int rng 256 in
          let at = Prng.bounded_int rng (n + 1) in
          if n + 1 <= max_len then
            block_insert cur (big at) (String.make 1 (Char.chr byte))
          else cur
      | 6 when n > 1 ->
          let l = Prng.bounded_int rng (max 1 (n / 2)) in
          let src = Prng.bounded_int rng n in
          let dst = Prng.bounded_int rng (n + 1) in
          let len = 1 + l in
          if n + min len (n - src) <= max_len then
            block_duplicate cur (big src) (big len) (big dst)
          else cur
      | _ -> cur
    in
    let rec loop cur left work =
      if left = 0 then Work_ok (cur, work)
      else if cur = "" then loop "\000" left work
      else
        let n = String.length cur in
        if n > cap - work then Work_exhausted work
        else loop (step cur n) (left - 1) (work + n)
    in
    loop data steps work0

let mutate_sequence_with_cap cap data seed steps max_len =
  match mutate_sequence_counted data seed steps max_len cap with
  | Work_ok (out, _) -> out
  | Work_exhausted w -> raise (Work_cap_exceeded (w, cap))

let mutate_sequence data seed steps max_len =
  mutate_sequence_with_cap Fuzz_limits.max_sequence_work data seed steps max_len
