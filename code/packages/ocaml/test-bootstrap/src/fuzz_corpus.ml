(* The production runner for the shared fuzz-v1 corpus (code/specs/fixtures/
   fuzz-v1/cases.json, VER06 8, VER06r 11).

   It takes the parsed JSON, strictly decodes every case, evaluates it with THIS
   package's own engine functions (through [Fuzz_hooks], so a test can sabotage
   one) and returns human-readable problems.  An empty list means "conforms".  It
   does no file I/O.

   A runner that cannot report a problem is worse than none, so this one is
   strict:
   - a case has exactly the fields of its op, in canonical encodings;
   - an unknown op or a malformed value is a PROBLEM, never a silent skip;
   - every op appears, a registered failure id is accepted and an unregistered
     one rejected, and the case count is 25, so deleting cases is a problem;
   - fixture-controlled sizes are capped (CWE-400): a hostile steps = 10^9 is a
     reported problem, not a hang, and the aggregate [mutate_sequence] work of
     the whole corpus is capped at 2^24 bytes ACTUALLY copied (a breach zeroes
     the allowance, so every later case fails at once);
   - every problem about a case starts with that case's id, ESCAPED. *)

module J = Strict_json

type report = Corpus.report = { evaluated : int; problems : string list }

let fuzz_v1_case_count = 25
let max_corpus_cases = 10_000
let max_problems = 1000
let frag = Values.max_fragment_len
let parse_corpus = Corpus.parse_corpus

(* The closed field set of each op (besides id and op). *)
let shapes =
  [
    ("mutate_bit_flip", [ "input"; "bit_offset"; "count"; "output" ]);
    ("mutate_byte_flip", [ "input"; "byte_offset"; "count"; "output" ]);
    ( "mutate_arithmetic",
      [ "input"; "byte_offset"; "delta"; "width"; "endian"; "output" ] );
    ( "mutate_boundary",
      [ "input"; "byte_offset"; "width"; "val"; "endian"; "output" ] );
    ("mutate_block_delete", [ "input"; "offset"; "length"; "output" ]);
    ("mutate_block_insert", [ "input"; "offset"; "insert_bytes"; "output" ]);
    ( "mutate_block_duplicate",
      [ "input"; "src_offset"; "length"; "dst_offset"; "output" ] );
    ( "mutate_splice",
      [ "target"; "donor"; "target_split"; "donor_split"; "output" ] );
    ("mutate_sequence", [ "input"; "seed"; "steps"; "output" ]);
    ( "corpus_admit",
      [ "existing_edges"; "trace_edges"; "admitted"; "new_edges" ] );
    ( "crash_dedup",
      [
        "crash_a_kind";
        "crash_a_frames";
        "crash_b_kind";
        "crash_b_frames";
        "is_duplicate";
      ] );
    ("failure_id_valid", [ "input"; "valid" ]);
    ( "private_reporting",
      [
        "target";
        "failure_id";
        "raw_payload";
        "sig_hash";
        "tap_line";
        "contains_raw_payload";
      ] );
    ("corpus_reject", [ "text" ]);
  ]

exception Case_error of string

type case_result = { problems : string list; ran_ok : bool; accepted : bool }

let case_error m = raise (Case_error m)

let int_of what = function
  | Some (Val.Int i) -> i
  | _ -> case_error (what ^ " must be an int")

(* An int that drives a loop or an allocation: 0 <= n <= 10000.  Checked BEFORE
   conversion to a native int. *)
let size_of what v =
  match Bigint.to_int_opt (int_of what v) with
  | Some k when k >= 0 && k <= Values.max_size -> k
  | _ -> case_error (Printf.sprintf "%s outside [0, %d]" what Values.max_size)

let bytes_of what = function
  | Some (Val.Bytes b) -> b
  | _ -> case_error (what ^ " must be bytes")

let text_of what = function
  | Some (Val.Text t) -> t
  | _ -> case_error (what ^ " must be a string")

let bool_of what = function
  | Some (Val.Bool b) -> b
  | _ -> case_error (what ^ " must be a bool")

let endian_of v =
  match Fuzz_mutate.parse_endian (text_of "endian" v) with
  | Some e -> e
  | None -> case_error "endian must be 'little' or 'big'"

let texts_of what = function
  | Some (Val.List xs) ->
      List.map
        (function
          | Val.Text t -> t | _ -> case_error (what ^ " must be strings"))
        xs
  | _ -> case_error (what ^ " must be a list of strings")

let u64_max = Bigint.sub (Bigint.pow2 64) Bigint.one

