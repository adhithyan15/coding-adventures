open Coding_adventures_capability_analyzer

let unwrap = function
  | Ok value -> value
  | Error message -> Alcotest.fail message

let code_root () =
  let rec search remaining directory =
    let marker =
      Filename.concat directory
        "specs/fixtures/ocaml-capability-analyzer-v1/cases.json"
    in
    if Sys.file_exists marker then directory
    else if remaining = 0 then
      Alcotest.fail "could not locate the repository code root"
    else search (remaining - 1) (Filename.dirname directory)
  in
  search 12 (Sys.getcwd ())

let fixture_root () =
  Filename.concat (code_root ()) "specs/fixtures/ocaml-capability-analyzer-v1"

let sorted strings = List.sort_uniq String.compare strings

let detected_strings detections =
  detections
  |> List.map (fun detection -> Capability.to_string detection.capability)
  |> sorted

let banned_strings findings =
  findings
  |> List.map (fun (finding : banned_construct) -> finding.construct)
  |> sorted

let fixture_cases () =
  let path = Filename.concat (fixture_root ()) "cases.json" in
  match Yojson.Safe.from_file path with
  | `Assoc fields -> (
      match List.assoc_opt "cases" fields with
      | Some (`List cases) -> cases
      | _ -> Alcotest.fail "fixture cases must be an array")
  | _ -> Alcotest.fail "fixture root must be an object"

let string_field name fields =
  match List.assoc_opt name fields with
  | Some (`String value) -> value
  | _ -> Alcotest.failf "fixture field %s must be a string" name

let string_list_field name fields =
  match List.assoc_opt name fields with
  | Some (`List values) ->
      List.map
        (function
          | `String value -> value
          | _ -> Alcotest.failf "fixture field %s must contain strings" name)
        values
  | _ -> Alcotest.failf "fixture field %s must be an array" name

let test_behavior_fixture () =
  fixture_cases ()
  |> List.iter (function
       | `Assoc fields ->
           let id = string_field "id" fields in
           let source = string_field "source" fields in
           let kind =
             match string_field "source_kind" fields with
             | "implementation" -> Implementation
             | "interface" -> Interface
             | value -> Alcotest.failf "unknown source kind %s" value
           in
           let detections, banned =
             unwrap (analyze_source ~filename:(id ^ ".ml") kind source)
           in
           Alcotest.(check (list string))
             (id ^ " capabilities")
             (string_list_field "expected_capabilities" fields |> sorted)
             (detected_strings detections);
           Alcotest.(check (list string))
             (id ^ " banned")
             (string_list_field "expected_banned" fields |> sorted)
             (banned_strings banned)
       | _ -> Alcotest.fail "fixture case must be an object")

let pure_manifest =
  {|{
    "version": 1,
    "package": "ocaml/example",
    "capabilities": [],
    "justification": "Pure computation with no operating-system access."
  }|}

let ffi_manifest ~action ~construct =
  Printf.sprintf
    {|{
      "version": 1,
      "package": "ocaml/example",
      "capabilities": [{
        "category": "ffi",
        "action": %S,
        "target": "*",
        "justification": "Uses one explicitly reviewed native boundary."
      }],
      "justification": "Uses one explicitly reviewed native boundary and nothing else.",
      "banned_construct_exceptions": [{
        "construct": %S,
        "language": "ocaml",
        "justification": "Required by the reviewed native boundary implementation."
      }]
    }|}
    action construct

let test_manifest_zero_profile () =
  let manifest = unwrap (parse_manifest pure_manifest) in
  Alcotest.(check string) "package" "ocaml/example" manifest.package;
  Alcotest.(check int) "capabilities" 0 (List.length manifest.capabilities);
  Alcotest.(check int) "exceptions" 0 (List.length manifest.exceptions)

let test_manifest_closed_taxonomy () =
  let invalid_pair =
    {|{
      "version": 1,
      "package": "ocaml/example",
      "capabilities": [{
        "category": "fs",
        "action": "connect",
        "target": "*",
        "justification": "This invalid cross-pair must be rejected."
      }],
      "justification": "This invalid cross-pair must be rejected."
    }|}
  in
  (match parse_manifest invalid_pair with
  | Error _ -> ()
  | Ok _ -> Alcotest.fail "invalid category/action pair was accepted");
  let unknown_category =
    {|{
      "version": 1,
      "package": "ocaml/example",
      "capabilities": [{
        "category": "gpu",
        "action": "read",
        "target": "*",
        "justification": "Unknown categories must fail closed."
      }],
      "justification": "Unknown categories must fail closed."
    }|}
  in
  match parse_manifest unknown_category with
  | Error _ -> ()
  | Ok _ -> Alcotest.fail "unknown category was accepted"

