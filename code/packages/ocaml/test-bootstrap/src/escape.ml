(* Escaping of text that reaches a log line or a report (VER05q section 7).

   A witness, a case id or an exception name is attacker-influenced data about
   to be printed next to trusted text.  A raw newline forges a log line; a
   bidirectional override (U+202E) reorders what a reader sees; a zero-width
   character hides a difference.  So every such character is written as an
   escape.  The set is copied exactly from the Elixir and Haskell lanes. *)

let needs_escape c =
  c < 0x20 || c = 0x7F
  || (c >= 0x80 && c <= 0x9F)
  || c = 0x2028 || c = 0x2029 || c = 0x061C || c = 0x200E || c = 0x200F
  || (c >= 0x202A && c <= 0x202E)
  || (c >= 0x2066 && c <= 0x2069)
  || c = 0xAD || c = 0x34F || c = 0x180E || c = 0xFEFF
  || (c >= 0x200B && c <= 0x200D)
  || (c >= 0x2060 && c <= 0x2064)
  || (c >= 0xFFF9 && c <= 0xFFFB)
  || (c >= 0xE0000 && c <= 0xE007F)

let utf8_encode b c = Buffer.add_utf_8_uchar b (Uchar.of_int c)

(* One code point's escaped form, appended to [b].  [quote] also escapes the
   backslash and the double quote (inside a quoted text). *)
let escape_code_point quote b c =
  match c with
  | 0x0A -> Buffer.add_string b "\\n"
  | 0x0D -> Buffer.add_string b "\\r"
  | 0x09 -> Buffer.add_string b "\\t"
  | 0x5C when quote -> Buffer.add_string b "\\\\"
  | 0x22 when quote -> Buffer.add_string b "\\\""
  | _ when needs_escape c ->
      if c > 0xFFFF then Buffer.add_string b (Printf.sprintf "\\u{%x}" c)
      else Buffer.add_string b (Printf.sprintf "\\u%04x" c)
  | _ -> utf8_encode b c

(* Escapes at most [budget] bytes of OUTPUT and says whether it stopped early.
   The loop ends as soon as the budget is spent, so a 10 MiB text costs
   O(budget), not O(10 MiB): the cap is enforced BEFORE the allocation.  A byte
   that is not part of valid UTF-8 is written \xHH. *)
let escape_bounded quote budget s =
  let b = Buffer.create (min 64 (max 0 budget + 8)) in
  let n = String.length s in
  let rec go i =
    if i >= n then false
    else if Buffer.length b >= budget then true
    else
      let d = String.get_utf_8_uchar s i in
      if Uchar.utf_decode_is_valid d then (
        escape_code_point quote b (Uchar.to_int (Uchar.utf_decode_uchar d));
        go (i + Uchar.utf_decode_length d))
      else (
        Buffer.add_string b (Printf.sprintf "\\x%02x" (Char.code s.[i]));
        go (i + 1))
  in
  let cut = go 0 in
  (Buffer.contents b, cut)

let escape_text quote s = fst (escape_bounded quote max_int s)

(* At most [limit] bytes, cut on a character boundary, with an ellipsis when
   something was cut. *)
let clip_text limit s =
  if String.length s <= limit then s
  else
    let i = ref limit in
    while !i > 0 && Char.code s.[!i] land 0xC0 = 0x80 do
      decr i
    done;
    String.sub s 0 !i ^ "..."
