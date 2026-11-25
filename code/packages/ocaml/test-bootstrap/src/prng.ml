(* SplitMix64 and the unbiased bounded draw.

   OCaml's native [int] has 63 bits, so a 64-bit word lives in an [int64].
   Int64 arithmetic wraps modulo 2^64, which is exactly what the generator
   needs; the only care is that comparison, division and remainder of a word
   whose top bit may be set must use the UNSIGNED versions. *)

type t = { mutable s : int64; threshold : int64 -> int64 }

let golden_gamma = 0x9e3779b97f4a7c15L

let mix z0 =
  let open Int64 in
  let z1 = mul (logxor z0 (shift_right_logical z0 30)) 0xbf58476d1ce4e5b9L in
  let z2 = mul (logxor z1 (shift_right_logical z1 27)) 0x94d049bb133111ebL in
  logxor z2 (shift_right_logical z2 31)

(* Negation wraps modulo 2^64, so [neg n] IS 2^64 - n as an unsigned word. *)
let default_threshold n = Int64.unsigned_rem (Int64.neg n) n
let create ?(threshold = default_threshold) seed = { s = seed; threshold }
let state r = r.s

let next_u64 r =
  r.s <- Int64.add r.s golden_gamma;
  mix r.s

let bounded_u64 r n =
  let t = r.threshold n in
  let rec loop () =
    let x = next_u64 r in
    if Int64.unsigned_compare x t >= 0 then Int64.unsigned_rem x n else loop ()
  in
  loop ()

let two64 = Bigint.pow2 64

let bounded r n =
  if Bigint.compare n Bigint.one < 0 then
    raise (Error.Argument_error "bounded(n) requires n >= 1")
  else if Bigint.compare n two64 > 0 then
    raise (Error.Argument_error "bounded(n) requires n <= 2^64")
  else if Bigint.equal n two64 then Bigint.of_uint64 (next_u64 r)
  else Bigint.of_uint64 (bounded_u64 r (Bigint.to_int64_wrap n))

let bounded_int r n =
  if n < 1 then raise (Error.Argument_error "bounded_int: n must be >= 1")
  else Int64.to_int (bounded_u64 r (Int64.of_int n))

let bounded_big r span =
  if Bigint.compare span Bigint.one < 0 then
    raise (Error.Argument_error "span must be >= 1")
  else if Bigint.compare span two64 <= 0 then bounded r span
  else
    let bits = Bigint.bit_length (Bigint.sub span Bigint.one) in
    let k = max 2 ((bits + 63) / 64) in
    let t = Bigint.rem_pos (Bigint.sub (Bigint.pow2 (64 * k)) span) span in
    let rec loop () =
      let raw = ref Bigint.zero in
      for _ = 1 to k do
        raw :=
          Bigint.add (Bigint.shift_left !raw 64) (Bigint.of_uint64 (next_u64 r))
      done;
      if Bigint.compare !raw t >= 0 then Bigint.rem_pos !raw span else loop ()
    in
    loop ()

let fnv1a64 s =
  let h = ref 0xcbf29ce484222325L in
  String.iter
    (fun c ->
      h :=
        Int64.mul (Int64.logxor !h (Int64.of_int (Char.code c))) 0x100000001b3L)
    s;
  !h

let sub_seed run_seed id =
  mix (Int64.add (Int64.logxor run_seed (fnv1a64 id)) golden_gamma)
