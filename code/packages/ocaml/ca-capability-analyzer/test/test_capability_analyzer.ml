open Coding_adventures_capability_analyzer

let unwrap = function
  | Ok value -> value
  | Error message -> Alcotest.fail message

let package_root () =
  let rec search remaining directory =
    let marker =
      Filename.concat directory
        "test/fixtures/ocaml-capability-analyzer-v1/cases.json"
    in
    if Sys.file_exists marker then directory
    else if remaining = 0 then
      Alcotest.fail "could not locate the package fixture root"
    else search (remaining - 1) (Filename.dirname directory)
  in
  search 12 (Sys.getcwd ())

let fixture_root () =
  Filename.concat (package_root ()) "test/fixtures/ocaml-capability-analyzer-v1"

let sorted strings = List.sort_uniq String.compare strings

let contains_substring text needle =
  let text_length = String.length text and needle_length = String.length needle in
  let rec search index =
    index + needle_length <= text_length
    &&
    (String.sub text index needle_length = needle || search (index + 1))
  in
  needle_length = 0 || search 0

let write_file path contents =
  Out_channel.with_open_bin path (fun channel ->
      Out_channel.output_string channel contents)

let rec remove_tree path =
  match (Unix.lstat path).st_kind with
  | Unix.S_DIR ->
      Sys.readdir path
      |> Array.iter (fun name -> remove_tree (Filename.concat path name));
      Unix.rmdir path
  | _ -> Sys.remove path

let with_temp_directory callback =
  let directory = Filename.temp_file "ocaml-capability-" "-test" in
  Sys.remove directory;
  Unix.mkdir directory 0o700;
  Fun.protect
    ~finally:(fun () -> remove_tree directory)
    (fun () -> callback directory)

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
      {|{"$schema": 1, "version": 1, "package": "ocaml/x", "capabilities": [], "justification": "Long enough explanation."}|};
      {|{"version": 1, "package": "ocaml/x", "capabilities": [], "justification": "Long enough explanation.", "banned_construct_exceptions": [{"construct": "external", "language": "go", "justification": "Wrong analyzer language is rejected."}]}|};
    ]
  in
  List.iter
    (fun document ->
      match parse_manifest document with
      | Error _ -> ()
      | Ok _ -> Alcotest.failf "invalid manifest accepted: %s" document)
    invalid_documents

