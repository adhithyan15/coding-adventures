(* The pbt-v1 conformance evaluator.

   Why a runner that can fail matters: a conformance check that cannot report a
   problem is worse than none.  So the runner is strict:
   - a case must have exactly the fields of its op, with canonical encodings;
   - an unknown op, unknown property name or malformed value is a PROBLEM, never
     a silent skip;
   - every op and all four failure ids must appear, an unregistered id must be
     rejected, and the number of cases must equal 25, so deleting cases is
     itself a problem;
   - every problem about a case starts with that case's (escaped, clipped) id. *)

module J = Strict_json

type report = { evaluated : int; problems : string list }

let pbt_v1_case_count = 25
let max_corpus_cases = 10_000
let max_problems = 1000
let max_gen_bytes = 1_048_576
let max_size = Values.max_size
let frag = Values.max_fragment_len

let parse_corpus text =
  match J.parse text with
  | Error e -> Error e
  | Ok (J.Object fields) when List.length fields = 2 -> (
      match
        (List.assoc_opt "schema_version" fields, List.assoc_opt "cases" fields)
      with
      | Some (J.Number "1"), Some (J.Array cases) -> Ok cases
      | Some (J.Number "1"), _ -> Error "pbt corpus: \"cases\" must be an array"
      | _ -> Error "pbt corpus: schema_version must be 1")
  | Ok _ ->
      Error "pbt corpus: want exactly {\"schema_version\": 1, \"cases\": [...]}"

(* The closed field set of each op (besides id and op). *)
let shapes =
  [
    ("gen_primitive", [ "type"; "seed"; "count"; "outputs" ]);
    ("gen_bounded", [ "min"; "max"; "seed"; "count"; "outputs" ]);
    ("gen_bytes", [ "min_len"; "max_len"; "seed"; "count"; "outputs" ]);
    ( "shrink_int",
      [
        "initial";
        "target";
        "min";
        "max";
        "property";
        "expected_minimal";
        "expected_steps";
      ] );
    ( "shrink_refinement",
      [
        "initial";
        "target";
        "min";
        "max";
        "property";
        "expected_minimal";
        "expected_steps";
      ] );
    ( "shrink_list",
      [
        "initial";
        "elem_target";
        "property";
        "expected_minimal";
        "expected_steps";
      ] );
    ( "shrink_bytes",
      [ "initial"; "property"; "expected_minimal"; "expected_steps" ] );
    ( "replay_consistency",
      [ "seed"; "property"; "initial_fail"; "expected_minimal" ] );
    ("filter_exhausted", [ "max_retries"; "verdict"; "failure_id" ]);
    ("failure_id_valid", [ "input"; "valid" ]);
    ("corpus_reject", [ "text" ]);
  ]

let big = Bigint.of_int
let even x = Bigint.sign (Bigint.rem_pos x (big 2)) = 0
let le x n = Bigint.compare x (big n) <= 0
let ge x n = Bigint.compare x (big n) >= 0

let int_props =
  [
    ("int_le_100", fun x -> le x 100);
    ("int_in_range_neg10_50", fun x -> ge x (-10) && le x 50);
    ("int_is_even", even);
    ("int_abs_le_10", fun x -> le (Bigint.abs x) 10);
    ("refinement_x_le_50", fun x -> le x 50);
  ]

let rec no_consecutive_dups = function
  | a :: (b :: _ as rest) ->
      (not (Bigint.equal a b)) && no_consecutive_dups rest
  | _ -> true

let list_props =
  [
    ("list_no_consecutive_duplicates", no_consecutive_dups);
    ("list_sum_le_20", fun l -> le (List.fold_left Bigint.add Bigint.zero l) 20);
  ]

let bytes_props =
  [
    ("bytes_len_lt_5", fun b -> String.length b < 5);
    ("bytes_no_zero_byte", fun b -> not (String.contains b '\000'));
  ]

exception Case_error of string

type case_result = {
  problems : string list;
  ran_ok : bool;
  candidate : string;
  registered : bool;
}

let int_of what = function
  | Some (Val.Int i) -> i
  | _ -> raise (Case_error (what ^ " must be an int"))

(* An int that drives a loop or an allocation: 0 <= n <= 10000.  Checked BEFORE
   conversion to a native int. *)
let size_of what v =
  let n = int_of what v in
  match Bigint.to_int_opt n with
  | Some k when k >= 0 && k <= max_size -> k
  | _ -> raise (Case_error (Printf.sprintf "%s outside [0, %d]" what max_size))

let u64_max = Bigint.sub (Bigint.pow2 64) Bigint.one

(* A fixture seed is a u64: refused outside [0, 2^64-1], never reduced. *)
let seed_of v =
  let n = int_of "seed" v in
  if Bigint.sign n < 0 || Bigint.compare n u64_max > 0 then
    raise (Case_error "seed outside [0, 2^64-1]")
  else Bigint.to_int64_wrap n

let prop_of op table what = function
  | Some (Val.Text name) when List.mem_assoc name table -> List.assoc name table
  | _ ->
      raise (Case_error (Printf.sprintf "unknown property for %s (%s)" op what))

let show v = Escape.clip_text frag (Canon.render v)

let mismatch cid label got expected =
  match expected with
  | Some e when Canon.same_val got e -> []
  | Some e ->
      [
        Printf.sprintf "%s: %s: got %s, corpus %s" cid label (show got) (show e);
      ]
  | None ->
      [ Printf.sprintf "%s: %s: got %s, corpus <absent>" cid label (show got) ]

let ints_of_list what = function
  | Some (Val.List xs) -> List.map (fun x -> int_of what (Some x)) xs
  | _ -> raise (Case_error "initial must be a list of ints")

let check_gen cfg cid op v =
  let get k = List.assoc_opt k v in
  let seed = seed_of (get "seed") in
  let count = size_of "count" (get "count") in
  (* A case whose list length is not count cannot conform: refuse before generating. *)
  let expected_out =
    match get "outputs" with
    | Some (Val.List xs as o) when List.length xs = count -> o
    | _ ->
        raise
          (Case_error
             (Printf.sprintf
                "outputs must be a list of exactly count (%d) values" count))
  in
  let got =
    match op with
    | "gen_primitive" -> (
        match get "type" with
        | Some (Val.Text ty) -> Gen.gen_primitive cfg seed ty count
        | _ -> raise (Case_error "type must be a string"))
    | "gen_bounded" ->
        Gen.gen_bounded cfg seed
          (int_of "min" (get "min"))
          (int_of "max" (get "max"))
          count
    | _ ->
        let min_len = size_of "min_len" (get "min_len")
        and max_len = size_of "max_len" (get "max_len") in
        if count * max_len > max_gen_bytes then
          raise (Case_error "count x max_len exceeds the generation budget");
        Gen.gen_bytes cfg seed min_len max_len count
  in
  if Canon.same_val (Val.List got) expected_out then []
  else [ Printf.sprintf "%s: %s outputs mismatch" cid op ]

let check_shrink_int cfg cid op v =
  let get k = List.assoc_opt k v in
  let prop = prop_of op int_props "int" (get "property") in
  let initial = int_of "initial" (get "initial")
  and target = int_of "target" (get "target") in
  let lo = int_of "min" (get "min") and hi = int_of "max" (get "max") in
  let minimal, steps =
    Shrink.shrink_int ~config:cfg ~max_steps:1000 ~initial ~target ~lo ~hi prop
  in
  let p1 =
    mismatch cid (op ^ " minimal mismatch") (Val.Int minimal)
      (get "expected_minimal")
  in
  let p2 =
    mismatch cid
      (op ^ " step count mismatch")
      (Val.Int (big steps))
      (get "expected_steps")
  in
  (* Engine invariants, independent of the corpus's expected values. *)
  let p3 =
    if prop minimal then
      [ Printf.sprintf "%s: %s minimal does not falsify the property" cid op ]
    else []
  in
  let p4 =
    if Bigint.compare minimal lo < 0 || Bigint.compare minimal hi > 0 then
      [ Printf.sprintf "%s: %s minimal left [min, max]" cid op ]
    else []
  in
  p1 @ p2 @ p3 @ p4

let check_shrink_list cfg cid v =
  let get k = List.assoc_opt k v in
  let prop = prop_of "shrink_list" list_props "list" (get "property") in
  let initial = ints_of_list "element" (get "initial") in
  let elem_target = int_of "elem_target" (get "elem_target") in
  let minimal, steps =
    Shrink.shrink_list ~config:cfg ~max_steps:1000 ~initial ~elem_target prop
  in
  mismatch cid "shrink_list minimal mismatch"
    (Val.List (List.map (fun i -> Val.Int i) minimal))
    (get "expected_minimal")
  @ mismatch cid "shrink_list steps mismatch"
      (Val.Int (big steps))
      (get "expected_steps")
  @
  if prop minimal then
    [ cid ^ ": shrink_list minimal does not falsify the property" ]
  else []

let check_shrink_bytes cfg cid v =
  let get k = List.assoc_opt k v in
  let prop = prop_of "shrink_bytes" bytes_props "bytes" (get "property") in
  let initial =
    match get "initial" with
    | Some (Val.Bytes b) -> b
    | _ -> raise (Case_error "initial must be bytes")
  in
  let minimal, steps =
    Shrink.shrink_bytes ~config:cfg ~max_steps:1000 ~initial prop
  in
  mismatch cid "shrink_bytes minimal mismatch" (Val.Bytes minimal)
    (get "expected_minimal")
  @ mismatch cid "shrink_bytes steps mismatch"
      (Val.Int (big steps))
      (get "expected_steps")
  @
  if prop minimal then
    [ cid ^ ": shrink_bytes minimal does not falsify the property" ]
  else []

let check_replay cfg cid v =
  let get k = List.assoc_opt k v in
  let seed = seed_of (get "seed") in
  let prop = prop_of "replay_consistency" int_props "int" (get "property") in
  let initial_fail, minimal = Shrink.simulate_replay ~config:cfg seed prop in
  mismatch cid "replay initial_fail mismatch" (Val.Int initial_fail)
    (get "initial_fail")
  @ mismatch cid "replay minimal mismatch" (Val.Int minimal)
      (get "expected_minimal")

let check_filter_exhausted cfg cid v =
  let get k = List.assoc_opt k v in
  let max_retries = size_of "max_retries" (get "max_retries") in
  if max_retries < 1 then raise (Case_error "max_retries must be at least 1");
  let calls = ref 0 in
  let gen =
    Gen.filter ~max_retries
      (fun _ ->
        incr calls;
        false)
      (Gen.int 0 10)
  in
  let opts =
    { Types.default_options with num_tests = 1; ci = Types.Ci_on; config = cfg }
  in
  let verdict = Runner.check ~seed:0L gen opts (fun _ -> true) in
  let verdict_ok =
    match get "verdict" with
    | Some t -> Canon.same_val (Val.Text "fail") t
    | None -> false
  in
  let p1 =
    if (not verdict_ok) || verdict.passed then
      [ cid ^ ": an unsatisfiable filter must be a fail verdict" ]
    else []
  in
  let p2 =
    match verdict.failure_id with
    | Some fid -> mismatch cid "failure id" (Val.Text fid) (get "failure_id")
    | None -> [ cid ^ ": no failure id" ]
  in
  let p3 =
    if !calls <> max_retries then
      [
        Printf.sprintf "%s: filter sampled %d times, expected %d" cid !calls
          max_retries;
      ]
    else []
  in
  p1 @ p2 @ p3

let done_ problems =
  { problems; ran_ok = true; candidate = ""; registered = false }

let run_case cfg cid op fields =
  let decoded =
    List.filter_map
      (fun (k, j) ->
        if k = "id" || k = "op" then None
        else
          match Values.decode j with
          | Ok v -> Some (k, v)
          | Error m -> raise (Case_error m))
      fields
  in
  match op with
  | "gen_primitive" | "gen_bounded" | "gen_bytes" ->
      done_ (check_gen cfg cid op decoded)
  | "shrink_int" | "shrink_refinement" ->
      done_ (check_shrink_int cfg cid op decoded)
  | "shrink_list" -> done_ (check_shrink_list cfg cid decoded)
  | "shrink_bytes" -> done_ (check_shrink_bytes cfg cid decoded)
  | "replay_consistency" -> done_ (check_replay cfg cid decoded)
  | "filter_exhausted" -> done_ (check_filter_exhausted cfg cid decoded)
  | "failure_id_valid" ->
      let input =
        match List.assoc_opt "input" decoded with
        | Some (Val.Text t) -> Some t
        | _ -> None
      in
      let got =
        match input with Some t -> Types.is_failure_id t | None -> false
      in
      let ok =
        match List.assoc_opt "valid" decoded with
        | Some e -> Canon.same_val (Val.Bool got) e
        | None -> false
      in
      {
        problems =
          (if ok then [] else [ cid ^ ": failure id membership disagrees" ]);
        ran_ok = true;
        candidate = Option.value input ~default:"";
        registered = got;
      }
  | _ ->
      (* corpus_reject *)
      let text =
        match List.assoc_opt "text" decoded with
        | Some (Val.Text t) -> t
        | _ -> raise (Case_error "text must be a string")
      in
      done_
        (if Tfv.valid text then
           [ cid ^ ": corpus_reject text was accepted by the strict parser" ]
         else [])

let evaluate_case cfg cid op fields =
  match Safe.recoverable (fun () -> run_case cfg cid op fields) with
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
        candidate = "";
        registered = false;
      }

let string_of_field = function Some (J.String s) -> s | _ -> ""

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

let evaluate ?(config = Types.default_config)
    ?(expected = Some pbt_v1_case_count) cases_in =
  let count_problem =
    match expected with
    | Some n when List.length cases_in <> n ->
        [
          Printf.sprintf "pbt-v1: expected %d cases, got %d" n
            (List.length cases_in);
        ]
    | _ -> []
  in
  let too_many = List.compare_length_with cases_in max_corpus_cases > 0 in
  let cap_problem =
    if too_many then
      [
        Printf.sprintf "pbt-v1: more than %d cases exceeds the cap"
          max_corpus_cases;
      ]
    else []
  in
  let cases =
    if too_many then List.filteri (fun i _ -> i < max_corpus_cases) cases_in
    else cases_in
  in
  let id_problem = check_ids cases in
  let ops = Hashtbl.create 16 and regs = Hashtbl.create 4 in
  let rejected = ref false and evaluated = ref 0 and problems = ref [] in
  let add p = problems := p :: !problems in
  List.iter
    (function
      | J.Object fields -> (
          match List.assoc_opt "id" fields with
          | Some (J.String idb) when idb <> "" -> (
              let cid = Escape.escape_text false (Escape.clip_text 100 idb) in
              let op = string_of_field (List.assoc_opt "op" fields) in
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
                    let res = evaluate_case config cid op fields in
                    List.iter add res.problems;
                    if res.ran_ok then (
                      if op = "failure_id_valid" then
                        if res.registered then
                          Hashtbl.replace regs res.candidate ()
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
  let missing_ids =
    List.filter (fun i -> not (Hashtbl.mem regs i)) Types.failure_ids
  in
  let final =
    count_problem @ cap_problem @ id_problem @ List.rev !problems
    @ (if missing_ops <> [] then
         [
           "pbt-v1: operations never exercised: "
           ^ String.concat ", " missing_ops;
         ]
       else [])
    @ (if missing_ids <> [] then
         [
           "pbt-v1: registered failure ids missing: "
           ^ String.concat ", " missing_ids;
         ]
       else [])
    @
    if not !rejected then
      [ "pbt-v1: failure_id_valid needs an unregistered-id rejection" ]
    else []
  in
  let final = List.map (Escape.clip_text Values.max_problem_len) final in
  {
    evaluated = !evaluated;
    problems = List.filteri (fun i _ -> i < max_problems) final;
  }

let evaluate_text ?config text =
  match parse_corpus text with
  | Ok cases -> evaluate ?config cases
  | Error e ->
      {
        evaluated = 0;
        problems = [ "corpus does not parse: " ^ Escape.clip_text frag e ];
      }
