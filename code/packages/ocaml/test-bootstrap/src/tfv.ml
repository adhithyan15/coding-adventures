(* Grammar:

     tfv 1
     # comment
     <id> R<n> K<field> <value> K<field> <value> ...

   A value is one of N T F (null, true, false), I<int> (canonical decimal, at
   most 1000 digits), D<16 hex> (IEEE-754 bits, lowercase), X<hex> and S<hex>
   (bytes / UTF-8 text as even-length lowercase hex), L<n> v1..vn (list) and
   M<n> k1 v1..kn vn (map, no two keys deep-equal).  Tokens are separated by
   exactly one ASCII space.

   Worked examples: "tfv 1\nr R1 Kop S706274 Kdata XFF\n" fails (XFF is
   uppercase and odd length); "bad header\n" fails (the first line must be
   exactly "tfv 1").  A count is NEVER used to reserve memory. *)

let max_input = 1_048_576
let max_tokens = 200_000
let max_depth = 64
let max_int_digits = 1000

exception Bad of string

let is_digit c = c >= '0' && c <= '9'
let all p s = String.for_all p s

let is_lower_hex_even s =
  String.length s mod 2 = 0
  && all (fun c -> is_digit c || (c >= 'a' && c <= 'f')) s

let unhex s =
  let v c = if is_digit c then Char.code c - 48 else Char.code c - 87 in
  String.init
    (String.length s / 2)
    (fun i -> Char.chr ((v s.[2 * i] lsl 4) lor v s.[(2 * i) + 1]))

let valid_record_id t =
  t <> ""
  && all
       (fun c ->
         (c >= 'A' && c <= 'Z')
         || (c >= 'a' && c <= 'z')
         || is_digit c || String.contains "._/-" c)
       t

let valid_field_name t =
  t <> ""
  && t.[0] >= 'a'
  && t.[0] <= 'z'
  && all
       (fun c -> (c >= 'a' && c <= 'z') || is_digit c || c = '_')
       (String.sub t 1 (String.length t - 1))

(* A count: canonical digits, at most 9. *)
let canonical_nat t =
  if t = "" then raise (Bad "empty count")
  else if not (all is_digit t) then raise (Bad "non-digit in count")
  else if String.length t > 1 && t.[0] = '0' then
    raise (Bad "leading zero in count")
  else if String.length t > 9 then raise (Bad "count too large")
  else int_of_string t

let valid_int body =
  let neg = body <> "" && body.[0] = '-' in
  let digits =
    if neg then String.sub body 1 (String.length body - 1) else body
  in
  if digits = "" then raise (Bad "empty integer");
  if not (all is_digit digits) then raise (Bad "non-digit in integer");
  if digits.[0] = '0' && (String.length digits > 1 || neg) then
    raise (Bad "non-canonical integer");
  if String.length digits > max_int_digits then raise (Bad "integer too long")

(* Every NaN equals every NaN; -0 and +0 stay distinct. *)
let float_key tok body =
  let raw = unhex body in
  let b0 = Char.code raw.[0] and b1 = Char.code raw.[1] in
  let ex = (b0 mod 128 * 16) + (b1 / 16) in
  let mantissa_zero =
    b1 mod 16 = 0 && String.for_all (fun c -> c = '\000') (String.sub raw 2 6)
  in
  if ex = 0x7ff && not mantissa_zero then "Dnan" else tok

(* Reads one value; returns the remaining tokens and a deep-equality key. *)
let rec value depth = function
  | [] -> raise (Bad "record ends early")
  | tok :: rest -> (
      let body = String.sub tok 1 (String.length tok - 1) in
      match tok.[0] with
      | 'N' | 'T' | 'F' ->
          if body <> "" then raise (Bad "unexpected body");
          (rest, tok)
      | 'I' ->
          valid_int body;
          (rest, tok)
      | 'D' ->
          if not (String.length body = 16 && is_lower_hex_even body) then
            raise (Bad "float needs exactly 16 lowercase hex digits");
          (rest, float_key tok body)
      | 'X' ->
          if not (is_lower_hex_even body) then
            raise (Bad "bytes need even-length lowercase hex");
          (rest, tok)
      | 'S' ->
          if not (is_lower_hex_even body) then
            raise (Bad "text needs even-length lowercase hex");
          if not (String.is_valid_utf_8 (unhex body)) then
            raise (Bad "text is not strict UTF-8");
          (rest, tok)
      | 'L' -> container true body depth rest
      | 'M' -> container false body depth rest
      | _ -> raise (Bad "unknown tag"))

