open Microbenchmark

let check = Alcotest.check
let float = Alcotest.float 1e-12

let unwrap = function
  | Ok value -> value
  | Error message -> Alcotest.fail message

let assert_error label result =
  Alcotest.check Alcotest.bool label true (Result.is_error result)

let rec locate_root directory =
  let candidate =
    Filename.concat directory "code/specs/fixtures/microbenchmark-pairs-v1.json"
  in
  if Sys.file_exists candidate then Some candidate
  else
    let parent = Filename.dirname directory in
    if parent = directory then None else locate_root parent

let get_field name = function
  | `Assoc fields -> (
      match List.assoc_opt name fields with
      | Some value -> value
      | None -> Alcotest.fail ("missing fixture field " ^ name))
  | _ -> Alcotest.fail "fixture row must be an object"

let int64_json = function
  | `Int n -> Int64.of_int n
  | `Intlit n -> Int64.of_string n
  | _ -> Alcotest.fail "fixture integer expected"

let float_json = function
  | `Float n -> n
  | `Int n -> float_of_int n
  | _ -> Alcotest.fail "fixture float expected"

let contains source needle =
  let source_length = String.length source
  and needle_length = String.length needle in
  let rec search index =
    index + needle_length <= source_length
    && (String.sub source index needle_length = needle || search (index + 1))
  in
  search 0

let replace_once_raw ~needle ~replacement source =
  if
    String.length source < 2
    || source.[0] <> '{'
    || source.[String.length source - 1] <> '}'
  then Alcotest.fail "mutation source must be the complete encoded report";
  let needle_length = String.length needle in
  let rec find index =
    if index + needle_length > String.length source then
      Alcotest.fail ("mutation needle not present: " ^ needle)
    else if String.sub source index needle_length = needle then
      String.sub source 0 index ^ replacement
      ^ String.sub source (index + needle_length)
          (String.length source - index - needle_length)
    else find (index + 1)
  in
  find 0

let replace_once needle replacement source =
  let mutated = replace_once_raw ~needle ~replacement source in
  (* A rejected fragment is not evidence that a schema mutation was rejected.
     Every mutation below must retain a syntactically valid complete object. *)
  (match Yojson.Safe.from_string mutated with
  | `Assoc _ -> ()
  | _ -> Alcotest.fail "mutation did not retain a complete report object");
  mutated

