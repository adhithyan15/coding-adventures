(* Sign-magnitude integers with little-endian base 2^30 limbs.

   Why 2^30?  A limb product is below 2^60 and the carries added to it stay far
   below 2^62, the largest 63-bit native [int], so no step of the schoolbook
   loops can overflow.  The magnitude array is always normalised: it has no
   leading (high) zero limbs and is empty exactly when the value is zero, so
   structural properties (length, sign) are decided without scanning. *)

let base_bits = 30
let base = 1 lsl base_bits
let mask = base - 1

type t = { sign : int; mag : int array }

let zero = { sign = 0; mag = [||] }

(* Drops high zero limbs; the canonical constructor. *)
let make sign mag =
  let n = ref (Array.length mag) in
  while !n > 0 && mag.(!n - 1) = 0 do
    decr n
  done;
  if !n = 0 then zero
  else
    { sign; mag = (if !n = Array.length mag then mag else Array.sub mag 0 !n) }

(* ------------------------------------------------------- magnitudes ----- *)

let mag_cmp a b =
  let la = Array.length a and lb = Array.length b in
  if la <> lb then compare la lb
  else
    let i = ref (la - 1) in
    while !i >= 0 && a.(!i) = b.(!i) do
      decr i
    done;
    if !i < 0 then 0 else compare a.(!i) b.(!i)

let mag_add a b =
  let la = Array.length a and lb = Array.length b in
  let n = max la lb in
  let r = Array.make (n + 1) 0 in
  let carry = ref 0 in
  for i = 0 to n - 1 do
    let s =
      (if i < la then a.(i) else 0) + (if i < lb then b.(i) else 0) + !carry
    in
    r.(i) <- s land mask;
    carry := s lsr base_bits
  done;
  r.(n) <- !carry;
  r

(* a - b, requires a >= b. *)
let mag_sub a b =
  let la = Array.length a and lb = Array.length b in
  let r = Array.make la 0 in
  let borrow = ref 0 in
  for i = 0 to la - 1 do
    let d = a.(i) - (if i < lb then b.(i) else 0) - !borrow in
    if d < 0 then (
      r.(i) <- d + base;
      borrow := 1)
    else (
      r.(i) <- d;
      borrow := 0)
  done;
  r

let mag_mul a b =
  let la = Array.length a and lb = Array.length b in
  if la = 0 || lb = 0 then [||]
  else
    let r = Array.make (la + lb) 0 in
    for i = 0 to la - 1 do
      let carry = ref 0 in
      for j = 0 to lb - 1 do
        let cur = r.(i + j) + (a.(i) * b.(j)) + !carry in
        r.(i + j) <- cur land mask;
        carry := cur lsr base_bits
      done;
      r.(i + lb) <- r.(i + lb) + !carry
    done;
    r

(* Divides by a single limb d (0 < d < base). Returns (quotient, remainder). *)
let mag_divmod_small a d =
  let n = Array.length a in
  let q = Array.make n 0 in
  let rem = ref 0 in
  for i = n - 1 downto 0 do
    let cur = (!rem lsl base_bits) lor a.(i) in
    q.(i) <- cur / d;
    rem := cur mod d
  done;
  (q, !rem)

let mag_bit_length a =
  let n = Array.length a in
  if n = 0 then 0
  else
    let top = ref a.(n - 1) and bits = ref 0 in
    while !top > 0 do
      incr bits;
      top := !top lsr 1
    done;
    ((n - 1) * base_bits) + !bits

let mag_shift_left a k =
  if Array.length a = 0 then [||]
  else
    let limbs = k / base_bits and bits = k mod base_bits in
    let n = Array.length a in
    let r = Array.make (n + limbs + 1) 0 in
    for i = 0 to n - 1 do
      let v = a.(i) lsl bits in
      r.(i + limbs) <- r.(i + limbs) lor (v land mask);
      r.(i + limbs + 1) <- v lsr base_bits
    done;
    r

