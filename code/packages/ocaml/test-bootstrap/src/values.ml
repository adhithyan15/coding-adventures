(* Strict decoding of corpus values (VER05q section 7).

   Encoding (JSON):
     {"kind":"int","value":"-12"}     an integer (canonical decimal text)
     {"kind":"bytes","hex":"00ff"}    a byte string
     true / false                     a bool
     [ ... ]                          a list of the above
     "text"                           a text (types, property names, ids)

   Integers are canonical decimal text of at most 1,000 digits, checked BEFORE
   the digits are folded into a Bigint, so a hostile fixture cannot make the
   decoder allocate a huge number. *)

let max_size = 10_000
let max_int_digits = 1000
let max_fragment_len = 200
let max_problem_len = 400
let max_value_depth = 64
let is_digit c = c >= '0' && c <= '9'

(* Decimal digits only, optional leading '-', no leading zeros, no "-0". *)
let canonical_int t =
  let non_zero_lead d = d <> "" && String.for_all is_digit d && d.[0] <> '0' in
  if t = "" then false
  else if t.[0] = '-' then non_zero_lead (String.sub t 1 (String.length t - 1))
  else t = "0" || non_zero_lead t

let is_lower_hex_even = Tfv.is_lower_hex_even

(* [Error] is a malformed case (not an engine mismatch). *)
let decode json =
  let rec go depth (node : Strict_json.t) =
    if depth > max_value_depth then Error "value nesting too deep"
    else
      match node with
      | Strict_json.Bool b -> Ok (Val.Bool b)
      | Strict_json.String s -> Ok (Val.Text s)
      | Strict_json.Array xs ->
          if List.compare_length_with xs max_size > 0 then Error "list too long"
          else
            let rec all acc = function
              | [] -> Ok (Val.List (List.rev acc))
              | x :: rest -> (
                  match go (depth + 1) x with
                  | Ok v -> all (v :: acc) rest
                  | Error _ as e -> e)
            in
            all [] xs
      | Strict_json.Object fields -> (
          let keys = List.sort compare (List.map fst fields) in
          match (List.assoc_opt "kind" fields, keys) with
          | Some (Strict_json.String "int"), [ "kind"; "value" ] -> (
              match List.assoc_opt "value" fields with
              | Some (Strict_json.String t) when not (canonical_int t) ->
                  Error "non-canonical int"
              | Some (Strict_json.String t) ->
                  if
                    String.length t
                    > max_int_digits + if t.[0] = '-' then 1 else 0
                  then Error "int exceeds 1000 digits"
                  else Ok (Val.Int (Bigint.of_string t))
              | _ -> Error "non-canonical int (value must be a string)")
          | Some (Strict_json.String "bytes"), [ "hex"; "kind" ] -> (
              match List.assoc_opt "hex" fields with
              | Some (Strict_json.String t) ->
                  if not (is_lower_hex_even t) then
                    Error "bytes hex must be even-length lowercase"
                  else if String.length t > 2 * max_size then
                    Error "bytes too long"
                  else Ok (Val.Bytes (Tfv.unhex t))
              | _ -> Error "bytes hex must be a string")
          | _ -> Error "unrecognised value node")
      | _ -> Error "unsupported JSON value"
  in
  go 0 json