let test_fixture () =
  let local_path = "../testdata/microbenchmark-pairs-v1.json" in
  let local = In_channel.with_open_bin local_path In_channel.input_all in
  (match locate_root (Sys.getcwd ()) with
  | Some canonical ->
      let expected = In_channel.with_open_bin canonical In_channel.input_all in
      check Alcotest.string "fixture bytes match canonical" expected local
  | None -> ());
  let root = Yojson.Safe.from_string local in
  let confidence =
    int64_json (get_field "confidence_percent" root) |> Int64.to_int
  in
  let factor = float_json (get_field "minimum_factor" root) in
  let cases =
    match get_field "cases" root with
    | `List values -> values
    | _ -> Alcotest.fail "fixture cases must be array"
  in
  List.iter
    (fun row ->
      let baseline = int64_json (get_field "baseline_ns" row) in
      let candidates =
        match get_field "candidate_ns" row with
        | `List values -> values
        | _ -> Alcotest.fail "candidate_ns must be array"
      in
      let pairs =
        List.mapi
          (fun index value ->
            {
              baseline_ns = baseline;
              candidate_ns = int64_json value;
              order =
                (if index mod 2 = 0 then Baseline_first else Candidate_first);
            })
          candidates
      in
      let actual = unwrap (analyze pairs confidence factor) in
      check float "fixture ratio"
        (float_json (get_field "ratio" row))
        actual.ratio;
      check Alcotest.string "fixture verdict"
        (match get_field "verdict" row with `String value -> value | _ -> "")
        actual.verdict;
      let expected_interval =
        match get_field "interval" row with
        | `Null -> None
        | `List [ low; high ] -> Some (float_json low, float_json high)
        | _ -> Alcotest.fail "fixture interval malformed"
      in
      let actual_interval =
        Option.map (fun value -> (value.low, value.high)) actual.interval
      in
      check
        (Alcotest.option (Alcotest.pair float float))
        "fixture interval" expected_interval actual_interval)
    cases

let scripted_clock values =
  let remaining = ref values in
  fun () ->
    match !remaining with
    | value :: rest ->
        remaining := rest;
        value
    | [] -> ( match List.rev values with last :: _ -> last | [] -> 0L)

let fake_increment_clock step =
  let current = ref 0L in
  fun () ->
    let previous = !current in
    current := Int64.add !current step;
    previous

let test_collector () =
  let config =
    {
      default_config with
      pairs = 3;
      target_ns = 100L;
      max_batch = 8L;
      seed = -17L;
    }
  in
  let batch operations = Ok (Int64.mul 6L operations) in
  let report =
    unwrap (collect config 6L batch batch (fake_increment_clock 100L))
  in
  check Alcotest.int "selected batch" 1 (Int64.to_int report.batch_size);
  check Alcotest.int "raw pair retention" 3 (List.length report.pairs);
  check Alcotest.int "odd order has baseline-first extra" 2
    (List.length
       (List.filter (fun pair -> pair.order = Baseline_first) report.pairs));
  let replayed =
    unwrap (collect config 6L batch batch (fake_increment_clock 100L))
  in
  check
    (Alcotest.list Alcotest.bool)
    "seeded schedule repeat"
    (List.map (fun (p : pair) -> p.order = Baseline_first) report.pairs)
    (List.map (fun (p : pair) -> p.order = Baseline_first) replayed.pairs);
  assert_error "checksum mismatch at warmup"
    (collect config 6L (fun _ -> Ok 0L) batch (fake_increment_clock 100L));
  assert_error "workload failure"
    (collect config 6L
       (fun _ -> Error "failed")
       batch
       (fake_increment_clock 100L));
  assert_error "calibration exhaustion"
    (collect
       { config with pairs = 1; target_ns = 1000L; max_batch = 1L }
       6L batch batch (fake_increment_clock 10L));
  assert_error "zero timed duration"
    (collect
       { config with pairs = 1; max_batch = 1L }
       6L batch batch
       (scripted_clock [ 0L; 100L; 100L; 200L; 200L; 200L ]));
  assert_error "regressing clock"
    (collect
       { config with pairs = 1; max_batch = 1L }
       6L batch batch
       (scripted_clock [ 0L; 100L; 90L ]));
  assert_error "checksum multiplication overflow"
    (collect
       { config with pairs = 1; target_ns = 200L; max_batch = 2L }
       Int64.max_int
       (fun _ -> Ok Int64.max_int)
       (fun _ -> Ok Int64.max_int)
       (fake_increment_clock 100L))

let test_replay () =
  let config =
    {
      default_config with
      pairs = 1;
      target_ns = 1L;
      max_batch = Int64.max_int;
      seed = Int64.min_int;
    }
  in
  let pair =
    {
      baseline_ns = Int64.max_int;
      candidate_ns = Int64.max_int;
      order = Baseline_first;
    }
  in
  let analysis = unwrap (analyze [ pair ] 95 1.05) in
  let metadata =
    default_metadata [ "--pairs"; "1" ] "revision" "OCaml 5.2.1" "windows"
      "x86_64"
  in
  let report =
    {
      config;
      batch_size = 1L;
      pairs = [ pair ];
      analysis;
      metadata = Some metadata;
    }
  in
  let json = encode_report report in
  Alcotest.check Alcotest.bool "signed max token emitted" true
    (contains json "9223372036854775807");
  Alcotest.check Alcotest.bool "signed min token emitted" true
    (contains json "-9223372036854775808");
  check Alcotest.string "i64 report round trip" json
    (encode_report (unwrap (replay_report json)));
  assert_error "duplicate key"
    (replay_report
       (replace_once "\"seed\":-9223372036854775808"
          "\"seed\":-9223372036854775808,\"seed\":0" json));
  assert_error "escaped duplicate key"
    (replay_report
       (replace_once "\"seed\":-9223372036854775808"
          "\"seed\":-9223372036854775808,\"\\u0073eed\":0" json));
  assert_error "fractional integer token"
    (replay_report
       (replace_once "\"seed\":-9223372036854775808"
          "\"seed\":-9223372036854775808.0" json));
  assert_error "present null metadata"
    (replay_report
       (replace_once "\"workload\":\"indexed-array-sum-4096\""
          "\"workload\":null" json));
  let with_workload token =
    replace_once_raw ~needle:"\"workload\":\"indexed-array-sum-4096\""
      ~replacement:("\"workload\":" ^ token) json
  in
  let accepted_workloads =
    [
      ("\"indexed-array-sum-4096\"", "indexed-array-sum-4096");
      ("\"dependent-link-order\"", "dependent-link-order");
      ("\"\\u2003\"", "\226\128\131");
      ("\"\\ud83d\\udca1\"", "\240\159\146\161");
      ("\"\\ufffd\"", "\239\191\189");
      ("\"\\\\ud800\"", "\\ud800");
    ]
  in
  List.iter
    (fun (token, expected) ->
      let decoded = unwrap (replay_report (with_workload token)) in
      match decoded.metadata with
      | Some value ->
          Alcotest.check Alcotest.string "workload preserved" expected
            value.workload
      | None -> Alcotest.fail "expected metadata")
    accepted_workloads;
  let bracketed_workload =
    String.make 130 '[' ^ " \\\"[{}]" ^ String.make 130 ']'
  in
  let bracketed_token = Yojson.Safe.to_string (`String bracketed_workload) in
  (match (unwrap (replay_report (with_workload bracketed_token))).metadata with
  | Some value ->
      Alcotest.check Alcotest.string
        "quoted brackets and escaped quotes are metadata" bracketed_workload
        value.workload
  | None -> Alcotest.fail "expected metadata");
  let invalid_workloads =
    [
      "\"\"";
      "\" \"";
      "\"\\t\"";
      "\"\\n\"";
      "\"\\u000b\"";
      "\"\\f\"";
      "\"\\r\"";
      "\"\\u0000\"";
      "\"bad\\u0000id\"";
      "null";
      "17";
      "true";
      "[]";
      "{}";
      "\"\\ud800\"";
      "\"\\udfff\"";
    ]
  in
  List.iter
    (fun token ->
      assert_error
        ("invalid workload: " ^ token)
        (replay_report (with_workload token)))
    invalid_workloads;
  assert_error "unknown field"
    (replay_report
       (replace_once "\"batch_size\":1" "\"extra\":1,\"batch_size\":1" json));
  ignore
    (unwrap (replay_report (encode_report { report with metadata = None })));
  assert_error "JSON byte bound"
    (replay_report (String.make ((4 * 1024 * 1024) + 1) ' '));
  assert_error "nesting bound"
    (replay_report (String.make 129 '[' ^ String.make 129 ']'));
  let too_many_values =
    "[" ^ String.concat "," (List.init 65_536 (fun _ -> "0")) ^ "]"
  in
  (match replay_report too_many_values with
  | Error message ->
      Alcotest.check Alcotest.string "preflight value count"
        "report exceeds JSON value limit" message
  | Ok _ -> Alcotest.fail "JSON value budget was not enforced before decoding");
  let pairs = [ pair; { pair with order = Baseline_first } ] in
  let analysis2 = unwrap (analyze pairs 95 1.05) in
  assert_error "unbalanced report"
    (replay_report
       (encode_report
          {
            report with
            config = { config with pairs = 2 };
            pairs;
            analysis = analysis2;
            metadata = None;
          }));
  let many_unknown_fields =
    "{"
    ^ String.concat ","
        (List.init 16_000 (fun index -> Printf.sprintf "\"x%d\":0" index))
    ^ "}"
  in
  assert_error "many unique object keys rejected linearly"
    (replay_report many_unknown_fields);
  let many_pairs =
    List.init 10_000 (fun index ->
        {
          pair with
          order = (if index mod 2 = 0 then Baseline_first else Candidate_first);
        })
  in
  let many_config = { default_config with pairs = 10_000 } in
  let many_analysis = unwrap (analyze many_pairs 95 1.05) in
  let many_report =
    {
      report with
      config = many_config;
      pairs = many_pairs;
      analysis = many_analysis;
      metadata =
        Some
          (default_metadata
             (List.init 256 (fun _ -> "arg"))
             bracketed_workload "OCaml 5.2.1" "linux" "x86_64");
    }
  in
  ignore (unwrap (replay_report (encode_report many_report)))