let mag_shift_right a k =
  let limbs = k / base_bits and bits = k mod base_bits in
  let n = Array.length a in
  if limbs >= n then [||]
  else
    let m = n - limbs in
    let r = Array.make m 0 in
    for i = 0 to m - 1 do
      let lo = a.(i + limbs) lsr bits in
      let hi =
        if i + limbs + 1 < n then a.(i + limbs + 1) lsl (base_bits - bits)
        else 0
      in
      r.(i) <- lo lor hi land mask
    done;
    r

let mag_bit a i =
  let l = i / base_bits in
  l < Array.length a && (a.(l) lsr (i mod base_bits)) land 1 = 1

(* General division by shift and subtract, most significant bit first. The
   remainder grows by one bit per step, so it never exceeds the divisor's size
   plus one limb. *)
let mag_divmod a b =
  if Array.length b = 1 then
    let q, r = mag_divmod_small a b.(0) in
    (q, if r = 0 then [||] else [| r |])
  else if mag_cmp a b < 0 then ([||], a)
  else
    let bits = mag_bit_length a in
    let q = Array.make (Array.length a) 0 in
    let rem = ref [||] in
    for i = bits - 1 downto 0 do
      let shifted =
        if Array.length !rem = 0 then [| 0 |] else mag_shift_left !rem 1
      in
      shifted.(0) <- (shifted.(0) lor if mag_bit a i then 1 else 0);
      let norm = (make 1 shifted).mag in
      if mag_cmp norm b >= 0 then (
        rem := (make 1 (mag_sub norm b)).mag;
        q.(i / base_bits) <- q.(i / base_bits) lor (1 lsl (i mod base_bits)))
      else rem := norm
    done;
    (q, !rem)

(* ------------------------------------------------------ construction ----- *)

let of_nonneg_int n =
  (* n >= 0 and n <= max_int *)
  let rec limbs n acc =
    if n = 0 then List.rev acc
    else limbs (n lsr base_bits) ((n land mask) :: acc)
  in
  make 1 (Array.of_list (limbs n []))

let one = of_nonneg_int 1
let neg x = if x.sign = 0 then x else { x with sign = -x.sign }
let abs x = if x.sign < 0 then neg x else x
let sign x = x.sign

let compare a b =
  if a.sign <> b.sign then Stdlib.compare a.sign b.sign
  else if a.sign = 0 then 0
  else if a.sign > 0 then mag_cmp a.mag b.mag
  else mag_cmp b.mag a.mag

let equal a b = compare a b = 0

let add a b =
  if a.sign = 0 then b
  else if b.sign = 0 then a
  else if a.sign = b.sign then make a.sign (mag_add a.mag b.mag)
  else
    let c = mag_cmp a.mag b.mag in
    if c = 0 then zero
    else if c > 0 then make a.sign (mag_sub a.mag b.mag)
    else make b.sign (mag_sub b.mag a.mag)

let sub a b = add a (neg b)

let mul a b =
  if a.sign = 0 || b.sign = 0 then zero
  else make (a.sign * b.sign) (mag_mul a.mag b.mag)

let of_int n =
  if n >= 0 then of_nonneg_int n
  else if n = min_int then neg (add (of_nonneg_int max_int) one)
  else neg (of_nonneg_int (-n))

(* Four 16-bit pieces avoid any signed-shift subtlety on the top bit. *)
let of_uint64 w =
  let piece k =
    Int64.to_int (Int64.logand (Int64.shift_right_logical w (16 * k)) 0xFFFFL)
  in
  let r = ref zero in
  for k = 3 downto 0 do
    r := add (make 1 (mag_shift_left !r.mag 16)) (of_nonneg_int (piece k))
  done;
  !r

let pow2 k = make 1 (mag_shift_left [| 1 |] k)