let test_manifest_shape_errors () =
  let invalid_documents =
    [
      "not-json";
      {|{"version": 2, "package": "ocaml/x", "capabilities": [], "justification": "Long enough explanation."}|};
      {|{"version": 1, "package": "go/x", "capabilities": [], "justification": "Long enough explanation."}|};
      {|{"version": 1, "package": "ocaml/x", "capabilities": [], "justification": "short"}|};
      {|{"version": 1, "package": "ocaml/x", "capabilities": [], "justification": "Long enough explanation.", "extra": true}|};
      {|{"version": 1, "package": "ocaml/x", "capabilities": [], "justification": "Long enough explanation.", "banned_construct_exceptions": [{"construct": "external", "language": "go", "justification": "Wrong analyzer language is rejected."}]}|};
    ]
  in
  List.iter
    (fun document ->
      match parse_manifest document with
      | Error _ -> ()
      | Ok _ -> Alcotest.failf "invalid manifest accepted: %s" document)
    invalid_documents

let test_manifest_duplicate_capability () =
  let document =
    {|{
      "version": 1,
      "package": "ocaml/example",
      "capabilities": [
        {"category": "fs", "action": "read", "target": "a", "justification": "Reads the first reviewed source path."},
        {"category": "fs", "action": "read", "target": "a", "justification": "Duplicates the first reviewed source path."}
      ],
      "justification": "Duplicate capability entries fail closed."
    }|}
  in
  match parse_manifest document with
  | Error _ -> ()
  | Ok _ -> Alcotest.fail "duplicate capability was accepted"

let test_evaluate_undeclared_and_declared () =
  let detections, banned =
    unwrap
      (analyze_source ~filename:"sample.ml" Implementation
         "let x = Sys.getenv \"HOME\"\n")
  in
  let empty = unwrap (parse_manifest pure_manifest) in
  let denied = evaluate ~dir:"." ~manifest:empty ~detections ~banned in
  Alcotest.(check bool) "denied" false (passed denied);
  Alcotest.(check (list string))
    "CAP001" [ "CAP001" ]
    (List.map (fun violation -> violation.code) denied.violations);
  let declared =
    unwrap
      (parse_manifest
         {|{
           "version": 1,
           "package": "ocaml/example",
           "capabilities": [{"category": "env", "action": "read", "target": "HOME", "justification": "Reads the selected home environment variable."}],
           "justification": "Reads one reviewed environment variable."
         }|})
  in
  Alcotest.(check bool)
    "declared" true
    (evaluate ~dir:"." ~manifest:declared ~detections ~banned |> passed)

let test_ffi_dual_opt_in () =
  let external_detections, external_banned =
    unwrap
      (analyze_source ~filename:"native.mli" Interface
         "external hash : bytes -> bytes = \"native_hash\"\n")
  in
  let fully_declared =
    unwrap (parse_manifest (ffi_manifest ~action:"call" ~construct:"external"))
  in
  Alcotest.(check bool)
    "external allowed" true
    (evaluate ~dir:"." ~manifest:fully_declared ~detections:external_detections
       ~banned:external_banned
    |> passed);
  let missing_exception =
    unwrap
      (parse_manifest
         {|{
           "version": 1,
           "package": "ocaml/example",
           "capabilities": [{"category": "ffi", "action": "call", "target": "*", "justification": "Calls the reviewed native boundary."}],
           "justification": "Calls the reviewed native boundary without its exception."
         }|})
  in
  let denied =
    evaluate ~dir:"." ~manifest:missing_exception
      ~detections:external_detections ~banned:external_banned
  in
  Alcotest.(check bool) "missing exception" false (passed denied);
  Alcotest.(check (list string))
    "CAP002" [ "CAP002" ]
    (List.map (fun violation -> violation.code) denied.violations)

let test_dynlink_dual_opt_in () =
  let detections, banned =
    unwrap
      (analyze_source ~filename:"plugin.ml" Implementation
         "let load path = Dynlink.loadfile path\n")
  in
  let manifest =
    unwrap
      (parse_manifest
         (ffi_manifest ~action:"load" ~construct:"Dynlink.loadfile"))
  in
  Alcotest.(check bool)
    "Dynlink allowed" true
    (evaluate ~dir:"." ~manifest ~detections ~banned |> passed)

let test_hard_ban_cannot_be_exempted () =
  let detections, banned =
    unwrap
      (analyze_source ~filename:"unsafe.ml" Implementation
         "let x value = Obj.magic value\n")
  in
  let manifest =
    unwrap
      (parse_manifest
         {|{
           "version": 1,
           "package": "ocaml/example",
           "capabilities": [],
           "justification": "Attempts to exempt a hard-banned unsafe construct.",
           "banned_construct_exceptions": [{"construct": "Obj.magic", "language": "ocaml", "justification": "This exception must not authorize unsafe coercion."}]
         }|})
  in
  let result = evaluate ~dir:"." ~manifest ~detections ~banned in
  Alcotest.(check bool) "still denied" false (passed result)