and container is_list body depth rest =
  if depth + 1 > max_depth then raise (Bad "nesting too deep");
  let count = canonical_nat body in
  if is_list then
    let rec items k ts acc =
      if k = 0 then (ts, acc)
      else
        let ts', key = value (depth + 1) ts in
        items (k - 1) ts' (key :: acc)
    in
    let rest', keys = items count rest [] in
    (rest', "L[" ^ String.concat "," (List.rev keys) ^ ",]")
  else
    let key_set = Hashtbl.create 8 in
    let rec pairs k ts acc =
      if k = 0 then (ts, acc)
      else
        let ts1, kk = value (depth + 1) ts in
        let ts2, vv = value (depth + 1) ts1 in
        if Hashtbl.mem key_set kk then raise (Bad "duplicate map key");
        Hashtbl.add key_set kk ();
        pairs (k - 1) ts2 (("(" ^ kk ^ "=>" ^ vv ^ ")") :: acc)
    in
    let rest', entries = pairs count rest [] in
    ( rest',
      "M{" ^ String.concat "" (List.sort_uniq String.compare entries) ^ "}" )

(* One record line: its id and the number of tokens it used. *)
let validate_record line =
  if String.contains line '\r' then
    raise (Bad "carriage return in a record line");
  let tokens = String.split_on_char ' ' line in
  if List.exists (fun t -> t = "") tokens then
    raise (Bad "leading, trailing or doubled space");
  match tokens with
  | rid :: rn :: rest ->
      if not (valid_record_id rid) then raise (Bad "bad record id");
      if rn.[0] <> 'R' then raise (Bad "expected R<n>");
      let count = canonical_nat (String.sub rn 1 (String.length rn - 1)) in
      let seen = Hashtbl.create 8 in
      let rec fields k ts =
        if k = 0 then ts
        else
          match ts with
          | [] -> raise (Bad "record ends early")
          | key :: rest ->
              if
                not
                  (key.[0] = 'K'
                  && valid_field_name (String.sub key 1 (String.length key - 1))
                  )
              then raise (Bad "bad field key");
              if Hashtbl.mem seen key then raise (Bad "duplicate field");
              Hashtbl.add seen key ();
              let rest', _ = value 0 rest in
              fields (k - 1) rest'
      in
      if fields count rest <> [] then
        raise (Bad "trailing tokens after the last field");
      (rid, List.length tokens)
  | _ -> raise (Bad "a record needs an id and R<n>")

let validate input =
  try
    if String.length input > max_input then raise (Bad "input too large");
    if not (String.is_valid_utf_8 input) then
      raise (Bad "document is not valid UTF-8");
    let ls = String.split_on_char '\n' input in
    let ls = match List.rev ls with "" :: r -> List.rev r | _ -> ls in
    (match ls with
    | h :: _ when h = "tfv 1" -> ()
    | _ -> raise (Bad "the first line must be exactly \"tfv 1\""));
    let seen = Hashtbl.create 16 and toks = ref 0 in
    List.iteri
      (fun i line ->
        if i > 0 && line <> "" && line.[0] <> '#' then (
          let rid, used =
            try validate_record line
            with Bad e -> raise (Bad (Printf.sprintf "line %d: %s" (i + 1) e))
          in
          if Hashtbl.mem seen rid then
            raise (Bad (Printf.sprintf "line %d: duplicate record id" (i + 1)));
          if !toks + used > max_tokens then raise (Bad "too many tokens");
          Hashtbl.add seen rid ();
          toks := !toks + used))
      ls;
    Ok ()
  with Bad e -> Error e

let valid s = Result.is_ok (validate s)