let test_metadata_contract () =
  let config =
    {
      pairs = 1;
      confidence_percent = 95;
      minimum_factor = 1.05;
      seed = 0L;
      target_ns = 1L;
      max_batch = 1L;
    }
  in
  let pair = { baseline_ns = 1L; candidate_ns = 1L; order = Baseline_first } in
  let analysis = unwrap (analyze [ pair ] 95 1.05) in
  let metadata = default_metadata [ "" ] "rev" "OCaml" "test" "x64" in
  let report =
    {
      config;
      batch_size = 1L;
      pairs = [ pair ];
      analysis;
      metadata = Some metadata;
    }
  in
  ignore (unwrap (replay_report (encode_report report)));
  let whitespace_revision =
    { report with metadata = Some { metadata with revision = " " } }
  in
  ignore (unwrap (replay_report (encode_report whitespace_revision)));
  let nul_revision =
    { report with metadata = Some { metadata with revision = "bad\000value" } }
  in
  assert_error "NUL descriptive metadata rejected"
    (replay_report (encode_report nul_revision));
  let nul_invocation =
    {
      report with
      metadata = Some { metadata with invocation = [ "bad\000value" ] };
    }
  in
  assert_error "NUL invocation rejected"
    (replay_report (encode_report nul_invocation));
  let nul_profile =
    {
      report with
      metadata = Some { metadata with profile = Some "bad\000value" };
    }
  in
  assert_error "NUL profile rejected"
    (replay_report (encode_report nul_profile));
  let empty_profile =
    { report with metadata = Some { metadata with profile = Some "" } }
  in
  assert_error "empty profile rejected"
    (replay_report (encode_report empty_profile))