let test_manifest_schema_boundaries () =
  let expect_error name document =
    match parse_manifest document with
    | Error _ -> ()
    | Ok _ -> Alcotest.failf "%s: invalid manifest was accepted" name
  in
  let valid_justification = "This explanation is deliberately long enough." in
  let manifest capabilities exceptions =
    Printf.sprintf
      {|{"version":1,"package":"ocaml/example","capabilities":%s,"justification":%S%s}|}
      capabilities valid_justification exceptions
  in
  let capability fields = "[{" ^ fields ^ "}]" in
  [
    ("root array", "[]");
    ("missing version", {|{"package":"ocaml/example","capabilities":[],"justification":"Long enough explanation."}|});
    ("non-string schema", {|{"$schema":1,"version":1,"package":"ocaml/example","capabilities":[],"justification":"Long enough explanation."}|});
    ("non-string package", {|{"version":1,"package":7,"capabilities":[],"justification":"Long enough explanation."}|});
    ("capabilities object", {|{"version":1,"package":"ocaml/example","capabilities":{},"justification":"Long enough explanation."}|});
    ("capability scalar", manifest "[1]" "");
    ("missing category", manifest (capability {|"action":"read","target":"*","justification":"Long enough explanation."|}) "");
    ("category type", manifest (capability {|"category":1,"action":"read","target":"*","justification":"Long enough explanation."|}) "");
    ("missing action", manifest (capability {|"category":"fs","target":"*","justification":"Long enough explanation."|}) "");
    ("action type", manifest (capability {|"category":"fs","action":1,"target":"*","justification":"Long enough explanation."|}) "");
    ("missing target", manifest (capability {|"category":"fs","action":"read","justification":"Long enough explanation."|}) "");
    ("empty target", manifest (capability {|"category":"fs","action":"read","target":" ","justification":"Long enough explanation."|}) "");
    ("target type", manifest (capability {|"category":"fs","action":"read","target":1,"justification":"Long enough explanation."|}) "");
    ("missing capability justification", manifest (capability {|"category":"fs","action":"read","target":"*"|}) "");
    ("capability justification type", manifest (capability {|"category":"fs","action":"read","target":"*","justification":1|}) "");
    ("short capability justification", manifest (capability {|"category":"fs","action":"read","target":"*","justification":"short"|}) "");
    ("unknown capability field", manifest (capability {|"category":"fs","action":"read","target":"*","justification":"Long enough explanation.","extra":true|}) "");
    ("duplicate capability field", manifest (capability {|"category":"fs","category":"fs","action":"read","target":"*","justification":"Long enough explanation."|}) "");
    ("exceptions object", manifest "[]" {|,"banned_construct_exceptions":{}|});
    ("exception scalar", manifest "[]" {|,"banned_construct_exceptions":[1]|});
    ("missing exception construct", manifest "[]" {|,"banned_construct_exceptions":[{"language":"ocaml","justification":"Long enough explanation."}]|});
    ("empty exception construct", manifest "[]" {|,"banned_construct_exceptions":[{"construct":" ","language":"ocaml","justification":"Long enough explanation."}]|});
    ("missing exception language", manifest "[]" {|,"banned_construct_exceptions":[{"construct":"external","justification":"Long enough explanation."}]|});
    ("exception language type", manifest "[]" {|,"banned_construct_exceptions":[{"construct":"external","language":1,"justification":"Long enough explanation."}]|});
    ("missing exception justification", manifest "[]" {|,"banned_construct_exceptions":[{"construct":"external","language":"ocaml"}]|});
    ("exception justification type", manifest "[]" {|,"banned_construct_exceptions":[{"construct":"external","language":"ocaml","justification":1}]|});
    ("short exception justification", manifest "[]" {|,"banned_construct_exceptions":[{"construct":"external","language":"ocaml","justification":"short"}]|});
    ("unknown exception field", manifest "[]" {|,"banned_construct_exceptions":[{"construct":"external","language":"ocaml","justification":"Long enough explanation.","extra":true}]|});
    ("duplicate exception field", manifest "[]" {|,"banned_construct_exceptions":[{"construct":"external","construct":"external","language":"ocaml","justification":"Long enough explanation."}]|});
    ("duplicate exception", manifest "[]" {|,"banned_construct_exceptions":[{"construct":"external","language":"ocaml","justification":"Long enough explanation."},{"construct":"external","language":"ocaml","justification":"Another long enough explanation."}]|});
  ]
  |> List.iter (fun (name, document) -> expect_error name document);
  let all_taxonomy_pairs =
    [
      ("fs", "read");
      ("fs", "write");
      ("fs", "create");
      ("fs", "delete");
      ("fs", "list");
      ("net", "connect");
      ("net", "listen");
      ("net", "dns");
      ("proc", "exec");
      ("proc", "fork");
      ("proc", "signal");
      ("env", "read");
      ("env", "write");
      ("ffi", "call");
      ("ffi", "load");
      ("time", "read");
      ("time", "sleep");
      ("stdin", "read");
      ("stdout", "write");
    ]
    |> List.map (fun (category, action) ->
           Printf.sprintf
             {|{"category":%S,"action":%S,"target":"*","justification":"Long enough capability explanation."}|}
             category action)
    |> String.concat ","
  in
  let parsed = unwrap (parse_manifest (manifest ("[" ^ all_taxonomy_pairs ^ "]") "")) in
  Alcotest.(check int) "all taxonomy arms" 19
    (List.length parsed.capabilities);
  let digit_package =
    unwrap
      (parse_manifest
         {|{
           "$schema": "../../../../../capability-schema.json",
           "version": 1,
           "package": "ocaml/9a_b-c",
           "capabilities": [],
           "justification": "Exercises every permitted package-name character class."
         }|})
  in
  Alcotest.(check string) "digit package" "ocaml/9a_b-c" digit_package.package

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