let of_int64 w =
  if Int64.compare w 0L >= 0 then of_uint64 w else sub (of_uint64 w) (pow2 64)

let to_int_opt x =
  if x.sign = 0 then Some 0
  else if mag_bit_length x.mag > 62 then
    (* only min_int = -2^62 needs 63 bits *)
    if x.sign < 0 && equal x (of_int min_int) then Some min_int else None
  else
    let v = ref 0 in
    for i = Array.length x.mag - 1 downto 0 do
      v := (!v lsl base_bits) lor x.mag.(i)
    done;
    Some (if x.sign < 0 then - !v else !v)

(* Low 64 bits of the magnitude, then two's-complement negation if negative:
   Int64.neg wraps modulo 2^64, which is exactly the mask of the reference. *)
let to_int64_wrap x =
  let low = ref 0L in
  for i = 2 downto 0 do
    let limb = if i < Array.length x.mag then x.mag.(i) else 0 in
    low := Int64.logor (Int64.shift_left !low base_bits) (Int64.of_int limb)
  done;
  (* three limbs hold 90 bits; the shifts above drop everything past bit 63 *)
  if x.sign < 0 then Int64.neg !low else !low

let bit_length x = mag_bit_length x.mag

let shift_left x k =
  if k < 0 then invalid_arg "Bigint.shift_left";
  make x.sign (mag_shift_left x.mag k)

let shift_right x k =
  if k < 0 then invalid_arg "Bigint.shift_right";
  make x.sign (mag_shift_right x.mag k)

let div_rem a b =
  if b.sign = 0 then raise Division_by_zero
  else if a.sign = 0 then (zero, zero)
  else
    let q, r = mag_divmod a.mag b.mag in
    (make (a.sign * b.sign) q, make a.sign r)

let rem_pos a d =
  let _, r = div_rem a d in
  if r.sign < 0 then add r (abs d) else r

let fdiv2 x =
  if x.sign >= 0 then shift_right x 1 else neg (shift_right (add (neg x) one) 1)

(* ---------------------------------------------------------- decimal ----- *)

let max_parse_digits = 100_000

let of_string s =
  let n = String.length s in
  let neg_, start = if n > 0 && s.[0] = '-' then (true, 1) else (false, 0) in
  if n - start = 0 then invalid_arg "Bigint.of_string: empty";
  if n - start > max_parse_digits then
    invalid_arg "Bigint.of_string: too many digits";
  let acc = ref [||] in
  (* fold digits in chunks of nine: acc <- acc * 10^k + chunk *)
  let i = ref start in
  while !i < n do
    let k = min 9 (n - !i) in
    let chunk = ref 0 and scale = ref 1 in
    for j = !i to !i + k - 1 do
      let c = s.[j] in
      if c < '0' || c > '9' then invalid_arg "Bigint.of_string: not a digit";
      chunk := (!chunk * 10) + (Char.code c - 48);
      scale := !scale * 10
    done;
    let scaled = mag_mul !acc [| !scale land mask; !scale lsr base_bits |] in
    let scaled = (make 1 scaled).mag in
    acc :=
      (make 1 (mag_add scaled [| !chunk land mask; !chunk lsr base_bits |])).mag;
    i := !i + k
  done;
  make (if neg_ then -1 else 1) !acc

let to_string x =
  if x.sign = 0 then "0"
  else
    let chunks = ref [] and cur = ref x.mag in
    while Array.length !cur > 0 do
      let q, r = mag_divmod_small !cur 1_000_000_000 in
      chunks := r :: !chunks;
      cur := (make 1 q).mag
    done;
    let b = Buffer.create 32 in
    if x.sign < 0 then Buffer.add_char b '-';
    (* a non-zero value always yields at least one chunk *)
    Buffer.add_string b (string_of_int (List.hd !chunks));
    List.iter
      (fun c -> Buffer.add_string b (Printf.sprintf "%09d" c))
      (List.tl !chunks);
    Buffer.contents b
