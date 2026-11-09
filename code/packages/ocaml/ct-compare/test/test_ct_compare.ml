(* Tests for the OCaml lane of CT01.

   Part 1 replays every case of the shared corpus
   (code/specs/fixtures/ct-compare-v1/cases.json), via the vendored copy in
   test/fixtures (dune's workspace root is this package, so the canonical file
   outside it cannot be a dune dependency); a drift test compares the copy to
   the canonical file whenever the test can find it.  Part 2 adds the checks the
   corpus only samples. *)

open Coding_adventures_ct_compare

(* ------------------------------------------------------------------ JSON -- *)

(* A minimal JSON reader: exactly what the corpus uses (objects, arrays,
   strings, booleans, integers).  yojson would be a dependency just for tests. *)
type json =
  | Obj of (string * json) list
  | Arr of json list
  | Str of string
  | Bool of bool
  | Int of int

let parse_json text =
  let pos = ref 0 and len = String.length text in
  let peek () = if !pos < len then text.[!pos] else '\000' in
  let rec skip () =
    if
      !pos < len
      && (peek () = ' ' || peek () = '\n' || peek () = '\t' || peek () = '\r')
    then (
      incr pos;
      skip ())
  in
  let expect c =
    skip ();
    if peek () <> c then failwith (Printf.sprintf "expected %c at %d" c !pos);
    incr pos
  in
  let string_lit () =
    expect '"';
    let b = Buffer.create 16 in
    while peek () <> '"' do
      if peek () = '\\' then incr pos;
      Buffer.add_char b (peek ());
      incr pos
    done;
    incr pos;
    Buffer.contents b
  in
  let rec value () =
    skip ();
    match peek () with
    | '{' ->
        incr pos;
        skip ();
        if peek () = '}' then (
          incr pos;
          Obj [])
        else
          let rec members acc =
            let k = string_lit () in
            expect ':';
            let v = value () in
            skip ();
            let acc = (k, v) :: acc in
            if peek () = ',' then (
              incr pos;
              members acc)
            else (
              expect '}';
              Obj (List.rev acc))
          in
          members []
    | '[' ->
        incr pos;
        skip ();
        if peek () = ']' then (
          incr pos;
          Arr [])
        else
          let rec elements acc =
            let v = value () in
            skip ();
            let acc = v :: acc in
            if peek () = ',' then (
              incr pos;
              elements acc)
            else (
              expect ']';
              Arr (List.rev acc))
          in
          elements []
    | '"' -> Str (string_lit ())
    | 't' ->
        pos := !pos + 4;
        Bool true
    | 'f' ->
        pos := !pos + 5;
        Bool false
    | _ ->
        let start = !pos in
        while !pos < len && peek () >= '0' && peek () <= '9' do
          incr pos
        done;
        Int (int_of_string (String.sub text start (!pos - start)))
  in
  let v = value () in
  skip ();
  if !pos <> len then failwith "trailing JSON content";
  v

let read_file path =
  let ic = open_in_bin path in
  let s = really_input_string ic (in_channel_length ic) in
  close_in ic;
  s

let field o k =
  match o with
  | Obj fs -> (
      try List.assoc k fs with Not_found -> failwith ("missing " ^ k))
  | _ -> failwith "not an object"

let str_field o k =
  match field o k with Str s -> s | _ -> failwith (k ^ " not a string")

let hex_to_bytes h =
  String.init
    (String.length h / 2)
    (fun i -> Char.chr (int_of_string ("0x" ^ String.sub h (2 * i) 2)))

let bytes_to_hex s =
  String.concat ""
    (List.init (String.length s) (fun i ->
         Printf.sprintf "%02x" (Char.code s.[i])))

let u64_of_hex h =
  assert (String.length h = 16);
  Int64.of_string ("0x" ^ h)

(* Corpus values are public test data, so this bool -> bit conditional is fine
   (adapters are test-only code). *)
let bit_of b = if b then 1 else 0

let error_name = function
  | Select_length_mismatch -> "CT_SELECT_LENGTH_MISMATCH"
  | Invalid_choice -> "invalid-choice"