let test_manifest_package_name_boundary () =
  match
    parse_manifest
      {|{
        "version": 1,
        "package": "ocaml/-invalid",
        "capabilities": [],
        "justification": "Rejects a package name whose first segment character is punctuation."
      }|}
  with
  | Error _ -> ()
  | Ok _ -> Alcotest.fail "schema-incompatible package name was accepted"

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

let test_policy_hint_boundaries () =
  let detections, banned =
    unwrap
      (analyze_source ~filename:"native.mli" Interface
         "external hash : bytes -> bytes = \"native_hash\"\n")
  in
  let missing_both =
    evaluate ~dir:"." ~manifest:(unwrap (parse_manifest pure_manifest))
      ~detections ~banned
  in
  Alcotest.(check bool) "missing both" false (passed missing_both);
  let exception_only =
    unwrap
      (parse_manifest
         {|{
           "version": 1,
           "package": "ocaml/example",
           "capabilities": [],
           "justification": "Declares only the reviewed exception for this boundary.",
           "banned_construct_exceptions": [{"construct": "external", "language": "ocaml", "justification": "The reviewed native boundary needs this exact exception."}]
         }|})
  in
  let missing_capability =
    evaluate ~dir:"." ~manifest:exception_only ~detections ~banned
  in
  Alcotest.(check bool) "missing capability" false (passed missing_capability);
  let duplicate_detections =
    evaluate ~dir:"." ~manifest:(unwrap (parse_manifest pure_manifest))
      ~detections:(detections @ detections) ~banned:[]
  in
  Alcotest.(check int) "duplicate violation collapsed" 1
    (List.length duplicate_detections.violations);
  let optional_boundary : banned_construct =
    {
      file = "optional.ml";
      line = 1;
      column = 0;
      construct = "optional-boundary";
      evidence = "synthetic optional boundary";
      required_capability = None;
      exemptible = true;
    }
  in
  let optional_result =
    evaluate ~dir:"." ~manifest:(unwrap (parse_manifest pure_manifest))
      ~detections:[] ~banned:[ optional_boundary ]
  in
  match optional_result.violations with
  | [ violation ] ->
      Alcotest.(check bool)
        "optional boundary hint" true
        (contains_substring violation.message "remove the construct")
  | _ -> Alcotest.fail "optional boundary did not produce one violation"

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

let test_unsupported_ast_fails_closed () =
  let rejected =
    [
      "let value = [%generated]";
      "let[@generated] value = 1";
      "let read = input_line";
    ]
  in
  List.iter
    (fun source ->
      match analyze_source ~filename:"unsupported.ml" Implementation source with
      | Error _ -> ()
      | Ok _ -> Alcotest.failf "unsupported AST was accepted: %s" source)
    rejected

