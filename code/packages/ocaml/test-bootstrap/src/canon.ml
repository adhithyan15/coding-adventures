(* Canonical rendering.

     Nil -> nil            Bool -> true / false
     Int -> decimal; above 100,000 bits "<bigint N bits>"
     Bytes -> 0x<lowercase hex>
     Text -> "quoted" (escapes: see Escape)
     List -> [a,b]         Tuple -> (a,b)

   Every cap is enforced WHILE the value is traversed and BEFORE the text it
   would allocate: depth 16, 10,000 nodes, 1 MiB of output; texts and byte
   strings are clipped to the remaining output budget before they are escaped
   or hex-encoded; a cyclic value ends at the depth cap. *)

let max_depth = 16
let max_nodes = 10_000
let max_output = 1_048_576
let max_bigint_bits = 100_000
let unrenderable = "<unrenderable>"

exception Truncated

let hex_digits = "0123456789abcdef"

let render v =
  let out = Buffer.create 64 in
  let nodes = ref 0 in
  let append s =
    Buffer.add_string out s;
    if Buffer.length out > max_output then raise Truncated
  in
  let remaining () = max 0 (max_output - Buffer.length out) in
  let rec emit depth = function
    | _
      when incr nodes;
           !nodes > max_nodes ->
        raise Truncated
    | Val.Nil -> append "nil"
    | Val.Bool x -> append (if x then "true" else "false")
    | Val.Int i ->
        let bits = Bigint.bit_length i in
        if bits > max_bigint_bits then
          append (Printf.sprintf "<bigint %d bits>" bits)
        else append (Bigint.to_string i)
    | Val.Bytes s ->
        let n = min (String.length s) ((remaining () / 2) + 1) in
        let h = Bytes.create (2 * n) in
        for i = 0 to n - 1 do
          let c = Char.code s.[i] in
          Bytes.set h (2 * i) hex_digits.[c lsr 4];
          Bytes.set h ((2 * i) + 1) hex_digits.[c land 15]
        done;
        append ("0x" ^ Bytes.unsafe_to_string h)
    | Val.Text s ->
        let body, cut = Escape.escape_bounded true (remaining ()) s in
        append "\"";
        append body;
        if cut then append "...";
        append "\""
    | Val.List xs -> container depth "[" "]" xs
    | Val.Tuple xs -> container depth "(" ")" xs
  and container depth op cl xs =
    if depth >= max_depth then append "..."
    else (
      append op;
      List.iteri
        (fun i x ->
          if i > 0 then append ",";
          emit (depth + 1) x)
        xs;
      append cl)
  in
  match emit 0 v with
  | () -> Buffer.contents out
  | exception Truncated ->
      let s = Buffer.contents out in
      Escape.clip_text max_output s |> fun s' ->
      if String.length s > max_output then s' else s ^ "..."

let render_with canon x =
  match canon x with
  | v -> render v
  | exception (Sys.Break as e) -> raise e
  | exception (Out_of_memory as e) -> raise e
  | exception _ -> unrenderable

let same_val a b =
  (* returns the remaining node budget, or -1 when unequal / over a cap *)
  let rec go depth budget x y =
    if depth > 64 || budget <= 0 then -1
    else
      match (x, y) with
      | Val.Nil, Val.Nil -> budget - 1
      | Val.Bool p, Val.Bool q when p = q -> budget - 1
      | Val.Int p, Val.Int q when Bigint.equal p q -> budget - 1
      | Val.Bytes p, Val.Bytes q when String.equal p q -> budget - 1
      | Val.Text p, Val.Text q when String.equal p q -> budget - 1
      | Val.List p, Val.List q | Val.Tuple p, Val.Tuple q ->
          go_list depth (budget - 1) p q
      | _ -> -1
  and go_list depth budget p q =
    match (p, q) with
    | [], [] -> budget
    | x :: xs, y :: ys ->
        let b = go (depth + 1) budget x y in
        if b < 0 then -1 else go_list depth b xs ys
    | _ -> -1
  in
  go 0 max_nodes a b >= 0