(* --------------------------------------------------------------- corpus --- *)

let corpus = parse_json (read_file "fixtures/ct-compare-v1/cases.json")
let cases = match field corpus "cases" with Arr l -> l | _ -> failwith "cases"

let run_case c () =
  let op = str_field c "operation" in
  let expected = field c "expected" in
  match op with
  | "eq" | "eq_fixed" ->
      let l = hex_to_bytes (str_field c "left_hex")
      and r = hex_to_bytes (str_field c "right_hex") in
      let want =
        match field expected "equal" with Bool b -> b | _ -> failwith "equal"
      in
      let got, bit =
        if op = "eq" then (ct_eq l r, ct_eq_bit l r)
        else (
          (match field c "fixed_size_bytes" with
          | Int n -> Alcotest.(check int) "fixed width" n (String.length l)
          | _ -> failwith "fixed_size_bytes");
          (ct_eq_fixed l r, ct_eq_fixed_bit l r))
      in
      Alcotest.(check bool) "boolean" want got;
      Alcotest.(check int) "bit" (bit_of want) bit
  | "select_bytes" -> (
      let l = hex_to_bytes (str_field c "left_hex")
      and r = hex_to_bytes (str_field c "right_hex") in
      let choice =
        match field c "choice" with
        | Bool b -> bit_of b
        | _ -> failwith "choice"
      in
      match field expected "error_id" with
      | Str id -> (
          Alcotest.(check string) "id" "CT_SELECT_LENGTH_MISMATCH" id;
          match ct_select_bytes l r choice with
          | Error e -> Alcotest.(check string) "error" id (error_name e)
          | Ok _ -> Alcotest.fail "expected an error")
      | _ -> failwith "error_id"
      | exception Failure _ -> (
          match ct_select_bytes l r choice with
          | Ok out ->
              Alcotest.(check string)
                "output"
                (str_field expected "output_hex")
                (bytes_to_hex out);
              Alcotest.(check string)
                "left unchanged" (str_field c "left_hex") (bytes_to_hex l);
              Alcotest.(check string)
                "right unchanged" (str_field c "right_hex") (bytes_to_hex r)
          | Error _ -> Alcotest.fail "unexpected error"))
  | "eq_u64" ->
      let l = u64_of_hex (str_field c "left_u64")
      and r = u64_of_hex (str_field c "right_u64") in
      let want =
        match field expected "equal" with Bool b -> b | _ -> failwith "equal"
      in
      Alcotest.(check bool) "boolean" want (ct_eq_u64 l r);
      Alcotest.(check int) "bit" (bit_of want) (ct_eq_u64_bit l r)
  | other -> Alcotest.failf "unknown corpus operation %s" other

let corpus_tests =
  Alcotest.test_case "profile and size" `Quick (fun () ->
      Alcotest.(check string)
        "profile" "ct-compare-v1"
        (str_field corpus "profile");
      Alcotest.(check int) "cases" 21 (List.length cases))
  :: List.map
       (fun c -> Alcotest.test_case (str_field c "id") `Quick (run_case c))
       cases

(* The vendored copy must match the canonical corpus.  Walk up from the test's
   working directory (inside _build, inside the repository) looking for it; if
   the package was built outside the monorepo there is nothing to compare. *)
let canonical_corpus () =
  let rel = "code/specs/fixtures/ct-compare-v1/cases.json" in
  let rec up dir n =
    if n = 0 then None
    else
      let candidate = Filename.concat dir rel in
      if Sys.file_exists candidate then Some candidate
      else
        let parent = Filename.dirname dir in
        if parent = dir then None else up parent (n - 1)
  in
  up (Sys.getcwd ()) 12

let test_vendored_copy_is_current () =
  match canonical_corpus () with
  | None -> () (* built outside the monorepo: nothing to compare *)
  | Some path ->
      Alcotest.(check bool)
        "vendored cases.json equals the canonical corpus" true
        (read_file path = read_file "fixtures/ct-compare-v1/cases.json")

(* -------------------------------------------------------------- equality -- *)

let bytes_of_list l =
  String.init (List.length l) (fun i -> Char.chr (List.nth l i))

