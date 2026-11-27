(* A compact base-10^9 natural number.  Each limb multiplication stays within
   signed 64-bit arithmetic even for the largest factor used by the cutoff. *)
module Big_nat = struct
  let base = 1_000_000_000
  let one = [ 1 ]

  let normalize limbs =
    let rec trim = function
      | [] -> [ 0 ]
      | [ 0 ] as value -> value
      | 0 :: rest -> trim rest
      | value -> value
    in
    trim (List.rev limbs) |> List.rev

  let add left right =
    let rec loop a b carry =
      match (a, b) with
      | [], [] -> if carry = 0 then [] else [ carry ]
      | _ ->
          let x, xs = match a with [] -> (0, []) | h :: t -> (h, t) in
          let y, ys = match b with [] -> (0, []) | h :: t -> (h, t) in
          let total = x + y + carry in
          (total mod base) :: loop xs ys (total / base)
    in
    normalize (loop left right 0)

  let multiply_small value factor =
    let rec loop limbs carry =
      match limbs with
      | [] -> if carry = 0 then [] else [ carry ]
      | limb :: rest ->
          let product = (limb * factor) + carry in
          (product mod base) :: loop rest (product / base)
    in
    normalize (loop value 0)

  let divide_small value divisor =
    let rec loop limbs remainder quotient =
      match limbs with
      | [] -> (quotient, remainder)
      | limb :: rest ->
          let combined = (remainder * base) + limb in
          loop rest (combined mod divisor) ((combined / divisor) :: quotient)
    in
    let quotient, remainder = loop (List.rev value) 0 [] in
    (normalize quotient, remainder)

  let compare left right =
    let left = normalize left and right = normalize right in
    let left = List.rev left and right = List.rev right in
    let rec loop a b =
      match (a, b) with
      | [], [] -> 0
      | x :: xs, y :: ys -> if x = y then loop xs ys else Int.compare x y
      | [], _ -> -1
      | _, [] -> 1
    in
    let by_length = Int.compare (List.length left) (List.length right) in
    if by_length = 0 then loop left right else by_length

  let power_of_two exponent =
    let rec loop remaining value =
      if remaining = 0 then value
      else loop (remaining - 1) (multiply_small value 2)
    in
    loop exponent one
end

type interval = { low : float; high : float }

let finite value = Float.is_finite value

let ordered_finite values =
  if Array.length values = 0 then Error "sample must not be empty"
  else if not (Array.for_all finite values) then
    Error "sample values must be finite"
  else
    Ok
      ( Array.copy values |> fun copy ->
        Array.sort Float.compare copy;
        copy )

let subnormal_midpoint low high =
  let left = Int64.logand (Int64.bits_of_float (abs_float low)) Int64.max_int in
  let right =
    Int64.logand (Int64.bits_of_float (abs_float high)) Int64.max_int
  in
  let sum = Int64.add left right in
  let quotient = Int64.div sum 2L in
  let rounded =
    if Int64.rem sum 2L <> 0L && Int64.logand quotient 1L = 1L then
      Int64.succ quotient
    else quotient
  in
  let signed_bits =
    if low < 0.0 then Int64.logor rounded Int64.min_int else rounded
  in
  Int64.float_of_bits signed_bits

let midpoint low high =
  if low >= 0.0 && high <= 2.0 *. Float.min_float then
    subnormal_midpoint low high
  else if low <= 0.0 && high <= 0.0 && -.low <= 2.0 *. Float.min_float then
    subnormal_midpoint low high
  else if low < 0.0 && high > 0.0 then (low +. high) /. 2.0
  else low +. ((high -. low) /. 2.0)

let median values =
  match ordered_finite values with
  | Error error -> Error error
  | Ok ordered ->
      let length = Array.length ordered in
      let middle = length / 2 in
      if length mod 2 = 1 then Ok ordered.(middle)
      else Ok (midpoint ordered.(middle - 1) ordered.(middle))

let exact_median_rank count confidence =
  if count <= 0 then Error "sample must not be empty"
  else if confidence < 1 || confidence > 99 then
    Error "confidence must be from 1 to 99"
  else
    let open Big_nat in
    let cutoff = multiply_small (power_of_two count) (100 - confidence) in
    let rec search j coefficient tail best =
      if j > count / 2 then best
      else
        let next_tail = add tail coefficient in
        if compare (multiply_small next_tail 200) cutoff > 0 then best
        else
          let next_coefficient, remainder =
            divide_small (multiply_small coefficient (count - j)) (j + 1)
          in
          assert (remainder = 0);
          search (j + 1) next_coefficient next_tail (j + 1)
    in
    Ok (search 0 one [ 0 ] 0)

let median_interval values confidence =
  match ordered_finite values with
  | Error error -> Error error
  | Ok ordered -> (
      match exact_median_rank (Array.length ordered) confidence with
      | Error error -> Error error
      | Ok 0 -> Ok None
      | Ok rank ->
          Ok
            (Some
               {
                 low = ordered.(rank - 1);
                 high = ordered.(Array.length ordered - rank);
               }))