let test_extended_source_resolution () =
  let check name source expected_capabilities expected_banned =
    let detections, banned =
      unwrap (analyze_source ~filename:(name ^ ".ml") Implementation source)
    in
    Alcotest.(check (list string))
      (name ^ " capabilities")
      (sorted expected_capabilities)
      (detected_strings detections);
    Alcotest.(check (list string))
      (name ^ " banned")
      (sorted expected_banned) (banned_strings banned)
  in
  check "unix prefixes"
    "let () = Unix.exec_reviewed ()\nlet () = Unix.send_reviewed ()\nlet () = \
     Unix.recv_reviewed ()\n"
    [ "net:connect:*"; "proc:exec:*" ] [];
  check "module alias and open"
    "module U = Unix\nopen U\nlet () = send_reviewed ()\nlet () = \
     recv_reviewed ()\n"
    [ "net:connect:*" ] [];
  check "constrained alias"
    "module U : module type of Unix = Unix\nlet () = U.create_process_reviewed \
     ()\n"
    [ "proc:exec:*" ] [];
  check "standard channels"
    "let _ = input_char stdin\nlet _ = In_channel.input_line In_channel.stdin\n\
     let () = output_string stdout \"ok\"\nlet () = Out_channel.flush \
     Out_channel.stderr\n"
    [ "stdin:read:*"; "stdout:write:*" ] [];
  check "shadowed values"
    "let input_line _ = \"local\"\nlet stdin = ()\nlet _ = input_line stdin\n\
     let _ = Stdlib.input_char stdin\n"
    [] [];
  check "expression open"
    "let run () = let open Unix in send_reviewed ()\n"
    [ "net:connect:*" ] [];
  check "marshal closures"
    "open Marshal\nlet flag = Closures\nlet flag2 = Marshal.Closures\n"
    [] [ "Marshal.Closures" ];
  check "first-class sensitive references"
    "let magic = Obj.magic\nlet decode = Marshal.from_reviewed\n"
    [] [ "Obj.magic" ];
  check "top-level evaluation and non-identifier callee"
    "Sys.getenv \"TOP_LEVEL\"\nlet _ = ((fun f -> f) Sys.getenv) \"HOME\"\n"
    [ "env:read:*" ] []

let test_extended_ast_walks () =
  let accepted =
    [
      "let rec loop x = if x = 0 then 0 else loop (x - 1)\n\
       and other (x as alias) = alias\n";
      "let choose ?(fallback = 0) (type a) value =\n\
       \  match value with\n\
       \  | Some (x as alias) when alias > 0 -> x\n\
       \  | _ -> fallback\n";
      "let handle value =\n\
       \  try (match value with Some x -> x | None -> raise Exit)\n\
       \  with Exit -> 0\n";
      "let sum limit =\n\
       \  let total = ref 0 in\n\
       \  for index = 0 to limit do total := !total + index done;\n\
       \  while !total < limit do incr total done;\n\
       \  !total\n";
      "module F (X : sig end) = struct let value = 1 end\n\
       module M = F (struct end)\n";
      "module G () = struct let value = 1 end\nmodule H = G ()\n";
      "module rec A : sig val value : int end = struct let value = 1 end\n\
       and B : sig val value : int end = struct let value = A.value end\n";
      "module _ = struct let value = 1 end\ninclude struct let included = 1 end\n";
      "let local =\n\
       \  let module M = struct let value = 1 end in\n\
       \  M.value\n";
    ]
  in
  List.iteri
    (fun index source ->
      match
        analyze_source
          ~filename:(Printf.sprintf "extended-%d.ml" index)
          Implementation source
      with
      | Ok _ -> ()
      | Error message -> Alcotest.failf "extended AST rejected: %s" message)
    accepted;
  let rejected =
    [
      "let value = ([%generated]) ()\n";
      "module M = (val (module struct end))\n";
      "module M = [%generated]\n";
      "[%%generated]\n";
    ]
  in
  List.iteri
    (fun index source ->
      match
        analyze_source
          ~filename:(Printf.sprintf "rejected-%d.ml" index)
          Implementation source
      with
      | Error _ -> ()
      | Ok _ -> Alcotest.failf "unsupported extended AST %d was accepted" index)
    rejected;
  match analyze_source ~filename:"rejected.mli" Interface "[%%generated]\n" with
  | Error _ -> ()
  | Ok _ -> Alcotest.fail "signature extension was accepted"