let test_workload () =
  let custom = [| 21; -3; 8; 19 |] in
  check Alcotest.int64 "indexed custom array" 90L
    (unwrap (Array_workload.indexed_batch custom 2L));
  check Alcotest.int64 "iterator custom array" 90L
    (unwrap (Array_workload.iteration_batch custom 2L));
  check Alcotest.int64 "zero operations" 0L
    (unwrap (Array_workload.indexed_batch custom 0L));
  assert_error "negative indexed count"
    (Array_workload.indexed_batch custom (-1L));
  assert_error "negative iteration count"
    (Array_workload.iteration_batch custom (-1L));
  let large = [| max_int; max_int |] in
  assert_error "sum overflow rejected" (Array_workload.indexed_batch large 2L);
  let values = Array_workload.make_array () in
  check Alcotest.int64 "same array checksum" Array_workload.expected_checksum
    (unwrap (Array_workload.indexed_batch values 1L));
  check Alcotest.int64 "iteration checksum" Array_workload.expected_checksum
    (unwrap (Array_workload.iteration_batch values 1L))

let test_native_clock () =
  let first = Native_clock.now_ns () in
  let second = Native_clock.now_ns () in
  Alcotest.check Alcotest.bool "native monotonic wall clock" true
    (first >= 0L && second >= first)

