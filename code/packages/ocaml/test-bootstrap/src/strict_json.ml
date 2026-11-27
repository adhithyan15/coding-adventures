type t =
  | Null
  | Bool of bool
  | Number of string
  | String of string
  | Array of t list
  | Object of (string * t) list

let max_depth = 64
let max_bytes = 8 * 1024 * 1024
let max_nodes = 200_000

exception Fail of string

let parse input =
  let n = String.length input in
  let pos = ref 0 and nodes = ref 0 in
  let at msg = Printf.sprintf "%s at byte %d" msg !pos in
  let peek () = if !pos < n then Some input.[!pos] else None in
  let is_digit c = c >= '0' && c <= '9' in
  let count_node () =
    incr nodes;
    if !nodes > max_nodes then
      raise (Fail (at (Printf.sprintf "more than %d values" max_nodes)))
  in
  let rec skip_ws () =
    match peek () with
    | Some (' ' | '\t' | '\n' | '\r') ->
        incr pos;
        skip_ws ()
    | _ -> ()
  in
  let literal word v =
    let l = String.length word in
    if !pos + l <= n && String.sub input !pos l = word then (
      pos := !pos + l;
      v)
    else raise (Fail (at "invalid literal"))
  in
  let hex4 () =
    if !pos + 4 > n then raise (Fail (at "invalid \\u escape"));
    let v = ref 0 in
    for k = 0 to 3 do
      let c = input.[!pos + k] in
      let d =
        if is_digit c then Char.code c - 48
        else if c >= 'a' && c <= 'f' then Char.code c - 87
        else if c >= 'A' && c <= 'F' then Char.code c - 55
        else raise (Fail (at "invalid \\u escape"))
      in
      v := (!v lsl 4) lor d
    done;
    pos := !pos + 4;
    !v
  in
  let unicode b =
    let unit = hex4 () in
    if unit >= 0xD800 && unit <= 0xDBFF then (
      if !pos + 1 < n && input.[!pos] = '\\' && input.[!pos + 1] = 'u' then
        pos := !pos + 2
      else raise (Fail (at "lone high surrogate"));
      let low = hex4 () in
      if low < 0xDC00 || low > 0xDFFF then
        raise (Fail (at "invalid surrogate pair"));
      Buffer.add_utf_8_uchar b
        (Uchar.of_int (0x10000 + ((unit - 0xD800) lsl 10) + (low - 0xDC00))))
    else if unit >= 0xDC00 && unit <= 0xDFFF then
      raise (Fail (at "lone low surrogate"))
    else Buffer.add_utf_8_uchar b (Uchar.of_int unit)
  in
  let str () =
    incr pos;
    let b = Buffer.create 16 in
    let rec go () =
      match peek () with
      | None -> raise (Fail (at "unterminated string"))
      | Some '"' -> incr pos
      | Some c when Char.code c < 0x20 ->
          raise (Fail (at "raw control character in string"))
      | Some '\\' ->
          incr pos;
          (match peek () with
          | None -> raise (Fail (at "unterminated escape"))
          | Some e -> (
              incr pos;
              match e with
              | '"' -> Buffer.add_char b '"'
              | '\\' -> Buffer.add_char b '\\'
              | '/' -> Buffer.add_char b '/'
              | 'b' -> Buffer.add_char b '\b'
              | 'f' -> Buffer.add_char b '\012'
              | 'n' -> Buffer.add_char b '\n'
              | 'r' -> Buffer.add_char b '\r'
              | 't' -> Buffer.add_char b '\t'
              | 'u' -> unicode b
              | _ -> raise (Fail (at "invalid escape"))));
          go ()
      | Some c ->
          Buffer.add_char b c;
          incr pos;
          go ()
    in
    go ();
    Buffer.contents b
  in
  let digits () =
    while match peek () with Some c -> is_digit c | None -> false do
      incr pos
    done
  in
  let need_digit msg =
    match peek () with
    | Some c when is_digit c -> ()
    | _ -> raise (Fail (at msg))
  in
  let number () =
    let start = !pos in
    if peek () = Some '-' then incr pos;
    (match peek () with
    | Some '0' -> (
        incr pos;
        match peek () with
        | Some c when is_digit c -> raise (Fail (at "leading zero"))
        | _ -> ())
    | Some c when is_digit c -> digits ()
    | _ -> raise (Fail (at "invalid number")));
    if peek () = Some '.' then (
      incr pos;
      need_digit "digits required after .";
      digits ());
    if peek () = Some 'e' || peek () = Some 'E' then (
      incr pos;
      if peek () = Some '+' || peek () = Some '-' then incr pos;
      need_digit "digits required in exponent";
      digits ());
    Number (String.sub input start (!pos - start))
  in
  (* [depth] is the number of containers already open; the 65th is refused
     before any recursion, so the native stack is bounded by the cap. *)
  let rec value depth =
    count_node ();
    match peek () with
    | Some '{' -> obj depth
    | Some '[' -> arr depth
    | Some '"' -> String (str ())
    | Some 't' -> literal "true" (Bool true)
    | Some 'f' -> literal "false" (Bool false)
    | Some 'n' -> literal "null" Null
    | Some c when c = '-' || is_digit c -> number ()
    | None -> raise (Fail (at "unexpected end of input"))
    | Some c ->
        raise (Fail (at (Printf.sprintf "unexpected byte %d" (Char.code c))))
  and arr depth =
    if depth + 1 > max_depth then
      raise (Fail (at (Printf.sprintf "nesting deeper than %d" max_depth)));
    incr pos;
    skip_ws ();
    if peek () = Some ']' then (
      incr pos;
      Array [])
    else
      let rec items acc =
        skip_ws ();
        let v = value (depth + 1) in
        skip_ws ();
        match peek () with
        | Some ',' ->
            incr pos;
            items (v :: acc)
        | Some ']' ->
            incr pos;
            List.rev (v :: acc)
        | _ -> raise (Fail (at "expected , or ]"))
      in
      Array (items [])
  and obj depth =
    if depth + 1 > max_depth then
      raise (Fail (at (Printf.sprintf "nesting deeper than %d" max_depth)));
    incr pos;
    skip_ws ();
    if peek () = Some '}' then (
      incr pos;
      Object [])
    else
      let seen = Hashtbl.create 8 in
      let rec members acc =
        skip_ws ();
        if peek () <> Some '"' then raise (Fail (at "expected a string key"));
        count_node ();
        let key = str () in
        if Hashtbl.mem seen key then raise (Fail (at "duplicate key"));
        Hashtbl.add seen key ();
        skip_ws ();
        if peek () <> Some ':' then raise (Fail (at "expected :"));
        incr pos;
        skip_ws ();
        let v = value (depth + 1) in
        skip_ws ();
        let acc = (key, v) :: acc in
        match peek () with
        | Some ',' ->
            incr pos;
            members acc
        | Some '}' ->
            incr pos;
            List.rev acc
        | _ -> raise (Fail (at "expected , or }"))
      in
      Object (members [])
  in
  try
    if n > max_bytes then
      raise
        (Fail (Printf.sprintf "document of %d bytes exceeds %d" n max_bytes));
    if n >= 3 && String.sub input 0 3 = "\xEF\xBB\xBF" then
      raise (Fail "byte-order mark");
    if not (String.is_valid_utf_8 input) then
      raise (Fail "document is not valid UTF-8");
    skip_ws ();
    let v = value 0 in
    skip_ws ();
    if !pos < n then raise (Fail (at "trailing data after the document"));
    Ok v
  with Fail m -> Error m