let test_module_expression_fail_closed_matrix () =
  let rejected =
    [
      ( "functor",
        "module F (X : sig end) = struct include Obj end\n",
        "unresolved sensitive module alias" );
      ( "application",
        "module F (X : sig end) = struct end\nmodule M = F (Obj)\n",
        "unresolved sensitive module alias" );
      ( "unit application",
        "module G () = struct include Marshal end\nmodule N = G ()\n",
        "unresolved sensitive module alias" );
      ( "nested open",
        "module M = struct open Obj end\n",
        "unresolved sensitive module alias" );
      ( "nested module",
        "module M = struct module N = Obj end\n",
        "unresolved sensitive module alias" );
      ( "local extended open",
        "let f x = let open struct include Obj end in magic x\n",
        "unresolved sensitive local open" );
      ( "top extended open",
        "open struct include Obj end\n",
        "unresolved sensitive open" );
      ( "top extended include",
        "include struct include Obj end\n",
        "unresolved sensitive include" );
      ( "unpack",
        "module type S = sig end\nmodule M = (val packed : S)\n",
        "first-class module unpacking" );
      ( "module extension",
        "module M = [%generated]\n",
        "module extension is unsupported" );
      ( "structure extension",
        "[%%generated]\n",
        "structure extension is unsupported" );
    ]
  in
  List.iter
    (fun (name, source, expected) ->
      match analyze_source ~filename:(name ^ ".ml") Implementation source with
      | Error message when contains_substring message expected -> ()
      | Error message ->
          Alcotest.failf "%s: unexpected analyzer error: %s" name message
      | Ok _ -> Alcotest.failf "%s: unsafe module expression was accepted" name)
    rejected;
  (match analyze_source ~filename:"signature.mli" Interface "[%%generated]\n" with
  | Error message when contains_substring message "signature extension" -> ()
  | Error message -> Alcotest.failf "unexpected signature error: %s" message
  | Ok _ -> Alcotest.fail "signature extension was accepted");
  [ "open struct end\n"; "include struct end\n" ]
  |> List.iter (fun source ->
         match analyze_source ~filename:"safe-module.ml" Implementation source with
         | Ok _ -> ()
         | Error message ->
             Alcotest.failf "safe extended module was rejected: %s" message)

let test_sensitive_module_wrappers_fail_closed () =
  let rejected =
    [
      "module M = struct include Obj end\nlet cast value = M.magic value\n";
      "module M = struct include Marshal end\n\
       let decode value = M.from_bytes value 0\n";
      "module M = struct include Unix end\nlet run value = M.system value\n";
      "module rec M : sig val magic : 'a -> 'b end = struct include Obj end\n\
       let cast value = M.magic value\n";
    ]
  in
  List.iter
    (fun source ->
      match analyze_source ~filename:"wrapper.ml" Implementation source with
      | Error _ -> ()
      | Ok _ ->
          Alcotest.failf "sensitive module wrapper was accepted: %s" source)
    rejected

let test_sensitive_local_module_wrappers_fail_closed () =
  let rejected =
    [
      "let cast value = let module M = struct include Obj end in M.magic value\n";
      "let decode value = let module M = struct include Marshal end in \
       M.from_bytes value 0\n";
      "let run value = let module M = struct include Unix end in M.system value\n";
    ]
  in
  List.iter
    (fun source ->
      match
        analyze_source ~filename:"local_wrapper.ml" Implementation source
      with
      | Error _ -> ()
      | Ok _ ->
          Alcotest.failf "sensitive local module wrapper was accepted: %s"
            source)
    rejected