let test_public_domains_and_failure_positions () =
  let pair =
    { baseline_ns = 10L; candidate_ns = 10L; order = Baseline_first }
  in
  assert_error "empty analysis" (analyze [] 95 2.);
  assert_error "analysis pair bound"
    (analyze (List.init 10_001 (fun _ -> pair)) 95 2.);
  List.iter
    (fun confidence ->
      assert_error "confidence bounds" (analyze [ pair ] confidence 2.))
    [ 0; 100 ];
  List.iter
    (fun factor ->
      assert_error "finite practical factor" (analyze [ pair ] 95 factor))
    [ nan; infinity; neg_infinity; 0.; 1. ];
  List.iter
    (fun p -> assert_error "positive durations" (analyze [ p ] 95 2.))
    [ { pair with baseline_ns = 0L }; { pair with candidate_ns = -1L } ];
  let config =
    {
      default_config with
      pairs = 6;
      target_ns = 1L;
      max_batch = 1L;
      minimum_factor = 2.;
    }
  in
  List.iter
    (fun invalid ->
      let called = ref false in
      let batch _ =
        called := true;
        Ok 1L
      in
      assert_error "invalid config"
        (collect invalid 1L batch batch (fun () ->
             called := true;
             0L));
      check Alcotest.bool "validate before invoking dependencies" false !called)
    [
      { config with pairs = 0 };
      { config with pairs = 10_001 };
      { config with confidence_percent = 0 };
      { config with confidence_percent = 100 };
      { config with minimum_factor = nan };
      { config with minimum_factor = infinity };
      { config with minimum_factor = 1. };
      { config with target_ns = 0L };
      { config with max_batch = 0L };
    ];
  let reads = ref 0 in
  let clock () =
    let tick = Int64.of_int (!reads * 10) in
    incr reads;
    tick
  in
  ignore (unwrap (collect config 1L (fun n -> Ok n) (fun n -> Ok n) clock));
  for failure = 0 to !reads - 1 do
    let index = ref 0 in
    let now () =
      let current = !index in
      incr index;
      Int64.of_int ((current * 10) - if current = failure then 11 else 0)
    in
    assert_error "regression at each clock boundary"
      (collect config 1L (fun n -> Ok n) (fun n -> Ok n) now)
  done;
  let calls = ref 0 in
  let success n =
    incr calls;
    Ok n
  in
  ignore (unwrap (collect config 1L success success (fake_increment_clock 10L)));
  for failure = 1 to !calls do
    let index = ref 0 in
    let batch n =
      incr index;
      if !index = failure then Error "injected workload failure" else Ok n
    in
    assert_error "workload failure at every phase"
      (collect config 1L batch batch (fake_increment_clock 10L))
  done;
  List.iter
    (fun batch ->
      check Alcotest.int64 "empty array" 0L (unwrap (batch [||] 2L));
      check Alcotest.int64 "zero valid operations" 0L
        (unwrap (batch [| 2; -1 |] 0L));
      assert_error "positive accumulation overflow"
        (batch [| max_int; max_int |] 2L);
      assert_error "negative accumulation overflow"
        (batch [| min_int; min_int |] 2L))
    [ Array_workload.indexed_batch; Array_workload.iteration_batch ]

let set_json key value = function
  | `Assoc fields -> `Assoc ((key, value) :: List.remove_assoc key fields)
  | _ -> Alcotest.fail "test fixture must be an object"

let remove_json key = function
  | `Assoc fields -> `Assoc (List.remove_assoc key fields)
  | _ -> Alcotest.fail "test fixture must be an object"

let change_json key change root =
  set_json key (change (get_field key root)) root