(* Edge ids are u64: 0 <= e < 2^64. *)
let edges_of what = function
  | Some (Val.List xs) ->
      List.map
        (function
          | Val.Int e ->
              if Bigint.sign e < 0 || Bigint.compare e u64_max > 0 then
                case_error (what ^ " element outside [0, 2^64)")
              else Bigint.to_int64_wrap e
          | _ -> case_error (what ^ " must be a list of ints"))
        xs
  | _ -> case_error (what ^ " must be a list of ints")

let hex s =
  let b = Buffer.create (2 * String.length s) in
  String.iter
    (fun c -> Buffer.add_string b (Printf.sprintf "%02x" (Char.code c)))
    s;
  Buffer.contents b

let hex_clip s = Escape.clip_text frag (hex s)

let check_mutator (hooks : Fuzz_hooks.t) work cid op v =
  let get k = List.assoc_opt k v in
  let i k = int_of k (get k) in
  let input () = bytes_of "input" (get "input") in
  let got =
    match op with
    | "mutate_bit_flip" ->
        hooks.bit_flip (input ()) (i "bit_offset") (i "count")
    | "mutate_byte_flip" ->
        hooks.byte_flip (input ()) (i "byte_offset") (i "count")
    | "mutate_arithmetic" ->
        hooks.byte_arithmetic (input ()) (i "byte_offset") (i "delta")
          (i "width")
          (endian_of (get "endian"))
    | "mutate_boundary" ->
        hooks.boundary_insert (input ()) (i "byte_offset") (i "width") (i "val")
          (endian_of (get "endian"))
    | "mutate_block_delete" ->
        hooks.block_delete (input ()) (i "offset") (i "length")
    | "mutate_block_insert" ->
        hooks.block_insert (input ()) (i "offset")
          (bytes_of "insert_bytes" (get "insert_bytes"))
    | "mutate_block_duplicate" ->
        hooks.block_duplicate (input ()) (i "src_offset") (i "length")
          (i "dst_offset")
    | "mutate_splice" ->
        hooks.splice
          (bytes_of "target" (get "target"))
          (bytes_of "donor" (get "donor"))
          (i "target_split") (i "donor_split")
    | _ -> (
        let data = input () in
        let seed = i "seed" in
        if Bigint.sign seed < 0 || Bigint.compare seed u64_max > 0 then
          case_error "seed outside [0, 2^64-1]";
        let steps = size_of "steps" (get "steps") in
        let exhausted =
          Printf.sprintf
            "mutate_sequence work budget of %d byte-steps exhausted"
            Fuzz_limits.corpus_sequence_work
        in
        if !work <= 0 then case_error exhausted;
        match
          hooks.mutate_sequence data seed (Bigint.of_int steps)
            Fuzz_limits.max_input_bytes !work
        with
        | Fuzz_mutate.Work_ok (out, done_) ->
            work := max 0 (!work - done_);
            out
        | Fuzz_mutate.Work_exhausted _ ->
            work := 0;
            case_error exhausted)
  in
  let want = bytes_of "output" (get "output") in
  let mismatch =
    if got <> want then
      [
        Printf.sprintf "%s: %s expected %s, got %s" cid op (hex_clip want)
          (hex_clip got);
      ]
    else []
  in
  let lens =
    match op with
    | "mutate_bit_flip" | "mutate_byte_flip" | "mutate_arithmetic"
    | "mutate_boundary" ->
        if String.length got <> String.length (input ()) then
          [ Printf.sprintf "%s: %s changed the length of its input" cid op ]
        else []
    | "mutate_splice" ->
        let t = bytes_of "target" (get "target")
        and d = bytes_of "donor" (get "donor") in
        let clamp size n =
          if Bigint.sign n <= 0 then 0
          else if Bigint.compare n (Bigint.of_int size) >= 0 then size
          else Option.value (Bigint.to_int_opt n) ~default:size
        in
        let want_len =
          clamp (String.length t) (i "target_split")
          + String.length d
          - clamp (String.length d) (i "donor_split")
        in
        if String.length got <> want_len then
          [ Printf.sprintf "%s: splice length is not prefix + suffix" cid ]
        else []
    | _ -> []
  in
  mismatch @ lens