let test_directory_input_boundaries () =
  with_temp_directory (fun directory ->
      let oversized = Filename.concat directory "oversized.ml" in
      write_file oversized (String.make ((4 * 1024 * 1024) + 1) 'x');
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "oversized source was accepted");
      Sys.remove oversized;
      let generated = Filename.concat directory "lexer.mll" in
      write_file generated "rule token = parse eof { () }\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "generated source input was accepted");
      Sys.remove generated;
      write_file (Filename.concat directory "input.ml") "let x = 1\n";
      write_file
        (Filename.concat directory "dune")
        "(library (name generated) (instrumentation (backend bisect_ppx)))\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "out-of-scope bisect instrumentation was accepted");
      write_file
        (Filename.concat directory "dune")
        "(library (name generated) (preprocess (pps unsafe_ppx)))\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "Dune preprocessing was accepted");
      write_file
        (Filename.concat directory "dune")
        "(library (name generated) ( preprocess (pps unsafe_ppx)))\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ ->
          Alcotest.fail "whitespace-separated Dune preprocessing was accepted");
      write_file (Filename.concat directory "dune") "( include generated.inc)\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "Dune include was accepted");
      write_file
        (Filename.concat directory "dune")
        "(library (name generated) (flags (:standard -ppx unsafe_ppx)))\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "Dune -ppx flag was accepted");
      write_file
        (Filename.concat directory "dune")
        "(library (name generated) (flags (:standard -pp\tunsafe_pp)))\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "Dune tab-separated -pp flag was accepted");
      write_file
        (Filename.concat directory "dune")
        "(library (name generated) (flags (:standard \"-ppx\" \"unsafe_ppx\")))\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "quoted Dune -ppx flag was accepted");
      write_file
        (Filename.concat directory "dune")
        "(library (name generated) (flags (:standard \"\\x2dppx\" \
         \"unsafe_ppx\")))\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "escaped Dune -ppx flag was accepted");
      write_file
        (Filename.concat directory "dune")
        "(library (name generated) (instrumentation (backend unsafe_ppx)))\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "unsafe Dune instrumentation was accepted");
      write_file
        (Filename.concat directory "dune")
        "(library (name native) (foreign_stubs (language c) (names native)))\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "Dune foreign stubs were accepted");
      write_file
        (Filename.concat directory "dune")
        "(library (name native) (ctypes (external_library_name native)))\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "Dune ctypes generation was accepted");
      write_file
        (Filename.concat directory "dune")
        "(library (name native) (; hidden field\n\
        \ ctypes (external_library_name native)))\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "comment-hidden Dune ctypes was accepted");
      write_file
        (Filename.concat directory "dune")
        "(library (name native) (extra_objects native) (link_flags -lnative))\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "Dune native link inputs were accepted");
      write_file
        (Filename.concat directory "dune")
        "(dynamic_include generated.inc)\n";
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "Dune dynamic include was accepted");
      Sys.remove (Filename.concat directory "dune");
      write_file
        (Filename.concat directory "dune-workspace")
        "(lang dune 3.16)\n";
      match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "Dune workspace was accepted")

let test_manifest_identity_and_kind () =
  with_temp_directory (fun directory ->
      let manifest = Filename.concat directory "required_capabilities.json" in
      write_file manifest
        {|{
          "version": 1,
          "package": "ocaml/not-this-directory",
          "capabilities": [],
          "justification": "This deliberately mismatched profile must fail closed."
        }|};
      (match analyze_directory directory with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "mismatched manifest identity was accepted");
      Sys.remove manifest;
      let target = Filename.concat directory "manifest-target.json" in
      write_file target "{}";
      try
        Unix.symlink target manifest;
        match analyze_directory directory with
        | Error _ -> ()
        | Ok _ -> Alcotest.fail "symlinked manifest was accepted"
      with Unix.Unix_error _ -> ())

let test_directory_safe_inputs_and_exclusions () =
  with_temp_directory (fun root ->
      let directory = Filename.concat root "ca-capability-analyzer" in
      Unix.mkdir directory 0o700;
      let src = Filename.concat directory "src" in
      Unix.mkdir src 0o700;
      write_file (Filename.concat src "input.ml") "let value = 1\n";
      write_file (Filename.concat src "input.mli") "val value : int\n";
      write_file (Filename.concat src "README.txt") "ignored\n";
      write_file (Filename.concat src "dune")
        "(library\n ; a safe comment\n (name example)\n \
         (instrumentation (backend bisect_ppx))\n \
         (libraries \"a\\\"quoted\\\"name\"))\n";
      write_file (Filename.concat directory "dune-project")
        "(lang dune 3.16)\n(name example)\n";
      List.iter
        (fun name ->
          let excluded = Filename.concat directory name in
          Unix.mkdir excluded 0o700;
          write_file (Filename.concat excluded "ignored.mll")
            "generated content is ignored in excluded directories\n")
        [ "_build"; ".git"; "_opam"; "node_modules" ];
      let result = unwrap (analyze_directory directory) in
      Alcotest.(check bool) "safe package" true (passed result);
      Alcotest.(check int) "two source files" 0
        (List.length result.detected);
      let target = Filename.concat directory "target.ml" in
      let link = Filename.concat directory "source-link.ml" in
      write_file target "let linked = 1\n";
      (try
         Unix.symlink target link;
         match analyze_directory directory with
         | Error _ -> ()
         | Ok _ -> Alcotest.fail "symlinked source was accepted"
       with Unix.Unix_error _ -> ()))