let test_schema_mutations () =
  (* Equal durations independently imply ratio/interval one. Six pairs at 95%
     have the min/max interval: its two-tail failure probability is 2/64. *)
  let config =
    {
      default_config with
      pairs = 6;
      minimum_factor = 2.;
      target_ns = 1L;
      max_batch = 1L;
    }
  in
  let pairs =
    List.init 6 (fun i ->
        {
          baseline_ns = 100L;
          candidate_ns = 100L;
          order = (if i mod 2 = 0 then Baseline_first else Candidate_first);
        })
  in
  let report =
    {
      config;
      batch_size = 1L;
      pairs;
      analysis =
        {
          ratio = 1.;
          interval = Some { low = 1.; high = 1. };
          verdict = "equivalent";
        };
      metadata = None;
    }
  in
  let base = Yojson.Safe.from_string (encode_report report) in
  ignore (unwrap (replay_report (Yojson.Safe.to_string base)));
  let reject label value =
    assert_error label (replay_report (Yojson.Safe.to_string value))
  in
  List.iter
    (fun key -> reject "missing root field" (remove_json key base))
    [ "config"; "batch_size"; "pairs"; "analysis" ];
  List.iter
    (fun key ->
      reject "missing config field"
        (change_json "config" (remove_json key) base);
      reject "null config field"
        (change_json "config" (set_json key `Null) base))
    [
      "pairs";
      "confidence_percent";
      "minimum_factor";
      "seed";
      "target_ns";
      "max_batch";
    ];
  List.iter
    (fun key ->
      List.iter
        (fun wrong ->
          reject "integer field type"
            (change_json "config" (set_json key wrong) base))
        [ `String "1"; `Bool true; `Float 1. ])
    [ "pairs"; "confidence_percent"; "seed"; "target_ns"; "max_batch" ];
  List.iter
    (fun key ->
      reject "signed integer overflow"
        (change_json "config"
           (set_json key (`Intlit "9223372036854775808"))
           base))
    [ "seed"; "target_ns"; "max_batch" ];
  List.iter
    (fun (key, value) ->
      reject "config semantic bound"
        (change_json "config" (set_json key value) base))
    [
      ("pairs", `Int 0);
      ("pairs", `Int 10_001);
      ("confidence_percent", `Int 100);
      ("minimum_factor", `Int 1);
      ("target_ns", `Int 0);
      ("max_batch", `Int 0);
    ];
  reject "unknown config field"
    (change_json "config" (set_json "extra" (`Int 1)) base);
  List.iter
    (fun wrong -> reject "batch bounds/type" (set_json "batch_size" wrong base))
    [ `Int 0; `Int 2; `Float 1.; `String "1" ];
  let change_pair f =
    change_json "pairs"
      (function
        | `List (first :: rest) -> `List (f first :: rest)
        | _ -> Alcotest.fail "pair fixture")
      base
  in
  List.iter
    (fun key -> reject "missing pair field" (change_pair (remove_json key)))
    [ "baseline_ns"; "candidate_ns"; "order" ];
  List.iter
    (fun key ->
      List.iter
        (fun wrong ->
          reject "duration bounds/type" (change_pair (set_json key wrong)))
        [ `Int 0; `Int (-1); `Float 1.; `Null ])
    [ "baseline_ns"; "candidate_ns" ];
  reject "unknown pair field" (change_pair (set_json "extra" (`Int 1)));
  reject "invalid order spelling"
    (change_pair (set_json "order" (`String "other")));
  reject "pair count mismatch" (set_json "pairs" (`List []) base);
  List.iter
    (fun key ->
      reject "missing analysis field"
        (change_json "analysis" (remove_json key) base))
    [ "ratio"; "interval"; "verdict" ];
  List.iter
    (fun (key, wrong) ->
      reject "forged analysis"
        (change_json "analysis" (set_json key wrong) base))
    [
      ("ratio", `Null);
      ("ratio", `Int 2);
      ("verdict", `String "wrong");
      ("interval", `Null);
      ("interval", `List []);
      ("interval", `List [ `Int 1 ]);
      ("interval", `List [ `Int 1; `Int 1; `Int 1 ]);
      ("interval", `List [ `Int 1; `Int 2 ]);
    ];
  reject "unknown analysis field"
    (change_json "analysis" (set_json "extra" (`Int 1)) base);
  let metadata =
    default_metadata [ "cli" ] "revision" "OCaml 5.2.1" "windows" "x86_64"
  in
  let with_meta =
    Yojson.Safe.from_string
      (encode_report { report with metadata = Some metadata })
  in
  List.iter
    (fun key -> reject "incomplete metadata" (remove_json key with_meta))
    [ "workload"; "invocation"; "revision"; "runtime"; "os"; "arch" ];
  List.iter
    (fun key ->
      List.iter
        (fun wrong ->
          reject "metadata value type" (set_json key wrong with_meta))
        [ `Null; `Int 1 ])
    [ "workload"; "revision"; "runtime"; "os"; "arch" ];
  List.iter
    (fun wrong ->
      reject "invocation shape" (set_json "invocation" wrong with_meta))
    [ `Null; `String "cli"; `List [ `Int 1 ] ];
  ignore
    (unwrap
       (replay_report
          (Yojson.Safe.to_string (set_json "invocation" (`List []) with_meta))));
  ignore
    (unwrap
       (replay_report
          (Yojson.Safe.to_string
             (set_json "invocation" (`List [ `String "" ]) with_meta))));
  reject "profile without metadata"
    (set_json "profile" (`String "release") base);
  reject "null profile" (set_json "profile" `Null with_meta)

let () =
  Alcotest.run "microbenchmark"
    [
      ( "fixture",
        [ Alcotest.test_case "shared analysis fixture" `Quick test_fixture ] );
      ( "collection",
        [ Alcotest.test_case "injected clock and errors" `Quick test_collector ]
      );
      ( "replay",
        [
          Alcotest.test_case "full-width strict report" `Quick test_replay;
          Alcotest.test_case "portable metadata contract" `Quick
            test_metadata_contract;
          Alcotest.test_case "complete schema mutations" `Quick
            test_schema_mutations;
        ] );
      ( "workload",
        [
          Alcotest.test_case "same prepared mutable input" `Quick test_workload;
        ] );
      ( "clock",
        [
          Alcotest.test_case "native monotonic source" `Quick test_native_clock;
        ] );
      ( "domains",
        [
          Alcotest.test_case "public bounds and every failure phase" `Quick
            test_public_domains_and_failure_positions;
        ] );
    ]