let check_corpus_admit (hooks : Fuzz_hooks.t) cid v =
  let get k = List.assoc_opt k v in
  let existing = edges_of "existing_edges" (get "existing_edges") in
  let trace = edges_of "trace_edges" (get "trace_edges") in
  let admitted = bool_of "admitted" (get "admitted") in
  let new_edges = int_of "new_edges" (get "new_edges") in
  let got_admitted, got_new = hooks.corpus_admit existing trace in
  let have = List.sort_uniq compare existing in
  [
    (if got_admitted <> admitted then
       Some
         (Printf.sprintf "%s: corpus_admit admitted %b, corpus %b" cid
            got_admitted admitted)
     else None);
    (if not (Bigint.equal (Bigint.of_int got_new) new_edges) then
       Some
         (Printf.sprintf "%s: corpus_admit new_edges %d, corpus %s" cid got_new
            (Bigint.to_string new_edges))
     else None);
    (if got_admitted <> List.exists (fun e -> not (List.mem e have)) trace then
       Some (cid ^ ": corpus_admit disagrees with set difference")
     else None);
  ]
  |> List.filter_map Fun.id

let check_crash_dedup (hooks : Fuzz_hooks.t) cid v =
  let get k = List.assoc_opt k v in
  let ka = text_of "crash_a_kind" (get "crash_a_kind")
  and fa = texts_of "crash_a_frames" (get "crash_a_frames")
  and kb = text_of "crash_b_kind" (get "crash_b_kind")
  and fb = texts_of "crash_b_frames" (get "crash_b_frames") in
  let want = bool_of "is_duplicate" (get "is_duplicate") in
  let got = hooks.duplicate_crash ka fa kb fb in
  if got <> want then
    [ Printf.sprintf "%s: crash_dedup is_duplicate %b, corpus %b" cid got want ]
  else []

let contains_sub hay needle =
  let n = String.length needle and h = String.length hay in
  let rec go i = i + n <= h && (String.sub hay i n = needle || go (i + 1)) in
  n > 0 && go 0

let check_private (hooks : Fuzz_hooks.t) cid v =
  let get k = List.assoc_opt k v in
  let raw = bytes_of "raw_payload" (get "raw_payload") in
  let target = text_of "target" (get "target") in
  let fid = text_of "failure_id" (get "failure_id") in
  let sig_hash = text_of "sig_hash" (get "sig_hash") in
  let tap = text_of "tap_line" (get "tap_line") in
  let want_leak = bool_of "contains_raw_payload" (get "contains_raw_payload") in
  let line =
    match
      Safe.recoverable (fun () -> hooks.format_tap 1 target fid sig_hash)
    with
    | Ok l -> l
    | Error e ->
        let m =
          match e with
          | Error.Argument_error m -> Escape.clip_text frag m
          | _ -> ""
        in
        case_error
          (Printf.sprintf "format_tap_failure refused: %s: %s" (Safe.exn_name e)
             m)
  in
  let leaked = raw <> "" && contains_sub line (hex raw) in
  (if line <> tap then
     [
       Printf.sprintf "%s: tap_line expected %s, got %s" cid
         (Escape.clip_text frag (Escape.escape_text true tap))
         (Escape.clip_text frag (Escape.escape_text true line));
     ]
   else [])
  @ (if leaked <> want_leak then
       [
         Printf.sprintf "%s: privacy verdict wrong, payload leaked: %b" cid
           leaked;
       ]
     else [])
  @
  if String.contains line '\n' || String.contains line '\r' then
    [ cid ^ ": failure line spans more than one line" ]
  else []

let done_ problems = { problems; ran_ok = true; accepted = false }

let run_case hooks work cid op fields =
  let decoded =
    List.filter_map
      (fun (k, j) ->
        if k = "id" || k = "op" then None
        else
          match Values.decode j with
          | Ok v -> Some (k, v)
          | Error m -> case_error m)
      fields
  in
  match op with
  | "corpus_admit" -> done_ (check_corpus_admit hooks cid decoded)
  | "crash_dedup" -> done_ (check_crash_dedup hooks cid decoded)
  | "private_reporting" -> done_ (check_private hooks cid decoded)
  | "failure_id_valid" ->
      let got =
        hooks.failure_id (text_of "input" (List.assoc_opt "input" decoded))
      in
      let valid = bool_of "valid" (List.assoc_opt "valid" decoded) in
      {
        problems =
          (if valid = got then []
           else [ cid ^ ": failure id membership disagrees" ]);
        ran_ok = true;
        accepted = got;
      }
  | "corpus_reject" ->
      let text = text_of "text" (List.assoc_opt "text" decoded) in
      done_
        (if hooks.tfv_valid text then
           [ cid ^ ": corpus_reject text was accepted by the strict parser" ]
         else [])
  | _ -> done_ (check_mutator hooks work cid op decoded)

let evaluate_case hooks work cid op fields =
  match Safe.recoverable (fun () -> run_case hooks work cid op fields) with
  | Ok r -> r
  | Error e ->
      let detail =
        match e with
        | Case_error m | Error.Argument_error m ->
            Escape.escape_text false (Escape.clip_text frag m)
        | _ -> Safe.exn_name e
      in
      {
        problems =
          [ Printf.sprintf "%s: %s could not be evaluated: %s" cid op detail ];
        ran_ok = false;
        accepted = false;
      }