let test_equal_and_empty () =
  Alcotest.(check bool) "empty" true (ct_eq "" "");
  Alcotest.(check bool) "same" true (ct_eq "secret" "secret");
  Alcotest.(check int) "bit 1" 1 (ct_eq_bit "x" "x");
  Alcotest.(check int) "fixed bit 1" 1 (ct_eq_fixed_bit "" "")

let test_every_position () =
  let base = String.init 64 (fun i -> Char.chr (((i * 7) + 7) land 0xFF)) in
  for pos = 0 to 63 do
    let b = Bytes.of_string base in
    Bytes.set b pos (Char.chr (Char.code base.[pos] lxor 1));
    let b = Bytes.to_string b in
    Alcotest.(check bool)
      (Printf.sprintf "position %d" pos)
      false (ct_eq base b);
    Alcotest.(check int) "bit" 0 (ct_eq_bit base b);
    Alcotest.(check bool) "fixed" false (ct_eq_fixed base b)
  done

let test_every_bit_of_every_byte () =
  for v = 0 to 255 do
    let s = String.make 1 (Char.chr v) in
    Alcotest.(check bool) "self" true (ct_eq s s);
    for bit = 0 to 7 do
      Alcotest.(check bool)
        "flip" false
        (ct_eq s (String.make 1 (Char.chr (v lxor (1 lsl bit)))))
    done
  done

let test_every_accumulator_value () =
  (* a single differing byte against zero yields every accumulator 1..255 *)
  for v = 1 to 255 do
    Alcotest.(check int)
      "different" 0
      (ct_eq_bit "\000\000" (bytes_of_list [ 0; v ]))
  done

let test_length_mismatch () =
  Alcotest.(check bool) "shorter left" false (ct_eq "a" "ab");
  Alcotest.(check bool) "shorter right" false (ct_eq "ab" "a");
  Alcotest.(check bool) "empty vs one" false (ct_eq "" "a");
  Alcotest.(check int) "bit" 0 (ct_eq_bit "" "a");
  Alcotest.(check bool) "fixed" false (ct_eq_fixed "abc" "abcd")

let test_nul_and_high_bit () =
  Alcotest.(check bool) "nul differs" false (ct_eq "a\000b" "a\000c");
  Alcotest.(check bool) "nul equal" true (ct_eq "a\000b" "a\000b");
  Alcotest.(check bool) "high bits" true (ct_eq "\xff\x80" "\xff\x80");
  Alcotest.(check bool) "high bit differs" false (ct_eq "\xff" "\x7f")

(* ------------------------------------------------------------- selection -- *)

let ok = function
  | Ok s -> s
  | Error e -> Alcotest.failf "unexpected %s" (error_name e)

let test_picks () =
  Alcotest.(check string) "left" "AB" (ok (ct_select_bytes "AB" "CD" 1));
  Alcotest.(check string) "right" "CD" (ok (ct_select_bytes "AB" "CD" 0))

let test_all_byte_pairs () =
  let right = String.init 256 Char.chr in
  for lv = 0 to 255 do
    let left = String.make 256 (Char.chr lv) in
    Alcotest.(check string) "left" left (ok (ct_select_bytes left right 1));
    Alcotest.(check string) "right" right (ok (ct_select_bytes left right 0))
  done

let test_empty_select () =
  Alcotest.(check string) "1" "" (ok (ct_select_bytes "" "" 1));
  Alcotest.(check string) "0" "" (ok (ct_select_bytes "" "" 0))

let test_select_errors () =
  let is_mismatch = function
    | Error Select_length_mismatch -> true
    | _ -> false
  in
  Alcotest.(check bool) "a/ab" true (is_mismatch (ct_select_bytes "a" "ab" 1));
  Alcotest.(check bool) "ab/a" true (is_mismatch (ct_select_bytes "ab" "a" 0));
  Alcotest.(check bool) "empty/a" true (is_mismatch (ct_select_bytes "" "a" 1));
  (* lengths are checked before the choice *)
  Alcotest.(check bool)
    "length first" true
    (is_mismatch (ct_select_bytes "a" "ab" 7))