let test_parse_error_fails_closed () =
  match analyze_source ~filename:"broken.ml" Implementation "let =" with
  | Error _ -> ()
  | Ok _ -> Alcotest.fail "invalid OCaml parsed successfully"

let test_directory_analysis () =
  let clean = Filename.concat (fixture_root ()) "package-clean" in
  let denied = Filename.concat (fixture_root ()) "package-violation" in
  let broken = Filename.concat (fixture_root ()) "package-parse-error" in
  let clean_result = unwrap (analyze_directory clean) in
  Alcotest.(check bool) "clean package" true (passed clean_result);
  let denied_result = unwrap (analyze_directory denied) in
  Alcotest.(check bool) "violation package" false (passed denied_result);
  Alcotest.(check (list string))
    "violation code" [ "CAP001" ]
    (List.map (fun violation -> violation.code) denied_result.violations);
  Alcotest.(check (list string))
    "deterministic relative paths"
    [ "src/nested/violation.ml" ]
    (denied_result.detected
    |> List.map (fun (detection : detection) -> detection.file)
    |> sorted);
  match analyze_directory broken with
  | Error _ -> ()
  | Ok _ -> Alcotest.fail "directory parse failure was ignored"

let test_cli_results () =
  let clean = Filename.concat (fixture_root ()) "package-clean" in
  let denied = Filename.concat (fixture_root ()) "package-violation" in
  let capture dir verbose =
    let output = Buffer.create 128 and errors = Buffer.create 128 in
    let code =
      run ~dir ~verbose ~stdout:(Buffer.add_string output)
        ~stderr:(Buffer.add_string errors)
    in
    (code, Buffer.contents output, Buffer.contents errors)
  in
  let code, output, errors = capture clean true in
  Alcotest.(check int) "clean exit" 0 code;
  Alcotest.(check bool) "verbose output" true (String.length output > 0);
  Alcotest.(check string) "clean stderr" "" errors;
  let code, output, _ = capture denied false in
  Alcotest.(check int) "violation exit" 1 code;
  Alcotest.(check bool) "violation output" true (String.length output > 0);
  let code, _, errors =
    capture (Filename.concat (fixture_root ()) "missing") false
  in
  Alcotest.(check int) "error exit" 2 code;
  Alcotest.(check bool) "error output" true (String.length errors > 0)

let test_format_and_sorting () =
  let detections, banned =
    unwrap
      (analyze_source ~filename:"z.ml" Implementation
         "let b () = print_endline \"b\"\nlet a () = Sys.getenv \"A\"\n")
  in
  let result =
    evaluate ~dir:"."
      ~manifest:(unwrap (parse_manifest pure_manifest))
      ~detections ~banned
  in
  Alcotest.(check (list string))
    "stable codes" [ "CAP001"; "CAP001" ]
    (List.map (fun violation -> violation.code) result.violations);
  let messages = List.map format_violation result.violations in
  Alcotest.(check bool)
    "source location" true
    (List.for_all
       (fun message -> String.starts_with ~prefix:"z.ml:" message)
       messages)

let () =
  Alcotest.run "OCaml capability analyzer"
    [
      ( "source analysis",
        [
          Alcotest.test_case "shared behavior fixture" `Quick
            test_behavior_fixture;
          Alcotest.test_case "parse errors fail closed" `Quick
            test_parse_error_fails_closed;
          Alcotest.test_case "stable formatting" `Quick test_format_and_sorting;
        ] );
      ( "manifests",
        [
          Alcotest.test_case "zero profile" `Quick test_manifest_zero_profile;
          Alcotest.test_case "closed taxonomy" `Quick
            test_manifest_closed_taxonomy;
          Alcotest.test_case "shape errors" `Quick test_manifest_shape_errors;
          Alcotest.test_case "duplicate capability" `Quick
            test_manifest_duplicate_capability;
        ] );
      ( "policy",
        [
          Alcotest.test_case "declared capabilities" `Quick
            test_evaluate_undeclared_and_declared;
          Alcotest.test_case "FFI dual opt-in" `Quick test_ffi_dual_opt_in;
          Alcotest.test_case "Dynlink dual opt-in" `Quick
            test_dynlink_dual_opt_in;
          Alcotest.test_case "hard bans" `Quick test_hard_ban_cannot_be_exempted;
        ] );
      ( "integration",
        [
          Alcotest.test_case "directory" `Quick test_directory_analysis;
          Alcotest.test_case "CLI" `Quick test_cli_results;
        ] );
    ]