let test_directory_depth_limit () =
  with_temp_directory (fun root ->
      let rec create depth directory =
        if depth = 66 then ()
        else
          let child = Filename.concat directory "d" in
          Unix.mkdir child 0o700;
          create (depth + 1) child
      in
      create 0 root;
      match analyze_directory root with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "over-deep source tree was accepted")

let test_directory_manifest_and_generated_boundaries () =
  with_temp_directory (fun root ->
      let manifest = Filename.concat root "required_capabilities.json" in
      write_file manifest (String.make ((1024 * 1024) + 1) 'x');
      (match analyze_directory root with
      | Error message when contains_substring message "input is" -> ()
      | Error message -> Alcotest.failf "unexpected manifest limit error: %s" message
      | Ok _ -> Alcotest.fail "oversized manifest was accepted");
      Sys.remove manifest;
      let generated = Filename.concat root "parser.mly" in
      write_file generated "generated parser source\n";
      match analyze_directory root with
      | Error message when contains_substring message "generated OCaml source" -> ()
      | Error message -> Alcotest.failf "unexpected generated-source error: %s" message
      | Ok _ -> Alcotest.fail "generated parser input was accepted")

let test_directory_analysis () =
  with_temp_directory (fun root ->
      let make_package name source manifest =
        let directory = Filename.concat root name in
        Unix.mkdir directory 0o700;
        write_file (Filename.concat directory "input.ml") source;
        Option.iter
          (write_file (Filename.concat directory "required_capabilities.json"))
          manifest;
        directory
      in
      let clean_name = "clean" in
      let clean =
        make_package clean_name "let value = Sys.getenv \"FIXTURE_VALUE\"\n"
          (Some
             (Printf.sprintf
                {|{"version":1,"package":"ocaml/%s","capabilities":[{"category":"env","action":"read","target":"FIXTURE_VALUE","justification":"Reads one deterministic fixture environment variable."}],"justification":"Fixture package for a declared environment read."}|}
                clean_name))
      in
      let denied =
        make_package "denied" "let value = Sys.getenv \"UNDECLARED\"\n" None
      in
      let broken = make_package "broken" "let =\n" None in
      let clean_result = unwrap (analyze_directory clean) in
      Alcotest.(check bool) "clean package" true (passed clean_result);
      let denied_result = unwrap (analyze_directory denied) in
      Alcotest.(check bool) "violation package" false (passed denied_result);
      Alcotest.(check (list string))
        "violation code" [ "CAP001" ]
        (List.map (fun violation -> violation.code) denied_result.violations);
      Alcotest.(check (list string))
        "deterministic relative paths" [ "input.ml" ]
        (denied_result.detected
        |> List.map (fun (detection : detection) -> detection.file)
        |> sorted);
      match analyze_directory broken with
      | Error _ -> ()
      | Ok _ -> Alcotest.fail "directory parse failure was ignored")

