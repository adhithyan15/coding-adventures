(* Tests for the OCaml lane of VER06 (VER06r).  Part 1 replays the complete
   fuzz-v1 corpus and the Go lane's 820 oracle vectors (both embedded as OCaml
   strings, so the tests touch no filesystem), with negative controls.  Part 2
   pins what the corpus cannot catch, with values cross-checked against the Ruby
   and Haskell lanes.  Part 3 exercises the handle, the executor and the campaign.
   Every wait is bounded: a worker that must be released is released by an atomic
   flag. *)

open Coding_adventures_test_bootstrap
module B = Bigint
module J = Strict_json
module M = Fuzz_mutate
module T = Fuzz_types
module H = Fuzz_handle
module HI = Fuzz_handle_internal

let bi = B.of_string
let check_s = Alcotest.(check string)
let check_i = Alcotest.(check int)
let check_b = Alcotest.(check bool)
let tc name f = Alcotest.test_case name `Quick f

let hex s =
  String.concat ""
    (List.map
       (fun c -> Printf.sprintf "%02x" (Char.code c))
       (List.of_seq (String.to_seq s)))

let unhex = Tfv.unhex

let contains hay needle =
  let n = String.length needle and h = String.length hay in
  let rec go i = i + n <= h && (String.sub hay i n = needle || go (i + 1)) in
  go 0

let starts_with ~prefix s =
  String.length s >= String.length prefix
  && String.sub s 0 (String.length prefix) = prefix

let raises_arg name f =
  match f () with
  | exception Error.Argument_error _ -> ()
  | _ -> Alcotest.failf "%s: expected Argument_error" name

(* ===================================================================== *)
(* Part 1: the corpus and the vectors                                    *)
(* ===================================================================== *)

let corpus_cases () =
  match Fuzz_corpus.parse_corpus Fuzz_corpus_data.json with
  | Ok c -> c
  | Error e -> Alcotest.failf "corpus does not parse: %s" e

let test_corpus_replay () =
  let r = Fuzz_corpus.evaluate (corpus_cases ()) in
  check_i "evaluated" 25 r.evaluated;
  Alcotest.(check (list string)) "no problems" [] r.problems;
  let t = Fuzz_corpus.evaluate_text Fuzz_corpus_data.json in
  check_i "text form" 25 t.evaluated

(* --- negative controls: perturb every field of every case --------------- *)

let flip_last_hex s =
  let n = String.length s in
  if n = 0 then "00"
  else
    let c = if s.[n - 1] = '0' then '1' else '0' in
    String.sub s 0 (n - 1) ^ String.make 1 c

let rec perturb ~case ~key (j : J.t) : J.t =
  match j with
  | J.Bool b -> J.Bool (not b)
  | J.String s when key = "text" && case = "corpus_reject" ->
      ignore s;
      J.String "tfv 1\n"
  | J.String s when key = "input" && case = "failure_id_valid" ->
      J.String (if s = "fuzz-crash" then s ^ "x" else "fuzz-crash")
  | J.String s -> J.String (s ^ "x")
  | J.Array xs -> J.Array (List.map (perturb ~case ~key) xs)
  | J.Object fields ->
      J.Object
        (List.map
           (fun (k, v) ->
             match (k, v) with
             | "value", J.String t ->
                 (* a canonical int: add 1 (3 for a width) *)
                 ( "value",
                   J.String
                     (B.to_string
                        (B.add (bi t)
                           (B.of_int
                              (if key = "width" then 3
                               else if
                                 key = "existing_edges" || key = "trace_edges"
                               then 10
                               else 1)))) )
             | "hex", J.String t -> ("hex", J.String (flip_last_hex t))
             | _ -> (k, v))
           fields)
  | other -> other

let get_str fields k =
  match List.assoc_opt k fields with Some (J.String s) -> s | _ -> ""

let replace_field fields k v =
  J.Object
    (List.map (fun (k', v') -> if k' = k then (k', v) else (k', v')) fields)

(* the fields whose change legitimately leaves a case passing *)
let benign id key =
  id = "fuzz/crash_dedup/divergent_signature"
  && List.mem key
       [ "crash_a_kind"; "crash_a_frames"; "crash_b_kind"; "crash_b_frames" ]
  (* the buffer is exactly one word wide: every offset wraps to 0 *)
  || (key = "byte_offset" && (contains id "u16_" || contains id "u32_"))
  (* a full-width overwrite does not read the old bytes *)
  || (key = "input" && contains id "mutate_boundary/u32_")
  (* only the target's prefix survives a splice *)
  || (key = "target" && contains id "mutate_splice/")

let test_negative_controls () =
  let cases = corpus_cases () in
  let count = ref 0 in
  List.iteri
    (fun idx c ->
      match c with
      | J.Object fields ->
          let id = get_str fields "id" and op = get_str fields "op" in
          List.iter
            (fun (key, v) ->
              if key <> "id" && key <> "op" && not (benign id key) then (
                let v' =
                  if op = "private_reporting" && key = "raw_payload" then
                    J.Object
                      [
                        ("kind", J.String "bytes"); ("hex", J.String "3a8f19b2");
                      ]
                  else perturb ~case:op ~key v
                in
                let mutated =
                  List.mapi
                    (fun i c' ->
                      if i = idx then replace_field fields key v' else c')
                    cases
                in
                let r = Fuzz_corpus.evaluate mutated in
                incr count;
                if r.problems = [] then
                  Alcotest.failf "%s.%s: a mutated fixture was accepted" id key;
                if not (List.exists (starts_with ~prefix:id) r.problems) then
                  Alcotest.failf "%s.%s: no problem names the case id" id key;
                List.iter
                  (fun p ->
                    if
                      not
                        (starts_with ~prefix:id p
                        || starts_with ~prefix:"fuzz-v1:" p)
                    then
                      Alcotest.failf "%s.%s: problem without an id: %s" id key p)
                  r.problems))
            fields
      | _ -> Alcotest.fail "case is not an object")
    cases;
  check_b "many mutations were run" true (!count > 100)

let test_corpus_structure_controls () =
  let cases = corpus_cases () in
  let bad name cs =
    let r = Fuzz_corpus.evaluate cs in
    if r.problems = [] then Alcotest.failf "%s: accepted" name
  in
  bad "dropped" (List.tl cases);
  bad "truncated" (List.filteri (fun i _ -> i < 10) cases);
  bad "duplicated" ((List.hd cases :: List.tl cases) @ [ List.hd cases ]);
  let dup_id =
    match (List.hd cases, List.nth cases 1) with
    | J.Object a, J.Object b ->
        J.Object
          (List.map
             (fun (k, v) -> if k = "id" then (k, List.assoc "id" a) else (k, v))
             b)
    | _ -> Alcotest.fail "shape"
  in
  bad "duplicate id"
    (List.hd cases :: dup_id :: List.filteri (fun i _ -> i > 1) cases);
  let r =
    Fuzz_corpus.evaluate ~expected:None
      [
        J.Object
          [ ("id", J.String "x\nforged: ok"); ("op", J.String "no_such_op") ];
      ]
  in
  (match r.problems with
  | p :: _ ->
      check_b "unknown op is a problem" true (contains p "unknown op");
      check_b "the id is escaped" false (String.contains p '\n')
  | [] -> Alcotest.fail "unknown op accepted");
  let r =
    Fuzz_corpus.evaluate ~expected:None
      [
        J.Object
          [
            ("id", J.String "a");
            ("op", J.String "corpus_reject");
            ("extra", J.Null);
          ];
      ]
  in
  check_b "wrong field set" true
    (List.exists (fun p -> contains p "fields") r.problems);
  let r =
    Fuzz_corpus.evaluate ~expected:None
      [ J.Null; J.Object [ ("op", J.String "x") ] ]
  in
  check_b "non-object and missing id" true
    (List.exists (fun p -> contains p "not an object") r.problems
    && List.exists (fun p -> contains p "missing a string id") r.problems);
  let r = Fuzz_corpus.evaluate ~expected:None (List.init 3 (fun _ -> J.Null)) in
  check_b "ids stop at first offender" true
    (List.mem "duplicate or missing case id" r.problems);
  (* the cap on cases *)
  let many = List.init (Fuzz_corpus.max_corpus_cases + 1) (fun _ -> J.Null) in
  let r = Fuzz_corpus.evaluate ~expected:None many in
  check_b "too many cases" true
    (List.exists (fun p -> contains p "exceeds the cap") r.problems);
  check_b "problems clipped" true
    (List.length r.problems <= Fuzz_corpus.max_problems);
  (* parse errors *)
  let t = Fuzz_corpus.evaluate_text "{" in
  check_i "parse failure" 0 t.evaluated;
  check_b "parse failure problem" true (t.problems <> []);
  List.iter
    (fun text ->
      match Fuzz_corpus.parse_corpus text with
      | Ok _ -> Alcotest.failf "accepted %s" text
      | Error _ -> ())
    [
      "{\"schema_version\": 2, \"cases\": []}";
      "{\"schema_version\": 1, \"cases\": 3}";
      "[]";
      "{\"schema_version\":1,\"cases\":[],\"x\":1}";
      "nope";
    ]

let test_hooks_sabotage () =
  let d = Fuzz_hooks.default in
  let wrong_bytes s = s ^ "!" in
  let sabotage =
    [
      ( "bit_flip",
        { d with bit_flip = (fun a b c -> wrong_bytes (d.bit_flip a b c)) } );
      ( "byte_flip",
        { d with byte_flip = (fun a b c -> wrong_bytes (d.byte_flip a b c)) } );
      ( "arith",
        {
          d with
          byte_arithmetic =
            (fun a b c w e -> wrong_bytes (d.byte_arithmetic a b c w e));
        } );
      ( "boundary",
        {
          d with
          boundary_insert =
            (fun a b c w e -> wrong_bytes (d.boundary_insert a b c w e));
        } );
      ( "delete",
        {
          d with
          block_delete = (fun a b c -> wrong_bytes (d.block_delete a b c));
        } );
      ( "insert",
        {
          d with
          block_insert = (fun a b c -> wrong_bytes (d.block_insert a b c));
        } );
      ( "duplicate",
        {
          d with
          block_duplicate =
            (fun a b c e -> wrong_bytes (d.block_duplicate a b c e));
        } );
      ( "splice",
        { d with splice = (fun a b c e -> wrong_bytes (d.splice a b c e)) } );
      ( "sequence",
        {
          d with
          mutate_sequence =
            (fun a b c m cap ->
              match d.mutate_sequence a b c m cap with
              | M.Work_ok (o, w) -> M.Work_ok (wrong_bytes o, w)
              | e -> e);
        } );
      ( "admit",
        {
          d with
          corpus_admit =
            (fun a b ->
              let ok, n = d.corpus_admit a b in
              (not ok, n));
        } );
      ( "admit count",
        {
          d with
          corpus_admit =
            (fun a b ->
              let ok, n = d.corpus_admit a b in
              (ok, n + 1));
        } );
      ( "dedup",
        {
          d with
          duplicate_crash = (fun a b c e -> not (d.duplicate_crash a b c e));
        } );
      ("failure id", { d with failure_id = (fun s -> not (d.failure_id s)) });
      ("failure id accepts all", { d with failure_id = (fun _ -> true) });
      ( "tap",
        { d with format_tap = (fun n t f s -> d.format_tap n t f s ^ " ") } );
      ( "tap leaks",
        {
          d with
          format_tap = (fun n t f s -> d.format_tap n t f s ^ "deadbeef");
        } );
      ( "tap newline",
        { d with format_tap = (fun n t f s -> d.format_tap n t f s ^ "\n") } );
      ("tfv", { d with tfv_valid = (fun _ -> true) });
    ]
  in
  List.iter
    (fun (name, hooks) ->
      let r = Fuzz_corpus.evaluate ~hooks (corpus_cases ()) in
      if r.problems = [] then Alcotest.failf "sabotage %s was not noticed" name)
    sabotage;
  (* a sabotaged mutator that raises is reported as a problem naming the case *)
  let raising = { d with bit_flip = (fun _ _ _ -> failwith "boom") } in
  let r = Fuzz_corpus.evaluate ~hooks:raising (corpus_cases ()) in
  check_b "raising hook" true
    (List.exists
       (fun p ->
         starts_with ~prefix:"fuzz/mutate_bit_flip/" p
         && contains p "could not be evaluated")
       r.problems)

(* --- the aggregate mutate_sequence cap ---------------------------------- *)

let seq_case id hexin steps =
  J.Object
    [
      ("id", J.String id);
      ("op", J.String "mutate_sequence");
      ("input", J.Object [ ("kind", J.String "bytes"); ("hex", J.String hexin) ]);
      ("seed", J.Object [ ("kind", J.String "int"); ("value", J.String "42") ]);
      ("steps", J.Object [ ("kind", J.String "int"); ("value", J.String steps) ]);
      ("output", J.Object [ ("kind", J.String "bytes"); ("hex", J.String "00") ]);
    ]

let test_aggregate_work_cap () =
  (* the allowance is 2^24 byte-copies for the WHOLE corpus.  A case that spends
     it (or is refused) zeroes it, so every later case fails at once *)
  let d = Fuzz_hooks.default in
  let spend =
    {
      d with
      mutate_sequence =
        (fun _ _ _ _ cap ->
          check_b "handed what is left" true
            (cap <= Fuzz_limits.corpus_sequence_work);
          M.Work_ok ("\000", Fuzz_limits.corpus_sequence_work));
    }
  in
  let r =
    Fuzz_corpus.evaluate ~hooks:spend ~expected:None
      [
        seq_case "spender" "00" "1";
        seq_case "small" "00" "1";
        seq_case "small2" "00" "1";
      ]
  in
  check_b "spender is fine" false
    (List.exists (starts_with ~prefix:"spender:") r.problems);
  check_b "later cases fail at once" true
    (List.exists
       (fun p -> starts_with ~prefix:"small:" p && contains p "exhausted")
       r.problems
    && List.exists
         (fun p -> starts_with ~prefix:"small2:" p && contains p "exhausted")
         r.problems);
  (* a breach (Work_exhausted) zeroes it too, and is a problem naming its case *)
  let breach =
    { d with mutate_sequence = (fun _ _ _ _ _ -> M.Work_exhausted 5) }
  in
  let r =
    Fuzz_corpus.evaluate ~hooks:breach ~expected:None
      [ seq_case "b1" "00" "1"; seq_case "b2" "00" "1" ]
  in
  check_b "breach named" true
    (List.exists
       (fun p -> starts_with ~prefix:"b1:" p && contains p "exhausted")
       r.problems);
  check_b "b2 fails without running" true
    (List.exists (fun p -> starts_with ~prefix:"b2:" p) r.problems);
  (* the real thing: a case that copies more than the allowance is refused *)
  let r =
    Fuzz_corpus.evaluate ~expected:None [ seq_case "real" "00" "10001" ]
  in
  check_b "steps cap" true
    (List.exists
       (fun p -> starts_with ~prefix:"real:" p && contains p "outside")
       r.problems);
  let ok = Fuzz_corpus.evaluate ~expected:None [ seq_case "fine" "00" "1" ] in
  check_b "an honest mismatch is reported, not an exhaustion" true
    (List.for_all (fun p -> not (contains p "exhausted")) ok.problems)

(* hostile fixtures: every malformed field is a problem naming its case, never a skip *)
let jint v = J.Object [ ("kind", J.String "int"); ("value", J.String v) ]
let jbytes h = J.Object [ ("kind", J.String "bytes"); ("hex", J.String h) ]

let jcase id op fields =
  J.Object (("id", J.String id) :: ("op", J.String op) :: fields)

let test_malformed_cases () =
  let bad name c =
    let r = Fuzz_corpus.evaluate ~expected:None [ c ] in
    if not (List.exists (starts_with ~prefix:(name ^ ":")) r.problems) then
      Alcotest.failf "%s: no problem names the case (%s)" name
        (String.concat " | " r.problems)
  in
  let flip = [ "input"; "bit_offset"; "count"; "output" ] in
  let mk name op keys vals =
    bad name (jcase name op (List.combine keys vals))
  in
  mk "m1" "mutate_bit_flip" flip [ jint "1"; jint "0"; jint "1"; jbytes "00" ];
  mk "m2" "mutate_bit_flip" flip
    [ jbytes "00"; jbytes "00"; jint "1"; jbytes "00" ];
  mk "m3" "mutate_bit_flip" flip [ jbytes "00"; jint "0"; jint "1"; jint "0" ];
  mk "m4" "mutate_arithmetic"
    [ "input"; "byte_offset"; "delta"; "width"; "endian"; "output" ]
    [ jbytes "00"; jint "0"; jint "1"; jint "1"; J.String "huge"; jbytes "01" ];
  mk "m5" "mutate_arithmetic"
    [ "input"; "byte_offset"; "delta"; "width"; "endian"; "output" ]
    [ jbytes "00"; jint "0"; jint "1"; jint "1"; J.Bool true; jbytes "01" ];
  mk "m6" "mutate_sequence"
    [ "input"; "seed"; "steps"; "output" ]
    [ jbytes "00"; jint "-1"; jint "1"; jbytes "00" ];
  mk "m7" "mutate_sequence"
    [ "input"; "seed"; "steps"; "output" ]
    [ jbytes "00"; jint "18446744073709551616"; jint "1"; jbytes "00" ];
  mk "m8" "mutate_sequence"
    [ "input"; "seed"; "steps"; "output" ]
    [ jbytes "00"; jint "1"; jint "-1"; jbytes "00" ];
  mk "m9" "mutate_sequence"
    [ "input"; "seed"; "steps"; "output" ]
    [ jbytes "00"; jint "1"; jbytes "00"; jbytes "00" ];
  let adm = [ "existing_edges"; "trace_edges"; "admitted"; "new_edges" ] in
  mk "m10" "corpus_admit" adm
    [
      J.Array [ jint "18446744073709551616" ];
      J.Array [];
      J.Bool false;
      jint "0";
    ];
  mk "m11" "corpus_admit" adm
    [ J.Array [ jint "-1" ]; J.Array []; J.Bool false; jint "0" ];
  mk "m12" "corpus_admit" adm
    [ J.Array [ J.Bool true ]; J.Array []; J.Bool false; jint "0" ];
  mk "m13" "corpus_admit" adm [ jint "1"; J.Array []; J.Bool false; jint "0" ];
  mk "m14" "corpus_admit" adm [ J.Array []; J.Array []; jint "1"; jint "0" ];
  let dd =
    [
      "crash_a_kind";
      "crash_a_frames";
      "crash_b_kind";
      "crash_b_frames";
      "is_duplicate";
    ]
  in
  mk "m15" "crash_dedup" dd
    [ jint "1"; J.Array []; J.String "k"; J.Array []; J.Bool true ];
  mk "m16" "crash_dedup" dd
    [
      J.String "k"; J.Array [ jint "1" ]; J.String "k"; J.Array []; J.Bool true;
    ];
  mk "m17" "crash_dedup" dd
    [ J.String "k"; jint "1"; J.String "k"; J.Array []; J.Bool true ];
  mk "m18" "crash_dedup" dd
    [ J.String "k"; J.Array []; J.String "k"; J.Array []; J.Bool false ];
  let pr =
    [
      "target";
      "failure_id";
      "raw_payload";
      "sig_hash";
      "tap_line";
      "contains_raw_payload";
    ]
  in
  mk "m19" "private_reporting" pr
    [
      J.String "t";
      J.String "nope";
      jbytes "";
      J.String "3a8f19b2";
      J.String "x";
      J.Bool false;
    ];
  mk "m20" "private_reporting" pr
    [
      J.String "t";
      J.String "fuzz-crash";
      jbytes "";
      J.String "BAD";
      J.String "x";
      J.Bool false;
    ];
  mk "m21" "failure_id_valid" [ "input"; "valid" ] [ jint "1"; J.Bool false ];
  mk "m22" "failure_id_valid" [ "input"; "valid" ]
    [ J.String "fuzz-crash"; jint "1" ];
  mk "m23" "corpus_reject" [ "text" ] [ jint "1" ];
  mk "m24" "mutate_bit_flip" flip
    [ jbytes "0"; jint "0"; jint "1"; jbytes "00" ];
  mk "m25" "mutate_bit_flip" flip
    [ J.Object [ ("kind", J.String "weird") ]; jint "0"; jint "1"; jbytes "00" ];
  mk "m26" "mutate_bit_flip" flip
    [ jbytes "00"; jint "01"; jint "1"; jbytes "00" ];
  mk "m27" "mutate_bit_flip" flip [ jbytes "00"; J.Null; jint "1"; jbytes "00" ];
  (* a list too long and nesting too deep are refused *)
  mk "m28" "corpus_admit" adm
    [
      J.Array (List.init 10_001 (fun _ -> jint "1"));
      J.Array [];
      J.Bool false;
      jint "0";
    ];
  let rec deep n = if n = 0 then J.Array [] else J.Array [ deep (n - 1) ] in
  mk "m29" "corpus_admit" adm [ deep 70; J.Array []; J.Bool false; jint "0" ];
  mk "m30" "mutate_bit_flip" flip
    [ jbytes (String.make 20_002 '0'); jint "0"; jint "1"; jbytes "00" ];
  mk "m31" "mutate_bit_flip" flip
    [ jint (String.make 1001 '1'); jint "0"; jint "1"; jbytes "00" ];
  mk "m32" "mutate_block_insert"
    [ "input"; "offset"; "insert_bytes"; "output" ]
    [ jbytes "00"; jint "0"; jint "1"; jbytes "00" ]

(* --- the Go lane's vector table ----------------------------------------- *)

type vector = { op : string; args : string list; out : string }

let split_on c s = String.split_on_char c s

let parse_vectors text =
  let lines = split_on '\n' text in
  let rec drop = function
    | [] -> None
    | l :: rest ->
        if l = "var referenceVectors = []refVector{" then Some rest
        else drop rest
  in
  match drop lines with
  | None -> Error "the vector table moved"
  | Some rest ->
      let rec take = function [] | "}" :: _ -> [] | l :: r -> l :: take r in
      let rows = take rest in
      let row i line =
        let n = String.length line in
        if
          n > 3
          && String.sub line 0 3 = "\t{\""
          && String.sub line (n - 3) 3 = "\"},"
        then
          let quoted =
            List.filteri (fun k _ -> k mod 2 = 1) (split_on '"' line)
          in
          match quoted with
          | op :: (_ :: _ as more) ->
              let rev = List.rev more in
              Ok { op; args = List.rev (List.tl rev); out = List.hd rev }
          | _ -> Error (Printf.sprintf "unparseable vector row %d" i)
        else Error (Printf.sprintf "unparseable vector row %d: %s" i line)
      in
      let rec all i acc = function
        | [] -> Ok (List.rev acc)
        | l :: r -> (
            match row i l with
            | Ok v -> all (i + 1) (v :: acc) r
            | Error _ as e -> e)
      in
      all 0 [] rows

let vector_ops =
  [
    "bit_flip";
    "byte_flip";
    "arithmetic";
    "boundary";
    "block_delete";
    "block_insert";
    "block_duplicate";
    "splice";
    "sequence";
  ]

let layout = function
  | "bit_flip" | "byte_flip" | "block_delete" -> Some "hii"
  | "arithmetic" | "boundary" -> Some "hiiie"
  | "block_insert" -> Some "hih"
  | "block_duplicate" -> Some "hiii"
  | "splice" -> Some "hhii"
  | "sequence" -> Some "hiii"
  | _ -> None

(* Every token is validated BEFORE it is converted: integers are at most 40
   decimal digits, hex tokens are lowercase even-length hex. *)
let check_tokens op args =
  match layout op with
  | None -> None
  | Some kinds ->
      if String.length kinds <> List.length args then Some (op ^ ": wrong arity")
      else
        let digits d =
          d <> ""
          && String.length d <= 40
          && String.for_all (fun c -> c >= '0' && c <= '9') d
        in
        let bad k a =
          match k with
          | 'i' ->
              let d =
                if a <> "" && a.[0] = '-' then
                  String.sub a 1 (String.length a - 1)
                else a
              in
              if digits d then None
              else Some "non-numeric or over-long integer token"
          | 'h' ->
              if Tfv.is_lower_hex_even a then None else Some "bad hex token"
          | _ -> None
        in
        let rec go i = function
          | [] -> None
          | a :: r -> (
              match bad kinds.[i] a with
              | Some e -> Some (op ^ ": " ^ e)
              | None -> go (i + 1) r)
        in
        go 0 args

let run_vector v =
  match check_tokens v.op v.args with
  | Some e -> Error e
  | None -> (
      let h = unhex and i = bi in
      let endian s = Option.value (M.parse_endian s) ~default:M.Little in
      let check got =
        if hex got = v.out then Ok ()
        else Error (Printf.sprintf "%s: got %s, want %s" v.op (hex got) v.out)
      in
      match (v.op, v.args) with
      | "bit_flip", [ d; o; c ] -> check (M.bit_flip (h d) (i o) (i c))
      | "byte_flip", [ d; o; c ] -> check (M.byte_flip (h d) (i o) (i c))
      | "arithmetic", [ d; o; dl; w; e ] ->
          check (M.byte_arithmetic (h d) (i o) (i dl) (i w) (endian e))
      | "boundary", [ d; o; w; x; e ] ->
          check (M.boundary_insert (h d) (i o) (i w) (i x) (endian e))
      | "block_delete", [ d; o; l ] -> check (M.block_delete (h d) (i o) (i l))
      | "block_insert", [ d; o; ins ] ->
          check (M.block_insert (h d) (i o) (h ins))
      | "block_duplicate", [ d; s; l; t ] ->
          check (M.block_duplicate (h d) (i s) (i l) (i t))
      | "splice", [ a; b; ts; ds ] -> check (M.splice (h a) (h b) (i ts) (i ds))
      | "sequence", [ d; sd; st; ml ] -> (
          match B.to_int_opt (i ml) with
          | None -> Error "sequence max_len too large"
          | Some ml -> check (M.mutate_sequence (h d) (i sd) (i st) ml))
      | _ -> Error ("unknown vector op or arity: " ^ v.op))

let test_vectors () =
  match parse_vectors Go_vectors_data.table with
  | Error e -> Alcotest.failf "vectors: %s" e
  | Ok vs ->
      let results = List.map (fun v -> (v, run_vector v)) vs in
      let bad =
        List.filter_map
          (fun (_, r) -> match r with Error e -> Some e | Ok () -> None)
          results
      in
      Alcotest.(check (list string))
        "no vector problems (first 10)" []
        (List.filteri (fun i _ -> i < 10) bad);
      let counts op = List.length (List.filter (fun v -> v.op = op) vs) in
      List.iter
        (fun op ->
          if counts op < 50 then Alcotest.failf "too few vectors for %s" op)
        vector_ops;
      check_i "all vectors" 820 (List.length vs);
      check_i "none failed" 0 (List.length bad)

let test_vector_replayer_rejects () =
  let bad v =
    match run_vector v with
    | Error _ -> ()
    | Ok () -> Alcotest.failf "accepted %s" v.op
  in
  bad
    {
      op = "bit_flip";
      args = [ "00"; "1" ^ String.make 5000 '9'; "1" ];
      out = "00";
    };
  bad { op = "bit_flip"; args = [ "00"; "1x"; "1" ]; out = "00" };
  bad { op = "bit_flip"; args = [ "0g"; "1"; "1" ]; out = "00" };
  bad { op = "bit_flip"; args = [ "00"; "1" ]; out = "00" };
  bad { op = "nope"; args = []; out = "" };
  bad { op = "bit_flip"; args = [ "00"; "0"; "1" ]; out = "02" };
  (match
     run_vector { op = "bit_flip"; args = [ "00"; "0"; "1" ]; out = "01" }
   with
  | Ok () -> ()
  | Error e -> Alcotest.fail e);
  (match
     run_vector
       {
         op = "sequence";
         args = [ "00"; "1"; "1"; "99999999999999999999" ];
         out = "00";
       }
   with
  | Error _ -> ()
  | Ok () -> Alcotest.fail "max_len");
  (match check_tokens "zzz" [] with
  | None -> ()
  | Some _ -> Alcotest.fail "unknown op has no layout");
  List.iter
    (fun text ->
      match parse_vectors text with
      | Ok _ -> Alcotest.failf "accepted %S" text
      | Error _ -> ())
    [
      "nothing";
      "var referenceVectors = []refVector{\n\tgarbage\n}\n";
      "var referenceVectors = []refVector{\n\t{\"x\"},\n}\n";
    ]

(* ===================================================================== *)
(* Part 2: pins, mutators, signatures, TAP                               *)
(* ===================================================================== *)

let big = B.of_int
let one = B.one

let test_mutators () =
  (* parity: bit_flip "\0\0" 9 1 is "\0\2"; the 17-count worked example *)
  check_s "bit 9" "0002" (hex (M.bit_flip "\000\000" (big 9) one));
  check_s "17 bits from 0: bits 1..15" "feff"
    (hex (M.bit_flip "\000\000" B.zero (big 17)));
  check_s "empty" "" (M.bit_flip "" B.zero one);
  check_s "count 0" "0000" (hex (M.bit_flip "\000\000" B.zero B.zero));
  check_s "negative count" "0000"
    (hex (M.byte_flip "\000\000" B.zero (big (-3))));
  check_s "negative offset wraps" "0080"
    (hex (M.bit_flip "\000\000" (big (-1)) one));
  check_s "wrapping window" "0180"
    (hex (M.bit_flip "\000\000" (big 15) (big 2)));
  check_s "byte flip wraps" "ffff"
    ( hex (M.byte_flip "\000\000" (big 1) (big 2)) |> fun s ->
      if s = "ffff" then s else s );
  (* a 1,000-digit count over 1 MiB is cheap and builds no big integer from the data *)
  let mib = String.make (1 lsl 20) 'a' in
  let huge = bi (String.make 1000 '7') in
  let t0 = Sys.time () in
  let out = M.bit_flip mib (big 3) huge in
  let out2 = M.byte_flip mib huge huge in
  check_i "length kept" (1 lsl 20) (String.length out);
  check_i "length kept 2" (1 lsl 20) (String.length out2);
  check_b "cheap" true (Sys.time () -. t0 < 5.0);
  (* every unit hit an even number of times is the identity *)
  check_s "even full passes" "abc" (M.byte_flip "abc" (big 7) (big 6));
  check_s "odd full passes"
    (String.map (fun c -> Char.chr (Char.code c lxor 0xff)) "abc")
    (M.byte_flip "abc" (big 7) (big 3));
  (* arithmetic *)
  check_s "u16 le" "0001"
    (hex (M.byte_arithmetic "\xff\x00" B.zero one (big 2) M.Little));
  check_s "u16 be" "0100"
    (hex (M.byte_arithmetic "\x00\xff" B.zero one (big 2) M.Big));
  check_s "u8 wrap" "00"
    (hex (M.byte_arithmetic "\xff" B.zero one one M.Little));
  check_s "u64 negative delta wraps" "ffffffffffffffff"
    (hex
       (M.byte_arithmetic (String.make 8 '\000') B.zero (big (-1)) (big 8)
          M.Little));
  check_s "u64 le add" "0000000000000001"
    (hex (M.byte_arithmetic (String.make 8 '\000') B.zero one (big 8) M.Big));
  check_s "huge delta is reduced mod 2^64" "01"
    (hex (M.byte_arithmetic "\x00" B.zero (B.add (B.pow2 64) one) one M.Little));
  check_s "width 3 unsupported" "0000"
    (hex (M.byte_arithmetic "\000\000" B.zero one (big 3) M.Little));
  check_s "too short" "00"
    (hex (M.byte_arithmetic "\000" B.zero one (big 2) M.Little));
  check_s "offset wraps into range" "0001"
    (hex (M.byte_arithmetic "\000\000" (big 7) one one M.Little));
  (* boundary *)
  check_s "u32 signed min" "00000080"
    (hex
       (M.boundary_insert "\000\000\000\000" B.zero (big 4) (bi "2147483648")
          M.Little));
  check_s "u64 max be" "ffffffffffffffff"
    (hex
       (M.boundary_insert (String.make 8 '\000') B.zero (big 8)
          (bi "18446744073709551615")
          M.Big));
  check_s "value reduced" "ff"
    (hex (M.boundary_insert "\000" B.zero one (bi "511") M.Little));
  check_s "negative value" "ff"
    (hex (M.boundary_insert "\000" B.zero one (big (-1)) M.Little));
  check_s "bad width" "00"
    (hex (M.boundary_insert "\000" B.zero (big 5) one M.Little));
  check_i "width_of" 8 (M.width_of (big 8));
  check_i "width_of huge" 0 (M.width_of (B.pow2 70));
  (* blocks *)
  check_s "delete middle" "010405"
    (hex (M.block_delete "\x01\x02\x03\x04\x05" one (big 2)));
  check_s "delete clamped" "01"
    (hex (M.block_delete "\x01\x02\x03" one (bi "99999999999999999999999")));
  check_s "delete empty" "" (M.block_delete "" one one);
  check_s "delete zero" "ab" (M.block_delete "ab" one B.zero);
  check_s "insert middle" "0102030405"
    (hex (M.block_insert "\x01\x02\x05" (big 2) "\x03\x04"));
  check_s "insert into empty" "zz" (M.block_insert "" (big 5) "zz");
  check_s "insert at end" "abz" (M.block_insert "ab" (big 2) "z");
  check_s "duplicate prefix" "0102030102"
    (hex (M.block_duplicate "\x01\x02\x03" B.zero (big 2) (big 3)));
  check_s "duplicate clamped" "abcc"
    (M.block_duplicate "abc" (big 2) (big 99) (big 3));
  check_s "duplicate empty" "" (M.block_duplicate "" one one one);
  check_s "duplicate zero" "ab" (M.block_duplicate "ab" one B.zero one);
  (* splice: unequal splits are not interchangeable *)
  check_s "splice cross" "01020708"
    (hex (M.splice "\x01\x02\x03\x04" "\x05\x06\x07\x08" (big 2) (big 2)));
  check_s "splice unequal" "010203080808"
    (hex (M.splice "\x01\x02\x03\x04" "\x08\x08\x08" (big 3) (big 0)));
  check_s "splice unequal swapped" "01020405060708"
    ( hex (M.splice "\x01\x02\x03\x04" "\x04\x05\x06\x07\x08" (big 2) (big 0))
    |> fun s -> if s = "01020405060708" then s else s );
  check_s "splice clamps" "0102"
    (hex (M.splice "\x01\x02" "\x03" (big 99) (big 99)));
  check_s "splice negative" "03"
    (hex (M.splice "\x01\x02" "\x03" (big (-4)) (big (-4))));
  check_s "splice huge" "0102"
    (hex (M.splice "\x01\x02" "\x03" (B.pow2 80) (B.pow2 80)));
  check_s "endians" "little,big"
    (String.concat ","
       [
         (match M.parse_endian "little" with
         | Some M.Little -> "little"
         | _ -> "?");
         (match M.parse_endian "big" with Some M.Big -> "big" | _ -> "?");
       ]);
  check_b "endian case" true (M.parse_endian "Little" = None)

let test_constants () =
  Alcotest.(check (list int64)) "i8" [ 0L; 1L; 127L; 128L; 255L ] M.interest8;
  Alcotest.(check (list int64))
    "i16"
    [ 0L; 1L; 255L; 256L; 32767L; 32768L; 65535L ]
    M.interest16;
  Alcotest.(check (list string))
    "i32"
    [ "0"; "1"; "65535"; "65536"; "2147483647"; "2147483648"; "4294967295" ]
    (List.map (Printf.sprintf "%Lu") M.interest32);
  Alcotest.(check (list string))
    "i64"
    [
      "0";
      "1";
      "4294967295";
      "4294967296";
      "9223372036854775807";
      "9223372036854775808";
      "18446744073709551615";
    ]
    (List.map (Printf.sprintf "%Lu") M.interest64);
  Alcotest.(check (list int)) "deltas" [ -35; -8; -1; 1; 8; 35 ] M.deltas;
  check_i "widths" 4 (List.length M.interest_by_width);
  (* every interest value is written as that width reads it back *)
  List.iter
    (fun (w, vals) ->
      List.iter
        (fun v ->
          let zero = String.make w '\000' in
          let out =
            M.boundary_insert zero B.zero (big w) (B.of_uint64 v) M.Little
          in
          let again = M.byte_arithmetic out B.zero B.zero (big w) M.Little in
          check_s "stable" (hex out) (hex again);
          let be =
            M.boundary_insert zero B.zero (big w) (B.of_uint64 v) M.Big
          in
          check_s "be is the reverse of le" (hex out)
            (hex (String.init w (fun i -> be.[w - 1 - i]))))
        vals)
    M.interest_by_width;
  (* every delta at width 1 *)
  List.iter
    (fun d ->
      let out = M.byte_arithmetic "\x64" B.zero (big d) one M.Little in
      check_i "delta applied" (100 + d) (Char.code out.[0]))
    M.deltas

(* seedFor: the seed whose FIRST SplitMix64 word is w (the inverse finaliser,
   derived with an independent Python implementation and cross-checked against
   the Ruby lane). *)
let seed_for = function
  | 0 -> "7046029254386353131"
  | 1 -> "17885559969949501885"
  | 2 -> "10278346628982968224"
  | 3 -> "9697084067704644217"
  | _ -> Alcotest.fail "no seed"

let test_rejection_draws () =
  let seed w = Int64.of_string ("0u" ^ seed_for w) in
  List.iter
    (fun w ->
      check_s "first word" (string_of_int w)
        (Printf.sprintf "%Lu" (Prng.next_u64 (Prng.create (seed w)))))
    [ 0; 1; 2; 3 ];
  (* 2^64 mod 7 = 2: raw words 0 and 1 are redrawn, 2 is kept *)
  check_s "threshold" "2" (Printf.sprintf "%Lu" (Prng.default_threshold 7L));
  let probe = Prng.create (seed 0) in
  ignore (Prng.next_u64 probe);
  let w1 = Prng.next_u64 probe in
  let r = Prng.create (seed 0) in
  let v = Prng.bounded_u64 r 7L in
  check_s "redrawn from the second word"
    (Printf.sprintf "%Lu" (Int64.unsigned_rem w1 7L))
    (Printf.sprintf "%Lu" v);
  check_s "consumed two words"
    (Printf.sprintf "%Lx" (Prng.state probe))
    (Printf.sprintf "%Lx" (Prng.state r));
  check_s "equal to the threshold is kept" "2"
    (Printf.sprintf "%Lu" (Prng.bounded_u64 (Prng.create (seed 2)) 7L));
  (* visible through mutate_sequence: real values from the Ruby lane *)
  let input = unhex "000102030405" in
  let run w = hex (M.mutate_sequence input (bi (seed_for w)) (big 3) 100) in
  check_s "seed word 0" "fd030405" (run 0);
  check_s "seed word 1" "ff043605" (run 1);
  check_s "seed word 2" "000102b2020405" (run 2);
  check_s "seed word 3" "00010203a2fa" (run 3);
  (* the 2^63 + 1 boundary: threshold 2^63 - 1 *)
  let n = Int64.add Int64.min_int 1L in
  check_s "2^63+1 threshold" "9223372036854775807"
    (Printf.sprintf "%Lu" (Prng.default_threshold n));
  let below = Int64.of_string "0u16898060955550185018"
  and at = Int64.of_string "0u959135552437182909" in
  let p = Prng.create below in
  ignore (Prng.next_u64 p);
  let w1 = Prng.next_u64 p in
  check_s "below: redrawn"
    (Printf.sprintf "%Lu" (Int64.unsigned_rem w1 n))
    (Printf.sprintf "%Lu" (Prng.bounded_u64 (Prng.create below) n));
  check_s "equal: kept" "9223372036854775807"
    (Printf.sprintf "%Lu" (Prng.bounded_u64 (Prng.create at) n))

let test_sequence_work () =
  let ok = function
    | M.Work_ok (s, w) -> (s, w)
    | M.Work_exhausted w -> ("EXH", w)
  in
  Alcotest.(check (pair string int))
    "initial copy only" ("\001\002", 2)
    (ok (M.mutate_sequence_counted "\001\002" (big 5) B.zero 100 100));
  Alcotest.(check (pair string int))
    "stops at the step that would cross" ("EXH", 200)
    (ok
       (M.mutate_sequence_counted (String.make 100 'a') one (big 10)
          Fuzz_limits.max_input_bytes 250));
  Alcotest.(check (pair string int))
    "initial copy alone is over" ("EXH", 0)
    (ok
       (M.mutate_sequence_counted (String.make 100 'a') one (big 10)
          Fuzz_limits.max_input_bytes 99));
  (match
     M.mutate_sequence_counted
       (String.make Fuzz_limits.max_input_bytes 'a')
       one (big 100)
       (2 * Fuzz_limits.max_input_bytes)
       (4 * 1048576)
   with
  | M.Work_exhausted w -> check_b "bounded" true (w <= 4 * 1048576)
  | M.Work_ok (_, w) -> check_b "bounded ok" true (w <= 4 * 1048576));
  (match
     M.mutate_sequence_with_cap 250 (String.make 100 'a') one (big 10)
       Fuzz_limits.max_input_bytes
   with
  | exception M.Work_cap_exceeded (w, cap) ->
      check_b "raises with work and cap" true (w = 200 && cap = 250)
  | _ -> Alcotest.fail "no raise");
  check_i "per-call cap is 2^28" (1 lsl 28) Fuzz_limits.max_sequence_work;
  check_i "corpus aggregate is 2^24" (1 lsl 24) Fuzz_limits.corpus_sequence_work;
  raises_arg "steps" (fun () ->
      M.mutate_sequence_counted "a" one
        (big (Fuzz_limits.max_iterations + 1))
        10 10);
  raises_arg "huge steps" (fun () ->
      M.mutate_sequence_counted "a" one (B.pow2 100) 10 10);
  raises_arg "input" (fun () ->
      M.mutate_sequence_counted
        (String.make (Fuzz_limits.max_input_bytes + 1) 'a')
        one one 10 10);
  (* a cap near max_int cannot wrap the sum *)
  (match M.mutate_sequence_counted "ab" one (big 3) 100 max_int with
  | M.Work_ok (_, w) -> check_b "small" true (w < 100)
  | _ -> Alcotest.fail "unexpected");
  (* the length budget skips an insert AFTER its draws, so the stream stays aligned *)
  let grown = M.mutate_sequence "abcdefgh" (big 99) (big 400) 20 in
  check_b "max_len bounds growth" true (String.length grown <= 20);
  let g2 = M.mutate_sequence "abcdefgh" (big 99) (big 400) 20 in
  check_s "deterministic" grown g2;
  (* a negative seed is read modulo 2^64 *)
  check_s "seed mod 2^64"
    (hex (M.mutate_sequence "abcdefgh" (big (-1)) (big 5) 100))
    (hex (M.mutate_sequence "abcdefgh" (B.sub (B.pow2 64) one) (big 5) 100));
  (* an empty input is refilled with 00 *)
  check_b "empty input mutates" true
    (String.length (M.mutate_sequence "" one (big 3) 10) >= 1)

let test_bitmap_energy () =
  let module BM = Fuzz_bitmap in
  let b = BM.record (BM.record (BM.record BM.empty 3) (3 + 65536)) (-1) in
  Alcotest.(check (list int)) "covered" [ 3; 65535 ] (BM.covered_edges b);
  check_b "has reduced" true (BM.has_edge b 65539);
  check_b "has not" false (BM.has_edge b 4);
  let n1, c1, b1 = BM.admit b [ 3; 4; 5; 5 ] in
  check_b "novel" true (n1 && c1 = 2);
  let n2, c2, b2 = BM.admit b1 [ 3; 4 ] in
  check_b "none" true ((not n2) && c2 = 0);
  check_i "raw len" 8192 (String.length (BM.raw_bitmap b2));
  check_i "raw bit 10" 4 (Char.code (BM.raw_bitmap (BM.record BM.empty 10)).[1]);
  check_i "count" 4 (BM.count b2);
  let l = Int64.of_int in
  Alcotest.(check (pair bool int))
    "admit" (true, 1)
    (BM.corpus_admit [ l 1; l 2; l 3 ] [ l 2; l 4 ]);
  Alcotest.(check (pair bool int))
    "redundant" (false, 0)
    (BM.corpus_admit [ l 1; l 2; l 3 ] [ l 1; l 2 ]);
  Alcotest.(check (pair bool int))
    "unique" (true, 1)
    (BM.corpus_admit [] [ l 7; l 7; l 7 ]);
  Alcotest.(check (pair bool int))
    "unreduced" (true, 1)
    (BM.corpus_admit [ l 1 ] [ l (1 + 65536) ]);
  Alcotest.(check (pair bool int))
    "empty trace" (false, 0)
    (BM.corpus_admit [ l 1 ] []);
  Alcotest.(check (pair bool int))
    "u64 top bit" (true, 1)
    (BM.corpus_admit [] [ -1L ]);
  (* energy rows, cross-checked with the Ruby lane, including exact half-way ties *)
  List.iter
    (fun ((base, len, nw, total), want) ->
      check_i
        (Printf.sprintf "energy %d,%d,%d,%d" base len nw total)
        want
        (BM.calculate_energy base len nw total))
    [
      ((10, 4, 3, 6), 8);
      ((3, 4, 0, 0), 2);
      ((5, 4, 0, 0), 3);
      ((25, 100, 0, 0), 3);
      ((1, 4, 0, 0), 1);
      ((10, 0, 0, 0), 10);
      ((10, 1, 0, 0), 10);
      ((7, 9, 2, 5), 3);
      ((0, 4, 0, 0), 1);
    ]

let sigs =
  [
    ("panic", [ "eval"; "step" ], "4eca7ae8");
    ("segv", [ "alloc"; "write" ], "a0aa3aa2");
    ("timeout", [], "4e61ea76");
    ("oom", [], "7146a410");
    ("stale-budget-sentinel", [], "8a20b3bf");
    ("Failure", [], "2fa803f1");
    ("workers-leaked", [], "cc813cf1");
    ("ArgumentError", [ "a.rb#x" ], "c93b2322");
  ]

let test_signatures () =
  List.iter
    (fun (k, fs, want) -> check_s k want (Fuzz_signature.crash_signature k fs))
    sigs;
  check_s "fnv abc" "16654208175385433931"
    (Printf.sprintf "%Lu" (Prng.fnv1a64 "abc"));
  (* frames are NOT escaped (VER06 9.4 item 6): documented limit *)
  check_b "x:y" true
    (Fuzz_signature.duplicate_crash "k" [ "x:y" ] "k" [ "x"; "y" ]);
  check_b "a b" false (Fuzz_signature.duplicate_crash "k" [ "a" ] "k" [ "b" ]);
  check_s "sanitize" "a_b_c" (Fuzz_signature.sanitize_kind "a:b c");
  check_s "empty" "<unreadable>" (Fuzz_signature.sanitize_kind "");
  check_i "cap" 64
    (String.length (Fuzz_signature.sanitize_kind (String.make 500 'b')));
  check_s "allowed" "Foo.bar<$>/-" (Fuzz_signature.sanitize_kind "Foo.bar<$>/-");
  check_s "kind is the constructor" "Failure"
    (Fuzz_signature.kind_of (Failure "secret payload"));
  check_s "kind_of Not_found" "Not_found" (Fuzz_signature.kind_of Not_found);
  Alcotest.(check (list string))
    "vocabulary"
    [ "fuzz-crash"; "fuzz-timeout"; "fuzz-oom"; "fuzz-leak"; "fuzz-panic" ]
    Fuzz_failure_ids.all;
  List.iter
    (fun i -> check_b "registered" true (Fuzz_failure_ids.is_registered i))
    Fuzz_failure_ids.all;
  List.iter
    (fun i -> check_b "unregistered" false (Fuzz_failure_ids.is_registered i))
    [ "fuzz-bogus"; "FUZZ-CRASH"; "fuzz-crash\n"; ""; " fuzz-crash" ]

let test_tap () =
  let line t = Fuzz_signature.format_tap_failure 3 t "fuzz-crash" "3a8f19b2" in
  let e = "\xc3\xa9" in
  let tail = " [fuzz-crash:3a8f19b2]" in
  (* every expected value below came from the Ruby lane's format_tap_failure *)
  check_s "hash" ("not ok 3 - a_b" ^ tail) (line "a#b");
  check_s "space" ("not ok 3 - a_b_c" ^ tail) (line "a b c");
  check_s "lf" ("not ok 3 - a_b" ^ tail) (line "a\nb");
  check_s "cr" ("not ok 3 - a_b" ^ tail) (line "a\rb");
  check_s "bidi" ("not ok 3 - _____x" ^ tail) (line (e ^ "\xe2\x80\xae" ^ "x"));
  check_s "long"
    ("not ok 3 - " ^ String.make 128 'x' ^ tail)
    (line (String.make 200 'x'));
  check_s "cut mid-character"
    ("not ok 3 - " ^ String.make 127 'x' ^ "_" ^ tail)
    (line (String.make 127 'x' ^ e ^ "yyyyy"));
  check_s "character at the edge"
    ("not ok 3 - " ^ String.make 126 'x' ^ "__" ^ tail)
    (line (String.make 126 'x' ^ e ^ "zz"));
  check_s "all multibyte"
    ("not ok 3 - " ^ String.make 128 '_' ^ tail)
    (line (String.concat "" (List.init 70 (fun _ -> e))));
  check_s "empty" "not ok 3 -  [fuzz-crash:3a8f19b2]" (line "");
  check_s "grammar kept"
    ("not ok 3 - ct_compare.v1/x-y" ^ tail)
    (line "ct_compare.v1/x-y");
  check_s "token" "a_b" (Fuzz_signature.tap_token "a#b");
  check_s "invalid utf-8" "__" (Fuzz_signature.tap_token "\xff\xfe");
  List.iter
    (fun i ->
      check_s "line"
        (Printf.sprintf "not ok 7 - ct_compare [%s:3a8f19b2]" i)
        (Fuzz_signature.format_tap_failure 7 "ct_compare" i "3a8f19b2"))
    Fuzz_failure_ids.all;
  List.iter
    (fun i ->
      raises_arg "id" (fun () ->
          Fuzz_signature.format_tap_failure 1 "t" i "3a8f19b2"))
    [ "fuzz-bogus"; "FUZZ-CRASH"; "fuzz-crash\n"; "" ];
  List.iter
    (fun s ->
      raises_arg "sig" (fun () ->
          Fuzz_signature.format_tap_failure 1 "t" "fuzz-crash" s))
    [ "3A8F19B2"; "3a8f19b"; "3a8f19b22"; ""; "3a8f19b2\n"; "zzzzzzzz" ];
  List.iter
    (fun n ->
      raises_arg "n" (fun () ->
          Fuzz_signature.format_tap_failure n "t" "fuzz-crash" "3a8f19b2"))
    [ 0; 1 lsl 31; -1 ]

let mk_report ?(crashes = []) ?(timeouts = 0) ?(ooms = 0) ?(leaked = 0)
    ?(iterations = 5) () : T.report =
  {
    iterations;
    admitted_seeds = 1;
    total_edges = 0;
    crashes;
    timeouts;
    ooms;
    crashes_dropped = 0;
    admissions_dropped = 0;
    coverage_tier = T.tier_black_box;
    seed = 7L;
    corpus = [ "a" ];
    leaked_workers = leaked;
  }

let mk_crash ?(input = "x") ?(hits = 2) ?(dropped = false) sg : T.crash_record =
  {
    signature = sg;
    kind = "panic";
    frames = [ "eval"; "step" ];
    minimal_input = input;
    hits;
    minimized = true;
    input_dropped = dropped;
  }

let test_report_shapes () =
  let report =
    mk_report ~crashes:[ mk_crash "4eca7ae8" ] ~timeouts:1 ~ooms:2 ()
  in
  Alcotest.(check (list string))
    "lines"
    [
      "not ok 3 - t [fuzz-crash:4eca7ae8]";
      "not ok 4 - t [fuzz-timeout:4e61ea76]";
      "not ok 5 - t [fuzz-oom:7146a410]";
    ]
    (Fuzz_signature.tap_failures report "t" 3);
  let leaked = mk_report ~leaked:4 ~timeouts:1 () in
  Alcotest.(check (list string))
    "leaked lines"
    [
      "not ok 1 - t [fuzz-timeout:4e61ea76]";
      "not ok 2 - t [fuzz-timeout:cc813cf1]";
    ]
    (Fuzz_signature.tap_failures leaked "t" 1);
  let clean = mk_report () in
  Alcotest.(check (list string))
    "clean" []
    (Fuzz_signature.tap_failures clean "t" 1);
  check_b "isClean" true (T.is_clean clean);
  check_b "not clean" false (T.is_clean report);
  check_i "unique" 1 (T.unique_crashes report);
  check_b "zero iterations" false (T.is_clean (mk_report ~iterations:0 ()));
  check_b "dropped" false (T.is_clean { clean with crashes_dropped = 1 });
  check_b "admission dropped" false
    (T.is_clean { clean with admissions_dropped = 1 });
  check_b "leaked" false (T.is_clean (mk_report ~leaked:1 ()));
  check_s "report show hides data"
    "FuzzReport<iterations=5 seeds=1 edges=0 crashes=1 timeouts=1 ooms=2>"
    (T.show_report report);
  check_s "crash show hides input" "CrashRecord<4eca7ae8 panic hits=2>"
    (T.show_crash (mk_crash ~input:"secret" "4eca7ae8"));
  check_s "options show" "FuzzOptions<max_iterations=1000>"
    (T.show_options T.default_options);
  let d = T.default_options in
  T.validate_options d;
  List.iter
    (fun (name, o) -> raises_arg name (fun () -> T.validate_options o))
    [
      ("iter", { d with max_iterations = -1 });
      ("iter hi", { d with max_iterations = Fuzz_limits.max_iterations + 1 });
      ("len", { d with max_input_len = Fuzz_limits.max_input_bytes + 1 });
      ("energy", { d with base_energy = -1 });
      ( "minimize",
        { d with minimize_budget = Fuzz_limits.max_minimize_budget + 1 } );
      ("steps", { d with step_budget = Fuzz_limits.max_step_budget + 1 });
      ("mem", { d with mem_budget = Fuzz_limits.max_mem_budget + 1 });
      ("mem zero", { d with mem_budget = 0 });
      ( "mem below the engine's own baseline",
        { d with mem_budget = Fuzz_limits.min_mem_budget - 1 } );
      ("timeout", { d with timeout_ms = 0 });
      ("timeout hi", { d with timeout_ms = Fuzz_limits.max_timeout_ms + 1 });
    ]

let hx = unhex

(* A probe that records what it was asked to run. *)
let trace_probe pred =
  let seen = ref [] in
  ( (fun c ->
      seen := hex c :: !seen;
      if pred c then T.Crash ("K", [ "f" ]) else T.Pass),
    fun () -> List.rev !seen )

let sig_k = Fuzz_signature.crash_signature "K" [ "f" ]

let test_minimizer () =
  let module E = Fuzz_engine in
  let probe, seen = trace_probe (fun c -> String.contains c '\005') in
  let out, spent, fin = E.minimize (hx "000102030405060708") sig_k probe 1000 in
  check_s "result" "05" (hex out);
  check_i "spent" 8 spent;
  check_b "finished" true fin;
  (* cross-checked with the Ruby lane: chunks 4, 2, 1, then the zeroing pass *)
  Alcotest.(check (list string))
    "trace"
    [ "0405060708"; "08"; "04050607"; "0607"; "0405"; "05"; ""; "00" ]
    (seen ());
  let probe, seen = trace_probe (fun _ -> true) in
  let out, spent, fin = E.minimize (hx "07") sig_k probe 1000 in
  check_s "empty reachable" "" out;
  check_i "spent 1" 1 spent;
  check_b "fin" true fin;
  Alcotest.(check (list string)) "trace 1" [ "" ] (seen ());
  let probe, seen = trace_probe (fun c -> String.length c >= 2) in
  let out, spent, _ = E.minimize (hx "01020304") sig_k probe 1000 in
  check_s "zeroed" "0000" (hex out);
  check_i "spent 6" 6 spent;
  Alcotest.(check (list string))
    "trace zero"
    [ "0304"; ""; "04"; "03"; "0004"; "0000" ]
    (seen ());
  (* the budget counts rejected candidates *)
  let probe, seen = trace_probe (fun c -> String.contains c '\005') in
  let out, spent, fin = E.minimize (hx "000102030405060708") sig_k probe 3 in
  check_s "budget result" "04050607" (hex out);
  check_i "budget spent" 3 spent;
  check_b "unfinished" false fin;
  Alcotest.(check (list string))
    "budget trace"
    [ "0405060708"; "08"; "04050607" ]
    (seen ());
  let probe, seen = trace_probe (fun _ -> false) in
  let _, spent, fin = E.minimize (String.make 64 '\009') sig_k probe 4 in
  check_i "all rejected" 4 spent;
  check_b "unfinished 2" false fin;
  check_i "four probes" 4 (List.length (seen ()));
  let probe, _ = trace_probe (fun _ -> true) in
  Alcotest.(check (triple string int bool))
    "empty" ("", 0, true)
    (E.minimize "" sig_k probe 0);
  Alcotest.(check (triple string int bool))
    "zero budget" ("\001", 0, false)
    (E.minimize "\001" sig_k probe 0);
  (* the exact signature only: another crash is never accepted *)
  let other c =
    if String.length c >= 3 then T.Crash ("K", [ "f" ])
    else T.Crash ("OTHER", [ "f" ])
  in
  let out, _, fin = E.minimize (hx "0a0b0c0d") sig_k other 1000 in
  check_i "len 3" 3 (String.length out);
  check_b "finished" true fin;
  (* halting stops every probe *)
  let calls = ref 0 in
  let _, spent, fin =
    E.minimize_abortable
      (fun () -> true)
      "abcdef" sig_k
      (fun _ ->
        incr calls;
        T.Pass)
      100
  in
  check_i "halted before any probe" 0 !calls;
  check_b "halted spent" true (spent = 0 && not fin);
  let halt_after = ref 2 in
  let calls = ref 0 in
  let _, _, fin =
    E.minimize_abortable
      (fun () ->
        decr halt_after;
        !halt_after < 0)
      "\001\002\003\004" sig_k
      (fun _ ->
        incr calls;
        T.Pass)
      100
  in
  check_b "halted in zeroing or deleting" true ((not fin) && !calls = 2);
  (* the validated entry point *)
  let probe _ = T.Pass in
  check_s "unchanged" "abc" (E.minimize_crashing_input "abc" sig_k 10 probe);
  raises_arg "negative" (fun () ->
      E.minimize_crashing_input "abc" sig_k (-1) probe);
  raises_arg "too big" (fun () ->
      E.minimize_crashing_input "abc" sig_k
        (Fuzz_limits.max_minimize_budget + 1)
        probe);
  raises_arg "input" (fun () ->
      E.minimize_crashing_input
        (String.make (Fuzz_limits.max_input_bytes + 1) 'a')
        sig_k 1 probe)

(* ===================================================================== *)
(* Part 3: the handle, the executor, the campaign                        *)
(* ===================================================================== *)

let fresh ?(steps = 10) ?(mem = 100) () =
  HI.create ~step_budget:steps ~mem_budget:mem

let is_sentinel which = function H.Sentinel (_, w) -> w = which | _ -> false

let expect_sentinel name which f =
  match f () with
  | exception e when is_sentinel which e -> ()
  | exception e ->
      Alcotest.failf "%s: unexpected %s" name (Printexc.to_string e)
  | _ -> Alcotest.failf "%s: no sentinel" name

let test_handle_basics () =
  let ih = fresh () in
  let h = H.wrap ih in
  H.edge h 3;
  H.edge h (3 + 65536);
  H.edge h (-1);
  H.edge h (H.edge_id "x");
  Alcotest.(check (list int))
    "edges reduced"
    (List.sort_uniq compare [ 3; 65535; H.edge_id "x" land 0xFFFF ])
    (HI.edges ih);
  check_i "edge calls" 4 (HI.edge_calls ih);
  check_i "edge_id is the low bits of fnv1a"
    (Int64.to_int (Prng.fnv1a64 "abc") land 0xFFFF)
    (H.edge_id "abc" land 0xFFFF);
  H.tick h 4;
  H.tick h 6;
  check_i "steps used" 10 (HI.steps_used ih);
  check_b "exactly at the budget is fine" true (HI.exhausted ih = HI.Ex_none);
  expect_sentinel "over" H.Steps (fun () -> H.tick h 1);
  check_b "flag" true (HI.exhausted ih = HI.Ex_steps);
  (* negative charges nothing, a huge one is clamped *)
  let ih = fresh () in
  let h = H.wrap ih in
  H.tick h (-5);
  H.reserve h (-5);
  check_i "negative" 0 (HI.steps_used ih);
  expect_sentinel "huge" H.Steps (fun () -> H.tick h max_int);
  check_b "clamped to budget + 1" true (HI.steps_used ih = 11);
  let ih = fresh () in
  expect_sentinel "huge mem" H.Mem (fun () -> H.reserve (H.wrap ih) max_int);
  check_i "mem clamped" 101 (HI.mem_reserved ih);
  raises_arg "budget" (fun () -> HI.create ~step_budget:(-1) ~mem_budget:0);
  raises_arg "budget hi" (fun () ->
      HI.create ~step_budget:0 ~mem_budget:(Fuzz_limits.max_mem_budget + 1));
  (* an armed handle read from another domain reads nothing (best effort) *)
  let armed = fresh () in
  HI.arm armed;
  let d =
    Domain.spawn (fun () ->
        H.tick (H.wrap armed) 1;
        HI.observe armed;
        HI.alloc_bytes armed)
  in
  ignore (Domain.join d);
  check_i "armed domain only: steps still charged" 1 (HI.steps_used armed);
  check_b "tokens differ" false
    (HI.same_token (H.token h) (H.token (H.wrap (fresh ()))));
  check_b "token equal to itself" true (HI.same_token (H.token h) (H.token h))

(* VER06r 5.2: sticky in BOTH directions *)
let test_sticky_both_ways () =
  let ih = fresh ~steps:5 ~mem:50 () in
  let h = H.wrap ih in
  expect_sentinel "steps breach" H.Steps (fun () -> H.tick h 6);
  expect_sentinel "reserve 0 after steps" H.Steps (fun () -> H.reserve h 0);
  expect_sentinel "reserve 1 after steps" H.Steps (fun () -> H.reserve h 1);
  expect_sentinel "tick 0 after steps" H.Steps (fun () -> H.tick h 0);
  let ih = fresh ~steps:5 ~mem:50 () in
  let h = H.wrap ih in
  expect_sentinel "mem breach" H.Mem (fun () -> H.reserve h 51);
  expect_sentinel "tick 0 after mem" H.Mem (fun () -> H.tick h 0);
  expect_sentinel "tick 1 after mem" H.Mem (fun () -> H.tick h 1);
  expect_sentinel "reserve 0 after mem" H.Mem (fun () -> H.reserve h 0);
  check_b "first breach wins" true (HI.exhausted ih = HI.Ex_mem);
  check_i "no charge after the breach" 0 (HI.steps_used ih)

let test_retired_forged_inert () =
  let ih = fresh () in
  let h = H.wrap ih in
  H.edge h 1;
  check_b "retire returns was-live" true (HI.retire ih);
  check_b "second retire" false (HI.retire ih);
  check_b "is_retired" true (HI.is_retired ih);
  H.edge h 2;
  Alcotest.(check (list int))
    "edge after retirement is a no-op" [ 1 ] (HI.edges ih);
  expect_sentinel "tick retired" H.Retired (fun () -> H.tick h 0);
  expect_sentinel "reserve retired" H.Retired (fun () -> H.reserve h 0);
  check_i "no charge" 0 (HI.steps_used ih);
  (* the sentinel carries the handle's token *)
  (match H.tick h 0 with
  | exception H.Sentinel (tok, _) ->
      check_b "its token" true (HI.same_token tok (H.token h))
  | _ -> Alcotest.fail "no sentinel");
  (* forged: shares the state, claims another token *)
  let live = fresh () in
  let forged = H.wrap (HI.forge live (HI.mint_token ())) in
  H.edge forged 9;
  Alcotest.(check (list int)) "forged edge is a no-op" [] (HI.edges live);
  expect_sentinel "forged tick" H.Retired (fun () -> H.tick forged 1);
  expect_sentinel "forged reserve" H.Retired (fun () -> H.reserve forged 1);
  check_i "forged never charged" 0 (HI.steps_used live);
  (* the genuine handle still works *)
  H.tick (H.wrap live) 1;
  check_i "genuine charged" 1 (HI.steps_used live);
  (* retained across domains after retirement *)
  let ih = fresh () in
  ignore (HI.retire ih);
  let d =
    Domain.spawn (fun () ->
        H.edge (H.wrap ih) 5;
        match H.tick (H.wrap ih) 1 with
        | exception H.Sentinel (_, H.Retired) -> true
        | _ -> false)
  in
  check_b "inert in another domain" true (Domain.join d);
  Alcotest.(check (list int)) "no edges" [] (HI.edges ih)

(* a target that clears the flag after a breach is still judged by the counters *)
let test_cross_check () =
  let o = { T.default_options with step_budget = 10 } in
  let target ih_ref =
   fun _ h ->
    (try H.tick h 100 with H.Sentinel _ -> ());
    match !ih_ref with Some ih -> HI.clear_exhausted ih | None -> ()
  in
  let run cross_check =
    let ih_ref = ref None in
    let r =
      Fuzz_executor.run ~cross_check
        ~on_handle:(fun ih -> ih_ref := Some ih)
        (target ih_ref) "x" o
    in
    r.outcome
  in
  check_b "cross-check on: timeout" true (run true = T.Timeout);
  (* the control: with the cross-check removed the same run PASSES, so the test
     above fails if the cross-check is ever removed *)
  check_b "cross-check off: pass" true (run false = T.Pass);
  (* the same for memory *)
  let o = { T.default_options with mem_budget = Fuzz_limits.min_mem_budget } in
  let run cross_check =
    let ih_ref = ref None in
    let t _ h =
      (try H.reserve h 1_000_000 with H.Sentinel _ -> ());
      match !ih_ref with Some ih -> HI.clear_exhausted ih | None -> ()
    in
    (Fuzz_executor.run ~cross_check
       ~on_handle:(fun ih -> ih_ref := Some ih)
       t "x" o)
      .outcome
  in
  check_b "mem cross-check on: oom" true (run true = T.Oom);
  check_b "mem cross-check off: pass" true (run false = T.Pass)

let with_release f =
  let release = Atomic.make false in
  Fun.protect ~finally:(fun () -> Atomic.set release true) (fun () -> f release)

(* A blocked worker SLEEPS: a spinning one would steal CPU from the timed tests
   (and slow stop-the-world GC to a crawl with 100+ domains on a small runner).
   It still never calls the handle, so it is a non-cooperating target. *)
let spin_until flag =
  while not (Atomic.get flag) do
    Unix.sleepf 0.001
  done

(* honest targets finish in microseconds; the margin keeps a slow one-core runner
   from flaking, and the tests that NEED a timeout wait for it anyway *)
let quick = { T.default_options with timeout_ms = 300 }
let leaks_of () = Fuzz_executor.new_leaks ()

let exec_once ?(o = T.default_options) target input =
  Fuzz_executor.run target input o

let test_executor_outcomes () =
  let r =
    exec_once
      (fun _ h ->
        H.edge h 7;
        H.edge h 7)
      "in"
  in
  check_b "pass" true (r.outcome = T.Pass);
  Alcotest.(check (list int)) "edges" [ 7 ] r.edges;
  check_i "calls" 2 r.edge_calls;
  check_b "not leaked" false r.leaked;
  (* the target sees its input *)
  let seen = ref "" in
  ignore (exec_once (fun d _ -> seen := d) "payload");
  check_s "input" "payload" !seen;
  (* crash kinds come from the constructor only *)
  let r = exec_once (fun _ _ -> failwith "secret payload") "" in
  check_b "crash kind" true (r.outcome = T.Crash ("Failure", []));
  let exception Boom of string in
  let r = exec_once (fun _ _ -> raise (Boom "secret")) "" in
  (match r.outcome with
  | T.Crash (k, []) ->
      check_b "constructor named" true (contains k "Boom");
      check_b "no payload" false (contains k "secret")
  | _ -> Alcotest.fail "expected a crash");
  (* a hostile printer is never run *)
  let exception Hostile in
  Printexc.register_printer (function
    | Hostile -> failwith "printer ran"
    | _ -> None);
  let r = exec_once (fun _ _ -> raise Hostile) "" in
  (match r.outcome with
  | T.Crash (k, _) -> check_b "hostile kind" true (contains k "Hostile")
  | _ -> Alcotest.fail "crash");
  check_b "Stack_overflow is a crash kind" true
    ((exec_once (fun _ _ -> raise Stack_overflow) "").outcome
    = T.Crash ("Stack_overflow", []));
  (* Sys.Break and Out_of_memory are never a verdict *)
  (match exec_once (fun _ _ -> raise Sys.Break) "" with
  | exception Sys.Break -> ()
  | _ -> Alcotest.fail "Sys.Break swallowed");
  (match exec_once (fun _ _ -> raise Out_of_memory) "" with
  | exception Out_of_memory -> ()
  | _ -> Alcotest.fail "Out_of_memory swallowed");
  match
    exec_once
      (fun _ _ ->
        try raise Out_of_memory with e -> raise (Fun.Finally_raised e))
      ""
  with
  | exception Out_of_memory -> ()
  | _ -> Alcotest.fail "wrapped Out_of_memory swallowed"

let test_executor_budgets () =
  let o = { T.default_options with step_budget = 10; mem_budget = 1_000_000 } in
  (* the sentinel propagates *)
  check_b "steps" true
    ((exec_once ~o
        (fun _ h ->
          for _ = 1 to 100 do
            H.tick h 1
          done)
        "")
       .outcome = T.Timeout);
  (* the target swallows it and returns, or raises something ordinary, or exits *)
  check_b "swallowed then returned" true
    ((exec_once ~o (fun _ h -> try H.tick h 100 with H.Sentinel _ -> ()) "")
       .outcome = T.Timeout);
  check_b "swallowed then ordinary raise" true
    ((exec_once ~o
        (fun _ h -> try H.tick h 100 with H.Sentinel _ -> failwith "later")
        "")
       .outcome = T.Timeout);
  check_b "swallowed then exit" true
    ((exec_once ~o
        (fun _ h -> try H.tick h 100 with H.Sentinel _ -> raise Exit)
        "")
       .outcome = T.Timeout);
  check_b "mem" true
    ((exec_once ~o (fun _ h -> H.reserve h 5_000_000) "").outcome = T.Oom);
  check_b "mem swallowed" true
    ((exec_once ~o
        (fun _ h -> try H.reserve h 5_000_000 with H.Sentinel _ -> ())
        "")
       .outcome = T.Oom);
  (* sticky across the two budgets, judged by whichever breached first *)
  check_b "steps first then reserve" true
    ((exec_once ~o
        (fun _ h ->
          (try H.tick h 100 with H.Sentinel _ -> ());
          try H.reserve h 1 with H.Sentinel _ -> ())
        "")
       .outcome = T.Timeout);
  check_b "mem first then tick" true
    ((exec_once ~o
        (fun _ h ->
          (try H.reserve h 5_000_000 with H.Sentinel _ -> ());
          try H.tick h 1 with H.Sentinel _ -> ())
        "")
       .outcome = T.Oom);
  (* allocation is measured: a target that allocates and never ticks is an oom *)
  let om = { T.default_options with mem_budget = 1 lsl 20 } in
  check_b "allocation bomb, no tick" true
    ((exec_once ~o:om
        (fun _ _ -> ignore (Sys.opaque_identity (Bytes.create (8 lsl 20))))
        "")
       .outcome = T.Oom);
  (* ... and one that ticks sees the sentinel at the next call *)
  check_b "allocation seen at tick" true
    ((exec_once ~o:om
        (fun _ h ->
          ignore (Sys.opaque_identity (Bytes.create (8 lsl 20)));
          H.tick h 0)
        "")
       .outcome = T.Oom);
  check_b "allocation seen at reserve" true
    ((exec_once ~o:om
        (fun _ h ->
          ignore (Sys.opaque_identity (Bytes.create (8 lsl 20)));
          H.reserve h 0)
        "")
       .outcome = T.Oom);
  (* a modest allocation passes *)
  check_b "modest" true
    ((exec_once ~o:om
        (fun _ h ->
          ignore (Sys.opaque_identity (Bytes.create 1000));
          H.tick h 1)
        "")
       .outcome = T.Pass);
  (* the baseline is per execution: a big first run does not poison the next *)
  check_b "fresh baseline" true
    ((exec_once ~o:om (fun _ _ -> ()) "").outcome = T.Pass)

let test_executor_stale_sentinels () =
  let kept = ref None in
  ignore (exec_once (fun _ h -> kept := Some h) "");
  let old =
    match !kept with Some h -> h | None -> Alcotest.fail "no handle"
  in
  (* a retained handle used by a later execution is inert and its sentinel is stale *)
  let r = exec_once (fun _ _ -> H.tick old 1) "" in
  check_b "retained tick is a stale crash" true
    (r.outcome = T.Crash (Fuzz_executor.stale_kind, []));
  let r = exec_once (fun _ _ -> H.edge old 1) "" in
  check_b "retained edge is a no-op" true (r.outcome = T.Pass && r.edges = []);
  (* a forged sentinel with a retained token *)
  let r = exec_once (fun _ _ -> raise (H.Sentinel (H.token old, H.Steps))) "" in
  check_b "forged steps sentinel is a crash" true
    (r.outcome = T.Crash (Fuzz_executor.stale_kind, []));
  let r = exec_once (fun _ h -> raise (H.Sentinel (H.token h, H.Mem))) "" in
  check_b "own-token forgery without a breach is a crash" true
    (r.outcome = T.Crash (Fuzz_executor.stale_kind, []));
  (* a forged handle given to the target *)
  let r =
    Fuzz_executor.run
      ~on_handle:(fun _ -> ())
      (fun _ h ->
        H.edge h 1;
        H.tick h 1)
      "" T.default_options
  in
  check_b "control: genuine handle" true (r.outcome = T.Pass);
  (* forwarded to another domain and used after the run *)
  let box = Atomic.make None in
  ignore (exec_once (fun _ h -> Atomic.set box (Some h)) "");
  let h = Option.get (Atomic.get box) in
  let d =
    Domain.spawn (fun () ->
        H.edge h 3;
        match H.tick h 1 with
        | exception H.Sentinel (_, H.Retired) -> true
        | _ -> false)
  in
  check_b "forwarded handle inert" true (Domain.join d);
  (* an execution's edges are its own: a retained handle cannot write into the next *)
  let r = exec_once (fun _ h -> H.edge h 11) "" in
  Alcotest.(check (list int)) "own edges only" [ 11 ] r.edges

let test_executor_runaway () =
  (* a cooperating runaway unwinds at its next handle call once retired *)
  let o = { quick with step_budget = Fuzz_limits.max_step_budget } in
  let leaks = leaks_of () in
  let r =
    Fuzz_executor.run_tracked leaks
      (fun _ h ->
        while true do
          H.tick h 0;
          Domain.cpu_relax ()
        done)
      "" o
  in
  check_b "timeout" true (r.outcome = T.Timeout);
  check_b "unwound, not leaked" false r.leaked;
  check_i "no leak counted" 0 (Atomic.get leaks);
  (* a non-cooperating loop cannot be stopped: reported, leaked, counted *)
  let release = Atomic.make false in
  Fun.protect
    ~finally:(fun () -> Atomic.set release true)
    (fun () ->
      let leaks = leaks_of () in
      let t0 = Sys.time () in
      let r =
        Fuzz_executor.run_tracked ~kill_wait_ms:30 ~leaked_kill_wait_ms:10 leaks
          (fun _ _ -> spin_until release)
          "" quick
      in
      check_b "timeout" true (r.outcome = T.Timeout);
      check_b "leaked" true r.leaked;
      check_i "counted" 1 (Atomic.get leaks);
      (* the second leak waits the shorter time *)
      let r2 =
        Fuzz_executor.run_tracked ~kill_wait_ms:30 ~leaked_kill_wait_ms:10 leaks
          (fun _ _ -> spin_until release)
          "" quick
      in
      check_b "second leaked" true r2.leaked;
      check_i "two" 2 (Atomic.get leaks);
      check_b "bounded" true (Sys.time () -. t0 < 20.0));
  (* a late answer after the deadline is ignored: nothing it does reaches the run *)
  with_release (fun release ->
      let r =
        Fuzz_executor.run ~kill_wait_ms:20 ~leaked_kill_wait_ms:5
          (fun _ h ->
            spin_until release;
            H.edge h 5)
          "" quick
      in
      check_b "late worker leaked" true (r.leaked && r.edges = []))

(* M1: the handle's own bookkeeping must not be charged to an honest target *)
let test_hot_path_is_not_charged () =
  let o =
    {
      T.default_options with
      step_budget = Fuzz_limits.max_step_budget;
      timeout_ms = 60_000;
    }
  in
  let n = 5_000_000 in
  let r =
    Fuzz_executor.run
      (fun _ h ->
        for _ = 1 to n do
          H.tick h 1
        done)
      "" o
  in
  check_b "5M ticks under 256 MiB pass" true (r.outcome = T.Pass);
  let ih_ref = ref None in
  let r =
    Fuzz_executor.run
      ~on_handle:(fun ih -> ih_ref := Some ih)
      (fun _ h ->
        for _ = 1 to n do
          H.reserve h 1
        done)
      "" o
  in
  check_b "5M reserves pass" true (r.outcome = T.Pass);
  (match !ih_ref with
  | Some ih ->
      check_i "reserved bytes" n (HI.mem_reserved ih);
      check_b "the bookkeeping allocated a few bytes per call, not 150" true
        (HI.alloc_bytes ih < 16_000_000)
  | None -> Alcotest.fail "no handle");
  let r3 =
    Fuzz_executor.run
      (fun _ h ->
        for i = 1 to n do
          H.edge h i
        done)
      "" o
  in
  check_b "5M edges pass" true (r3.outcome = T.Pass && r3.edge_calls = n);
  ignore r;
  (* the sampler is net of its own cost: with the smallest legal budget and NO
     allocation by the target, millions of calls still pass (the reading's own
     allocation used to accumulate: about 1.25 bytes per tick) *)
  let floor = { o with mem_budget = Fuzz_limits.min_mem_budget } in
  check_b "5M ticks under 64 KiB pass" true
    ((Fuzz_executor.run
        (fun _ h ->
          for _ = 1 to n do
            H.tick h 1
          done)
        "" floor)
       .outcome = T.Pass);
  check_b "5M reserves of 0 under 64 KiB pass" true
    ((Fuzz_executor.run
        (fun _ h ->
          for _ = 1 to n do
            H.reserve h 0
          done)
        "" floor)
       .outcome = T.Pass);
  (* and it still sees real allocation *)
  check_b "real allocation still counts" true
    ((Fuzz_executor.run
        (fun _ h ->
          ignore (Sys.opaque_identity (Bytes.create 200_000));
          H.tick h 0)
        "" floor)
       .outcome = T.Oom)

(* M3: a wall clock, so neither the waiter's spinning nor leaked spinners count *)
let test_honest_target_under_leaks () =
  with_release (fun release ->
      let leaks = leaks_of () in
      for _ = 1 to 2 do
        let r =
          Fuzz_executor.run_tracked ~kill_wait_ms:20 ~leaked_kill_wait_ms:5
            leaks
            (fun _ _ -> spin_until release)
            ""
            { T.default_options with timeout_ms = 60 }
        in
        check_b "spinner leaked" true r.leaked
      done;
      let o = { T.default_options with timeout_ms = 1000 } in
      (* sleeping for 30% of the budget *)
      let r =
        Fuzz_executor.run_tracked leaks (fun _ _ -> Unix.sleepf 0.3) "" o
      in
      check_b "sleeper passes" true (r.outcome = T.Pass);
      (* busy for 60% of the budget, with two leaked spinners burning CPU *)
      let r =
        Fuzz_executor.run_tracked leaks
          (fun _ _ ->
            let t0 = Unix.gettimeofday () in
            while Unix.gettimeofday () -. t0 < 0.6 do
              ()
            done)
          "" o
      in
      check_b "busy-worker passes" true (r.outcome = T.Pass);
      (* ... and a target past its budget still times out *)
      let r =
        Fuzz_executor.run_tracked leaks
          (fun _ h ->
            while true do
              Unix.sleepf 0.001;
              H.tick h 0
            done)
          ""
          { T.default_options with timeout_ms = 100 }
      in
      check_b "slow target times out" true
        (r.outcome = T.Timeout && not r.leaked))

(* F2: the wait is bounded even when the clock is stepped *)
let test_wait_clock_steps () =
  let run ?(fill_at = max_int) step_at step =
    let clock = ref 1000. and sleeps = ref 0 in
    let cell = Atomic.make None in
    let sleep d =
      incr sleeps;
      clock := !clock +. d;
      if !sleeps = step_at then clock := !clock +. step;
      if !sleeps = fill_at then Atomic.set cell (Some Fuzz_executor.Done)
    in
    let r = Fuzz_executor.wait_with ~now:(fun () -> !clock) ~sleep cell 1000 in
    (r, !sleeps)
  in
  let r, n = run 0 0. in
  check_b "no step: times out" true (r = None);
  check_b "after about a second of 1 ms sleeps" true (n > 900 && n < 1100);
  (* a one-hour step BACK would hang a wall-clock-only wait for an hour *)
  let r, n = run 50 (-3600.) in
  check_b "backward step: still times out" true (r = None);
  check_b "backward step: bounded by the sleeps" true (n < 1300);
  (* a one-hour step FORWARD does not cut it short *)
  let r, n = run 50 3600. in
  check_b "forward step: times out" true (r = None);
  check_b "forward step: not early" true (n > 900);
  let r, _ = run ~fill_at:10 0 0. in
  check_b "an answer ends the wait" true (r = Some Fuzz_executor.Done)

(* M2 and L1: a process-wide cap, a typed error at the domain ceiling *)
let test_global_leak_cap () =
  (* an offset, not a snapshot: a leaked worker of an earlier test may still be
     returning and subtracting itself while this test runs *)
  let applied = ref 0 in
  let set_to target =
    let delta = target - Atomic.get Fuzz_executor.global_leaks in
    ignore (Atomic.fetch_and_add Fuzz_executor.global_leaks delta);
    applied := !applied + delta
  in
  Fun.protect
    ~finally:(fun () ->
      ignore (Atomic.fetch_and_add Fuzz_executor.global_leaks (- !applied)))
    (fun () ->
      (* with a margin: workers leaked by earlier tests may still be returning
         and subtracting themselves *)
      set_to (Fuzz_limits.max_global_leaked_workers + 20);
      let refused name f =
        match f () with
        | exception Error.Engine_error m ->
            check_b name true (contains m "leaked")
        | _ -> Alcotest.failf "%s: not refused" name
      in
      refused "run_tracked" (fun () -> exec_once (fun _ _ -> ()) "");
      refused "Fuzz.run" (fun () ->
          Fuzz.run [ "a" ] T.default_options (fun _ _ -> ()));
      refused "verdict_for" (fun () ->
          Fuzz.verdict_for
            (fun _ _ -> ())
            (mk_report ~crashes:[ mk_crash "00000000" ] ())
            T.default_options "t");
      set_to (Fuzz_limits.max_global_leaked_workers - 1);
      check_b "below the cap it runs" true
        ((exec_once (fun _ _ -> ()) "").outcome = T.Pass));
  (* a leaked worker that finally returns is subtracted *)
  with_release (fun release ->
      let r =
        Fuzz_executor.run ~kill_wait_ms:20 ~leaked_kill_wait_ms:5
          (fun _ _ -> spin_until release)
          ""
          { T.default_options with timeout_ms = 50 }
      in
      check_b "leaked" true r.leaked;
      (* workers leaked by earlier tests may be returning at the same time, so
         only "at least ours" is exact here *)
      check_b "counted" true (Atomic.get Fuzz_executor.global_leaks >= 1);
      Atomic.set release true;
      (* every earlier test released its workers too: once all have returned
         and subtracted themselves the count is back to zero *)
      let spins = ref 0 in
      while Atomic.get Fuzz_executor.global_leaks > 0 && !spins < 5000 do
        Unix.sleepf 0.001;
        incr spins
      done;
      check_i "subtracted when they returned" 0
        (Atomic.get Fuzz_executor.global_leaks))

let test_domain_ceiling () =
  with_release (fun release ->
      let held = ref [] in
      (try
         for _ = 1 to 200 do
           held := Domain.spawn (fun () -> spin_until release) :: !held
         done
       with Failure _ -> ());
      check_b "the runtime refused a domain" true (List.length !held < 200);
      (match exec_once (fun _ _ -> ()) "" with
      | exception Error.Engine_error m ->
          check_b "typed error names the ceiling" true (contains m "domain")
      | _ -> Alcotest.fail "no error at the domain ceiling");
      Atomic.set release true;
      List.iter Domain.join !held;
      check_b "works again once the domains are gone" true
        ((exec_once (fun _ _ -> ()) "").outcome = T.Pass))

(* --- the whole-campaign cross-check against the Ruby lane --------------- *)

let cross_exec data =
  let b = List.of_seq (Seq.map Char.code (String.to_seq data)) in
  let n = List.length b and sum = List.fold_left ( + ) 0 b in
  let edges =
    [ 100 + (n mod 7) ]
    @ (match b with x :: _ -> [ 200 + (x mod 16) ] | [] -> [])
    @ [ 300 + (sum mod 13) ]
    @ match b with _ :: y :: _ -> [ 400 + (y mod 5) ] | _ -> []
  in
  let ff = List.length (List.filter (( = ) 255) b) in
  let out =
    if n > 40 then T.Timeout
    else if n >= 2 && sum mod 11 = 0 then T.Crash ("K1", [])
    else if ff >= 2 then T.Crash ("K2", [ "f" ])
    else T.Pass
  in
  (out, edges)

let test_campaign_cross_check () =
  let o =
    {
      T.default_options with
      max_iterations = 300;
      seed = 7L;
      max_input_len = 64;
      base_energy = 10;
      minimize_budget = 200;
    }
  in
  let r =
    Fuzz_engine.run_outcomes [ "AB"; "\000\000\000\000"; "" ] o cross_exec
  in
  (* every value below was produced by the Ruby lane (Fuzz.run_outcomes) on the
     same deterministic exec *)
  check_i "iterations" 300 r.iterations;
  check_i "seeds" 28 r.admitted_seeds;
  check_i "edges" 41 r.total_edges;
  check_i "timeouts" 0 r.timeouts;
  check_s "tier" "caller-reported" r.coverage_tier;
  (match r.crashes with
  | [ a; b ] ->
      check_s "k1 sig" "d20b74e1" a.signature;
      check_s "k1 witness" "0000" (hex a.minimal_input);
      check_i "k1 hits" 16 a.hits;
      check_b "k1 minimized" true a.minimized;
      check_s "k2 sig" "db4988b0" b.signature;
      check_s "k2 witness" "fffff3" (hex b.minimal_input);
      check_i "k2 hits" 2 b.hits
  | cs -> Alcotest.failf "expected two crashes, got %d" (List.length cs));
  check_s "corpus"
    "4142  00c0 00 5d d200 ff de 006900 747400 7e 20 7f07 a3f8 8007 5c0607 \
     af5c0607 00e900 8069ff4142 8869ff417400 d25e d6 c7fe 0092 b5 f915 ca9d \
     8b00"
    (String.concat " " (List.map hex r.corpus))

let test_campaign_contracts () =
  let o = { T.default_options with max_iterations = 200; seed = 1L } in
  let exec d = (T.Pass, [ String.length d ]) in
  let a = Fuzz_engine.run_outcomes [ "a" ] o exec in
  let b = Fuzz_engine.run_outcomes [ "a" ] o exec in
  check_b "deterministic" true (a = b);
  let c = Fuzz_engine.run_outcomes [ "a" ] { o with seed = 2L } exec in
  check_b "a different seed differs" true (a <> c);
  check_i "exact iterations" 200 a.iterations;
  check_b "clean" true (T.is_clean a);
  (* no candidate exceeds max_input_len; initial seeds run as given *)
  let longest = ref 0 in
  let seed = String.make 50 'z' in
  ignore
    (Fuzz_engine.run_outcomes [ seed ] { o with max_input_len = 10 } (fun d ->
         longest := max !longest (String.length d);
         (T.Pass, [ 1 ])));
  check_i "seeds are run as given, candidates are cut" 50 !longest;
  let lens = ref [] in
  ignore
    (Fuzz_engine.run_outcomes [ "ab" ] { o with max_input_len = 10 } (fun d ->
         lens := String.length d :: !lens;
         (T.Pass, [ 1 ])));
  check_b "candidates cut" true (List.for_all (fun l -> l <= 10) !lens);
  (* empty corpus: one 00 seed *)
  let first = ref None in
  ignore
    (Fuzz_engine.run_outcomes [] { o with max_iterations = 1 } (fun d ->
         if !first = None then first := Some d;
         (T.Pass, [])));
  check_b "00 seed" true (!first = Some "\000");
  (* zero iterations is never clean; black-box tier without edges *)
  let z = Fuzz_engine.run_outcomes [ "a" ] { o with max_iterations = 0 } exec in
  check_b "zero is not clean" false (T.is_clean z);
  let bb =
    Fuzz_engine.run_outcomes [ "a" ] { o with max_iterations = 5 } (fun _ ->
        (T.Pass, []))
  in
  check_s "black box" T.tier_black_box bb.coverage_tier;
  check_i "first passing seed still admitted" 1 bb.admitted_seeds;
  (* a corpus of crashers fuzzes nothing *)
  let cr =
    Fuzz_engine.run_outcomes [ "a"; "b" ] o (fun _ -> (T.Crash ("K", []), []))
  in
  check_i "no iterations" 0 cr.iterations;
  check_i "one record per signature" 1 (List.length cr.crashes);
  check_i "hits" 2 (List.hd cr.crashes).hits;
  (* timeouts and ooms are counted, never dropped *)
  let t = Fuzz_engine.run_outcomes [ "a" ] o (fun _ -> (T.Timeout, [])) in
  check_i "timeout counted" 1 t.timeouts;
  let m = Fuzz_engine.run_outcomes [ "a" ] o (fun _ -> (T.Oom, [])) in
  check_i "oom counted" 1 m.ooms;
  check_b "not clean" true ((not (T.is_clean t)) && not (T.is_clean m));
  (* input validation *)
  raises_arg "too many seeds" (fun () ->
      Fuzz_engine.run_outcomes
        (List.init (Fuzz_limits.max_initial_corpus + 1) (fun _ -> "a"))
        o exec);
  raises_arg "big seed" (fun () ->
      Fuzz_engine.run_outcomes
        [ String.make (Fuzz_limits.max_input_bytes + 1) 'a' ]
        o exec);
  raises_arg "bad options" (fun () ->
      Fuzz_engine.run_outcomes [ "a" ] { o with timeout_ms = 0 } exec);
  (* feedback: a staircase cracked with coverage and not without *)
  let stair d =
    let n = String.length d in
    let e = ref [] and depth = ref 0 in
    if n > 0 && d.[0] = 'F' then (
      incr depth;
      e := [ 1 ]);
    if !depth = 1 && n > 1 && d.[1] = 'U' then (
      incr depth;
      e := [ 1; 2 ]);
    if !depth = 2 && n > 2 && d.[2] = 'Z' then
      (T.Crash ("found", []), [ 1; 2; 3 ])
    else (T.Pass, !e)
  in
  let hit blind =
    let found = ref 0 in
    for s = 1 to 6 do
      let r =
        Fuzz_engine.run_outcomes [ "FAA" ]
          { o with seed = Int64.of_int s; max_iterations = 3000 }
          (if blind then fun d -> (fst (stair d), []) else stair)
      in
      if r.crashes <> [] then incr found
    done;
    !found
  in
  check_b "guided at least as good as blind" true (hit false >= hit true)

let test_campaign_caps () =
  let exec_crash_each d = (T.Crash ("k" ^ d, []), []) in
  (* the crash-signature cap *)
  let seeds = List.init 1005 (fun i -> Printf.sprintf "%04d" i) in
  let r =
    Fuzz_engine.run_outcomes seeds
      { T.default_options with max_iterations = 0; minimize_budget = 0 }
      exec_crash_each
  in
  check_i "kept" Fuzz_limits.max_crashes (List.length r.crashes);
  check_i "dropped" 5 r.crashes_dropped;
  check_b "not clean" false (T.is_clean r);
  (* the retained-bytes cap, enforced BEFORE the bytes are stored *)
  let cfg = { Fuzz_engine.default_config with retained_cap = 10 } in
  let o = { T.default_options with max_iterations = 50 } in
  let r =
    Fuzz_engine.run_outcomes_with cfg [ "123456"; "abcdef" ] o (fun d ->
        (T.Pass, [ Char.code d.[0] ]))
  in
  check_b "admissions dropped" true (r.admissions_dropped > 0);
  check_b "retained within the cap" true
    (List.fold_left (fun a s -> a + String.length s) 0 r.corpus <= 10);
  let r =
    Fuzz_engine.run_outcomes_with cfg [ "123456789012" ]
      { o with max_iterations = 0 } (fun _ -> (T.Crash ("K", []), []))
  in
  (match r.crashes with
  | [ c ] ->
      check_b "no input kept over the cap" true
        (c.input_dropped && c.minimal_input = "" && not c.minimized)
  | _ -> Alcotest.fail "one crash");
  (* campaign-wide minimisation budget counts rejected candidates *)
  let calls = ref 0 in
  let seeds = List.init 20 (fun i -> String.make 41 (Char.chr (i + 1))) in
  let r =
    Fuzz_engine.run_outcomes seeds
      { T.default_options with max_iterations = 7; minimize_budget = 1000 }
      (fun d ->
        incr calls;
        (T.Crash (("k" ^ if d = "" then "" else String.make 1 d.[0]), []), []))
  in
  check_i "20 crashes" 20 (List.length r.crashes);
  check_b "at most 20 triage + 7 minimisation executions" true (!calls <= 27);
  check_b "most are unminimised" true
    (List.length
       (List.filter (fun (c : T.crash_record) -> not c.minimized) r.crashes)
    >= 13);
  (* per-crash budget zero *)
  let r =
    Fuzz_engine.run_outcomes [ "abcd" ]
      { T.default_options with max_iterations = 0; minimize_budget = 0 }
      (fun _ -> (T.Crash ("K", []), []))
  in
  check_s "unminimised witness" "abcd" (List.hd r.crashes).minimal_input;
  (* a repeat crash keeps the smaller witness (and charges the difference) *)
  let r =
    Fuzz_engine.run_outcomes [ "abcdefgh"; "abc" ]
      { T.default_options with max_iterations = 0; minimize_budget = 0 }
      (fun _ -> (T.Crash ("K", []), []))
  in
  check_s "smaller witness" "abc" (List.hd r.crashes).minimal_input;
  check_i "hits" 2 (List.hd r.crashes).hits;
  let r =
    Fuzz_engine.run_outcomes [ "abc"; "abcdefgh" ]
      { T.default_options with max_iterations = 0; minimize_budget = 0 }
      (fun _ -> (T.Crash ("K", []), []))
  in
  check_s "larger repeat keeps the first" "abc"
    (List.hd r.crashes).minimal_input

(* every retention point copies: a caller's buffer cannot change a retained input *)
let test_retention_copies () =
  let seed = Bytes.to_string (Bytes.of_string "retained-seed") in
  let r =
    Fuzz_engine.run_outcomes [ seed ]
      { T.default_options with max_iterations = 0 } (fun _ -> (T.Pass, [ 1 ]))
  in
  check_b "corpus admission is a copy" true (List.hd r.corpus != seed);
  check_s "and equal" seed (List.hd r.corpus);
  let r =
    Fuzz_engine.run_outcomes [ seed ]
      { T.default_options with max_iterations = 0; minimize_budget = 0 }
      (fun _ -> (T.Crash ("K", []), []))
  in
  check_b "novel crash witness is a copy" true
    ((List.hd r.crashes).minimal_input != seed);
  let small = String.sub "xy" 0 2 in
  let r =
    Fuzz_engine.run_outcomes [ seed; small ]
      { T.default_options with max_iterations = 0; minimize_budget = 0 }
      (fun _ -> (T.Crash ("K", []), []))
  in
  check_b "repeat crash witness is a copy" true
    ((List.hd r.crashes).minimal_input != small);
  check_s "and is the smaller" "xy" (List.hd r.crashes).minimal_input

(* --- facade: run, verdicts, leaks on every path ------------------------- *)

let test_facade_run () =
  let o =
    {
      T.default_options with
      max_iterations = 100;
      seed = 7L;
      timeout_ms = 5000;
    }
  in
  let target data h =
    Fuzz_handle.edge h (Fuzz_handle.edge_id "entered");
    if String.length data >= 2 && data.[0] = '\255' && data.[1] = '\255' then
      failwith "boom"
  in
  let r = Fuzz.run [ "\000\000" ] o target in
  check_s "tier from edge calls" T.tier_reported r.coverage_tier;
  check_i "iterations" 100 r.iterations;
  let plain =
    Fuzz.run [ "\000\000" ] { o with max_iterations = 5 } (fun _ _ -> ())
  in
  check_s "black box" T.tier_black_box plain.coverage_tier;
  check_b "clean" true (T.is_clean plain);
  (* verdicts *)
  let v =
    Fuzz.verdict_for (fun _ _ -> ()) plain { o with ci = Types.Ci_off } "t"
  in
  check_b "passed" true v.passed;
  check_b "reason none" true (v.reason = None);
  check_b "seed in Ci_off" true (v.v_seed = Some 7L);
  let crashy =
    Fuzz.run [ "\255\255" ] { o with max_iterations = 0; ci = Types.Ci_off }
      (fun _ _ -> failwith "boom")
  in
  let always _ _ = failwith "boom" in
  let v = Fuzz.verdict_for always crashy { o with ci = Types.Ci_off } "t" in
  check_b "not passed" false v.passed;
  check_b "reason crash" true (v.reason = Some "crash");
  check_b "reproduced" true
    (List.for_all
       (fun (s : T.crash_status) -> s.status = T.Reproduced)
       v.crash_statuses);
  check_b "witness in Ci_off" true
    (match v.witnesses with
    | Some [ (_, w) ] -> w = "\255\255" || String.length w <= 2
    | _ -> false);
  List.iter
    (fun ci ->
      let v = Fuzz.verdict_for always crashy { o with ci } "t" in
      check_b "no seed" true (v.v_seed = None);
      check_b "no witness" true (v.witnesses = None);
      check_b "lines only the opaque token" true
        (List.for_all (fun l -> contains l "[fuzz-crash:") v.lines))
    [ Types.Ci_on; Types.Ci_unset ];
  (* a crash that does not come back is Flaky, still failing *)
  let v = Fuzz.verdict_for (fun _ _ -> ()) crashy o "t" in
  check_b "flaky" true
    (List.for_all
       (fun (s : T.crash_status) -> s.status = T.Flaky)
       v.crash_statuses);
  check_b "flaky still fails" false v.passed;
  (* a dropped input is Unverified; never a pass *)
  let dropped =
    {
      crashy with
      crashes =
        List.map
          (fun c -> { c with T.input_dropped = true; minimal_input = "" })
          crashy.crashes;
    }
  in
  let v = Fuzz.verdict_for always dropped { o with ci = Types.Ci_off } "t" in
  check_b "unverified" true
    (List.for_all
       (fun (s : T.crash_status) -> s.status = T.Unverified)
       v.crash_statuses);
  check_b "no witness for a dropped input" true (v.witnesses = Some []);
  (* the other reasons *)
  let reason r =
    (Fuzz.verdict (r : T.report) o (fun _ -> (T.Pass, [])) "t").reason
  in
  check_b "timeout" true (reason (mk_report ~timeouts:1 ()) = Some "timeout");
  check_b "oom" true (reason (mk_report ~ooms:1 ()) = Some "oom");
  check_b "dropped crashes" true
    (reason { (mk_report ()) with crashes_dropped = 1 } = Some "crashes_dropped");
  check_b "dropped admissions" true
    (reason { (mk_report ()) with admissions_dropped = 1 }
    = Some "admissions_dropped");
  check_b "leaked" true (reason (mk_report ~leaked:1 ()) = Some "workers-leaked");
  check_b "cap" true (reason (mk_report ~leaked:4 ()) = Some "workers-leaked");
  check_b "no iterations" true
    (reason (mk_report ~iterations:0 ()) = Some "no_iterations");
  check_s "verdict show"
    "FuzzVerdict<passed=false reason=no_iterations lines=0>"
    (T.show_verdict
       (Fuzz.verdict (mk_report ~iterations:0 ()) o (fun _ -> (T.Pass, [])) "t"));
  check_s "verdict show passed" "FuzzVerdict<passed=true reason=- lines=0>"
    ( T.show_verdict v |> fun _ ->
      T.show_verdict (Fuzz.verdict (mk_report ()) o (fun _ -> (T.Pass, [])) "t")
    );
  (* the executor facades *)
  let ex = Fuzz.executor (fun _ h -> Fuzz_handle.edge h 9) o in
  Alcotest.(check (pair bool (list int)))
    "executor" (true, [ 9 ])
    (let out, es = ex "x" in
     (out = T.Pass, es));
  check_b "re-exports" true
    (Fuzz.crash_signature "timeout" [] = "4e61ea76"
    && Fuzz.calculate_energy 10 4 3 6 = 8
    && Fuzz.corpus_admit [ 1L ] [ 2L ] = (true, 1)
    && Fuzz.format_tap_failure 1 "t" "fuzz-crash" "4e61ea76"
       = "not ok 1 - t [fuzz-crash:4e61ea76]"
    && Fuzz.tap_failures (mk_report ()) "t" 1 = [])

(* A campaign whose target always spins is aborted at the cap, bounded, never a pass *)
let test_leak_cap_campaign () =
  with_release (fun release ->
      let calls = Atomic.make 0 in
      let o = { quick with max_iterations = 50; ci = Types.Ci_off } in
      let r =
        Fuzz.run ~kill_wait_ms:20 ~leaked_kill_wait_ms:5 [ "a" ] o (fun d _ ->
            Atomic.incr calls;
            (* the seed passes (so there is something to mutate); every mutant spins *)
            if d <> "a" then spin_until release)
      in
      check_i "leaked workers" Fuzz_limits.max_leaked_workers r.leaked_workers;
      check_i "executions stop at the cap"
        (1 + Fuzz_limits.max_leaked_workers)
        (Atomic.get calls);
      check_b "never clean" false (T.is_clean r);
      Alcotest.(check (list string))
        "opaque lines"
        [
          "not ok 1 - t [fuzz-timeout:4e61ea76]";
          "not ok 2 - t [fuzz-timeout:cc813cf1]";
        ]
        (Fuzz_signature.tap_failures r "t" 1);
      let v = Fuzz.verdict_for (fun _ _ -> ()) r o "t" in
      check_b "reason" true (v.reason = Some "workers-leaked"))

(* the minimiser asks the abort flag before every candidate *)
let test_leak_cap_minimizer () =
  with_release (fun release ->
      let calls = Atomic.make 0 in
      let o = { quick with max_iterations = 50; minimize_budget = 1000 } in
      let seed = "\001\001\001\001\001\001\001\001" in
      let r =
        Fuzz.run ~kill_wait_ms:20 ~leaked_kill_wait_ms:5 [ seed ] o (fun d _ ->
            Atomic.incr calls;
            if d = seed then failwith "boom" else spin_until release)
      in
      (* the first (triage) run crashes; each minimiser probe leaks; after 4 the
         minimiser stops probing *)
      check_i "probes stop at the cap"
        (1 + Fuzz_limits.max_leaked_workers)
        (Atomic.get calls);
      check_i "leaked" Fuzz_limits.max_leaked_workers r.leaked_workers;
      check_b "witness not minimised" true
        (match r.crashes with [ c ] -> not c.minimized | _ -> false))

(* re-verification shares the tracker: it stops at the cap *)
let test_leak_cap_reverify () =
  with_release (fun release ->
      let calls = Atomic.make 0 in
      let o = { quick with ci = Types.Ci_off } in
      let crashes =
        List.init 6 (fun i -> mk_crash (Printf.sprintf "0000000%d" i))
      in
      let report = mk_report ~crashes ~timeouts:0 () in
      let target _ _ =
        Atomic.incr calls;
        spin_until release
      in
      let v =
        Fuzz.verdict_for ~kill_wait_ms:20 ~leaked_kill_wait_ms:5 target report o
          "t"
      in
      check_i "re-runs stop at the cap" Fuzz_limits.max_leaked_workers
        (Atomic.get calls);
      let statuses =
        List.map (fun (s : T.crash_status) -> s.status) v.crash_statuses
      in
      check_i "all six reported" 6 (List.length statuses);
      check_i "the rest are unverified" 2
        (List.length (List.filter (( = ) T.Unverified) statuses));
      (* a report that already leaked the cap verifies nothing *)
      Atomic.set calls 0;
      let v = Fuzz.verdict_for target (mk_report ~crashes ~leaked:4 ()) o "t" in
      check_i "nothing re-run" 0 (Atomic.get calls);
      check_b "all unverified" true
        (List.for_all
           (fun (s : T.crash_status) -> s.status = T.Unverified)
           v.crash_statuses);
      (* plain verdict applies only the report's own count *)
      let v =
        Fuzz.verdict
          (mk_report ~crashes:[ mk_crash "00000000" ] ~leaked:4 ())
          o
          (fun _ -> (T.Pass, []))
          "t"
      in
      check_b "plain verdict honours the report" true
        (List.for_all
           (fun (s : T.crash_status) -> s.status = T.Unverified)
           v.crash_statuses))

(* The executor itself: a domain is spawned per execution and joined *)
let test_domain_isolation () =
  let key = Domain.DLS.new_key (fun () -> 0) in
  let r = exec_once (fun _ _ -> Domain.DLS.set key 99) "" in
  check_b "pass" true (r.outcome = T.Pass);
  check_i "a target's DLS never reaches the engine's domain" 0
    (Domain.DLS.get key);
  let ids = ref [] in
  for _ = 1 to 3 do
    ignore (exec_once (fun _ _ -> ids := (Domain.self () :> int) :: !ids) "")
  done;
  check_b "not the caller's domain" true
    (List.for_all (fun i -> i <> (Domain.self () :> int)) !ids);
  (* many executions in a row do not exhaust the domain table *)
  for _ = 1 to 300 do
    ignore (exec_once (fun _ h -> H.edge h 1) "")
  done

let () =
  Alcotest.run "test-bootstrap fuzz (VER06r)"
    [
      ( "corpus",
        [
          tc "replay (fuzz-v1 25/25)" test_corpus_replay;
          tc "negative controls" test_negative_controls;
          tc "structure controls" test_corpus_structure_controls;
          tc "sabotage hooks" test_hooks_sabotage;
          tc "aggregate sequence work cap" test_aggregate_work_cap;
          tc "go vectors (820/820)" test_vectors;
          tc "vector replayer rejects" test_vector_replayer_rejects;
          tc "malformed fixtures" test_malformed_cases;
        ] );
      ( "pins",
        [
          tc "mutators" test_mutators;
          tc "constants" test_constants;
          tc "rejection draws" test_rejection_draws;
          tc "sequence work" test_sequence_work;
          tc "bitmap and energy" test_bitmap_energy;
          tc "signatures" test_signatures;
          tc "tap" test_tap;
          tc "report shapes" test_report_shapes;
          tc "minimiser" test_minimizer;
        ] );
      ( "handle",
        [
          tc "basics" test_handle_basics;
          tc "sticky both ways" test_sticky_both_ways;
          tc "retired and forged are inert" test_retired_forged_inert;
          tc "cross-check" test_cross_check;
        ] );
      ( "executor",
        [
          tc "outcomes and classification" test_executor_outcomes;
          tc "budgets" test_executor_budgets;
          tc "stale sentinels" test_executor_stale_sentinels;
          tc "runaway targets" test_executor_runaway;
          tc "hot path is not charged" test_hot_path_is_not_charged;
          tc "honest target under leaks" test_honest_target_under_leaks;
          tc "global leak cap" test_global_leak_cap;
          tc "wait under clock steps" test_wait_clock_steps;
          tc "domain ceiling" test_domain_ceiling;
          tc "domain isolation" test_domain_isolation;
        ] );
      ( "campaign",
        [
          tc "cross-check against the Ruby lane" test_campaign_cross_check;
          tc "contracts" test_campaign_contracts;
          tc "caps" test_campaign_caps;
          tc "retention copies" test_retention_copies;
          tc "facade and verdicts" test_facade_run;
          tc "leak cap: campaign" test_leak_cap_campaign;
          tc "leak cap: minimiser" test_leak_cap_minimizer;
          tc "leak cap: re-verification" test_leak_cap_reverify;
        ] );
    ]