(* Duplicate or missing ids stop the id scan at the first offender. *)
let check_ids cases =
  let seen = Hashtbl.create 32 in
  let rec go = function
    | [] -> []
    | J.Object fields :: rest -> (
        match List.assoc_opt "id" fields with
        | Some (J.String i) when i <> "" && not (Hashtbl.mem seen i) ->
            Hashtbl.add seen i ();
            go rest
        | _ -> [ "duplicate or missing case id" ])
    | _ -> [ "duplicate or missing case id" ]
  in
  go cases

let evaluate_with hooks expected cases_in =
  let work = ref Fuzz_limits.corpus_sequence_work in
  let count_problem =
    match expected with
    | Some n when List.length cases_in <> n ->
        [
          Printf.sprintf "fuzz-v1: expected %d cases, got %d" n
            (List.length cases_in);
        ]
    | _ -> []
  in
  let too_many = List.compare_length_with cases_in max_corpus_cases > 0 in
  let cap_problem =
    if too_many then
      [
        Printf.sprintf "fuzz-v1: more than %d cases exceeds the cap"
          max_corpus_cases;
      ]
    else []
  in
  let cases =
    if too_many then List.filteri (fun i _ -> i < max_corpus_cases) cases_in
    else cases_in
  in
  let id_problem = check_ids cases in
  let ops = Hashtbl.create 16 in
  let accepted = ref false and rejected = ref false in
  let evaluated = ref 0 and problems = ref [] in
  let add p = problems := p :: !problems in
  List.iter
    (function
      | J.Object fields -> (
          match List.assoc_opt "id" fields with
          | Some (J.String idb) when idb <> "" -> (
              let cid = Escape.escape_text false (Escape.clip_text 100 idb) in
              let op =
                match List.assoc_opt "op" fields with
                | Some (J.String o) -> o
                | _ -> ""
              in
              match List.assoc_opt op shapes with
              | None ->
                  add
                    (Printf.sprintf "%s: unknown op %s" cid
                       (Escape.escape_text true (Escape.clip_text 60 op)))
              | Some want ->
                  let want_keys = List.sort compare ("id" :: "op" :: want) in
                  let have_keys = List.sort compare (List.map fst fields) in
                  if have_keys <> want_keys then
                    add
                      (Printf.sprintf "%s: fields %s != expected %s" cid
                         (Escape.escape_text true
                            (Escape.clip_text 120 (String.concat "," have_keys)))
                         (String.concat "," want_keys))
                  else
                    let res = evaluate_case hooks work cid op fields in
                    List.iter add res.problems;
                    if res.ran_ok then (
                      if op = "failure_id_valid" then
                        if res.accepted then accepted := true
                        else rejected := true;
                      Hashtbl.replace ops op ();
                      incr evaluated))
          | _ -> add "case missing a string id")
      | _ -> add "case is not an object")
    cases;
  let missing_ops =
    List.sort compare
      (List.filter_map
         (fun (op, _) -> if Hashtbl.mem ops op then None else Some op)
         shapes)
  in
  let unregistered =
    List.filter
      (fun i -> not (hooks.Fuzz_hooks.failure_id i))
      Fuzz_failure_ids.all
  in
  let final =
    count_problem @ cap_problem @ id_problem @ List.rev !problems
    @ (if missing_ops <> [] then
         [
           "fuzz-v1: operations never exercised: "
           ^ String.concat ", " missing_ops;
         ]
       else [])
    @ (if not !accepted then
         [ "fuzz-v1: failure_id_valid needs a registered-id acceptance" ]
       else [])
    @ (if not !rejected then
         [ "fuzz-v1: failure_id_valid needs an unregistered-id rejection" ]
       else [])
    @
    if unregistered <> [] then
      [
        "fuzz-v1: engine rejects registered ids "
        ^ String.concat ", " unregistered;
      ]
    else []
  in
  let final = List.map (Escape.clip_text Values.max_problem_len) final in
  {
    evaluated = !evaluated;
    problems = List.filteri (fun i _ -> i < max_problems) final;
  }

let evaluate ?(hooks = Fuzz_hooks.default) ?(expected = Some fuzz_v1_case_count)
    cases =
  evaluate_with hooks expected cases

let evaluate_text ?hooks text =
  match parse_corpus text with
  | Ok cases -> evaluate ?hooks cases
  | Error e ->
      {
        evaluated = 0;
        problems = [ "corpus does not parse: " ^ Escape.clip_text frag e ];
      }