let test_cli_results () =
  let capture dir verbose =
    let output = Buffer.create 128 and errors = Buffer.create 128 in
    let code =
      run ~dir ~verbose ~stdout:(Buffer.add_string output)
        ~stderr:(Buffer.add_string errors)
    in
    (code, Buffer.contents output, Buffer.contents errors)
  in
  with_temp_directory (fun root ->
      let clean = Filename.concat root "clean" in
      let denied = Filename.concat root "denied" in
      Unix.mkdir clean 0o700;
      Unix.mkdir denied 0o700;
      write_file (Filename.concat clean "clean.ml") "let x = 1\n";
      write_file
        (Filename.concat denied "denied.ml")
        "let value = Sys.getenv \"UNDECLARED\"\n";
      let code, output, errors = capture clean true in
      Alcotest.(check int) "clean exit" 0 code;
      Alcotest.(check bool) "verbose output" true (String.length output > 0);
      Alcotest.(check string) "clean stderr" "" errors;
      let code, output, _ = capture denied false in
      Alcotest.(check int) "violation exit" 1 code;
      Alcotest.(check bool) "violation output" true (String.length output > 0);
      let code, output, _ = capture denied true in
      Alcotest.(check int) "verbose violation exit" 1 code;
      Alcotest.(check bool)
        "verbose detection" true
        (String.ends_with ~suffix:"\n" output
        && String.contains output ':');
      let code, _, errors = capture (Filename.concat root "missing") false in
      Alcotest.(check int) "error exit" 2 code;
      Alcotest.(check bool) "error output" true (String.length errors > 0))

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
          Alcotest.test_case "unsupported AST fails closed" `Quick
            test_unsupported_ast_fails_closed;
          Alcotest.test_case "extended resolution" `Quick
            test_extended_source_resolution;
          Alcotest.test_case "extended AST walks" `Quick
            test_extended_ast_walks;
          Alcotest.test_case "module expressions fail closed" `Quick
            test_module_expression_fail_closed_matrix;
          Alcotest.test_case "sensitive module wrappers fail closed" `Quick
            test_sensitive_module_wrappers_fail_closed;
          Alcotest.test_case "sensitive local module wrappers fail closed"
            `Quick test_sensitive_local_module_wrappers_fail_closed;
          Alcotest.test_case "stable formatting" `Quick test_format_and_sorting;
        ] );
      ( "manifests",
        [
          Alcotest.test_case "zero profile" `Quick test_manifest_zero_profile;
          Alcotest.test_case "closed taxonomy" `Quick
            test_manifest_closed_taxonomy;
          Alcotest.test_case "shape errors" `Quick test_manifest_shape_errors;
          Alcotest.test_case "schema boundaries" `Quick
            test_manifest_schema_boundaries;
          Alcotest.test_case "duplicate capability" `Quick
            test_manifest_duplicate_capability;
          Alcotest.test_case "package name boundary" `Quick
            test_manifest_package_name_boundary;
        ] );
      ( "policy",
        [
          Alcotest.test_case "declared capabilities" `Quick
            test_evaluate_undeclared_and_declared;
          Alcotest.test_case "FFI dual opt-in" `Quick test_ffi_dual_opt_in;
          Alcotest.test_case "policy hint boundaries" `Quick
            test_policy_hint_boundaries;
          Alcotest.test_case "Dynlink dual opt-in" `Quick
            test_dynlink_dual_opt_in;
          Alcotest.test_case "hard bans" `Quick test_hard_ban_cannot_be_exempted;
        ] );
      ( "integration",
        [
          Alcotest.test_case "directory" `Quick test_directory_analysis;
          Alcotest.test_case "input boundaries" `Quick
            test_directory_input_boundaries;
          Alcotest.test_case "manifest identity" `Quick
            test_manifest_identity_and_kind;
          Alcotest.test_case "safe inputs and exclusions" `Quick
            test_directory_safe_inputs_and_exclusions;
          Alcotest.test_case "directory depth limit" `Quick
            test_directory_depth_limit;
          Alcotest.test_case "manifest and generated boundaries" `Quick
            test_directory_manifest_and_generated_boundaries;
          Alcotest.test_case "CLI" `Quick test_cli_results;
        ] );
    ]