let test_invalid_choices () =
  List.iter
    (fun c ->
      match ct_select_bytes "a" "b" c with
      | Error Invalid_choice -> ()
      | _ -> Alcotest.failf "choice %d must be rejected" c)
    [ 2; 3; -1; 255; 256; min_int; max_int; -2 ]

let test_compose () =
  Alcotest.(check string)
    "equal" "yes"
    (ok (ct_select_bytes "yes" "no!" (ct_eq_bit "tag" "tag")));
  Alcotest.(check string)
    "different" "no!"
    (ok (ct_select_bytes "yes" "no!" (ct_eq_bit "tag" "tab")));
  Alcotest.(check string)
    "u64" "no!"
    (ok (ct_select_bytes "yes" "no!" (ct_eq_u64_bit 5L 6L)))

let test_fresh_output () =
  let l = "abc" and r = "xyz" in
  let out = ok (ct_select_bytes l r 1) in
  Alcotest.(check string) "content" l out;
  (* strings are immutable; the result is a fresh buffer, never an input *)
  Alcotest.(check bool) "not physically the input" false (out == l)

(* ------------------------------------------------------------------ u64 --- *)

let test_u64_equal () =
  List.iter
    (fun v ->
      Alcotest.(check bool) "equal" true (ct_eq_u64 v v);
      Alcotest.(check int) "bit" 1 (ct_eq_u64_bit v v))
    [ 0L; 1L; -1L; Int64.min_int; Int64.max_int; 0x0123456789ABCDEFL ]

let test_u64_bit_flips () =
  List.iter
    (fun base ->
      for bit = 0 to 63 do
        let flipped = Int64.logxor base (Int64.shift_left 1L bit) in
        Alcotest.(check bool)
          (Printf.sprintf "bit %d" bit)
          false (ct_eq_u64 base flipped);
        Alcotest.(check bool) "symmetric" false (ct_eq_u64 flipped base);
        Alcotest.(check int) "bit result" 0 (ct_eq_u64_bit base flipped)
      done)
    [ 0L; -1L; 0x5555555555555555L; 0x0123456789ABCDEFL; Int64.min_int ]

let test_u64_extremes () =
  Alcotest.(check bool) "0 vs sign" false (ct_eq_u64 0L Int64.min_int);
  Alcotest.(check bool) "0 vs max" false (ct_eq_u64 0L (-1L));
  Alcotest.(check bool)
    "int64 max vs u64 max" false
    (ct_eq_u64 Int64.max_int (-1L))

let () =
  Alcotest.run "ct-compare"
    [
      ( "corpus",
        corpus_tests
        @ [
            Alcotest.test_case "vendored copy is current" `Quick
              test_vendored_copy_is_current;
          ] );
      ( "equality",
        [
          Alcotest.test_case "equal and empty" `Quick test_equal_and_empty;
          Alcotest.test_case "every byte position" `Quick test_every_position;
          Alcotest.test_case "every bit of every byte" `Quick
            test_every_bit_of_every_byte;
          Alcotest.test_case "every accumulator value" `Quick
            test_every_accumulator_value;
          Alcotest.test_case "length mismatch" `Quick test_length_mismatch;
          Alcotest.test_case "NUL and high bit" `Quick test_nul_and_high_bit;
        ] );
      ( "selection",
        [
          Alcotest.test_case "picks left and right" `Quick test_picks;
          Alcotest.test_case "all 256x256 byte pairs" `Quick test_all_byte_pairs;
          Alcotest.test_case "empty" `Quick test_empty_select;
          Alcotest.test_case "length errors" `Quick test_select_errors;
          Alcotest.test_case "invalid choices" `Quick test_invalid_choices;
          Alcotest.test_case "composes with bit equalities" `Quick test_compose;
          Alcotest.test_case "fresh output" `Quick test_fresh_output;
        ] );
      ( "u64",
        [
          Alcotest.test_case "equal values" `Quick test_u64_equal;
          Alcotest.test_case "64 single-bit flips" `Quick test_u64_bit_flips;
          Alcotest.test_case "extremes" `Quick test_u64_extremes;
        ] );
    ]
