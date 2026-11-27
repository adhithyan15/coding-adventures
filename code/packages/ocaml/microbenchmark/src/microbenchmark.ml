open Rank_statistics
module String_set = Set.Make (String)

type config = {
  pairs : int;
  confidence_percent : int;
  minimum_factor : float;
  seed : int64;
  target_ns : int64;
  max_batch : int64;
}

type order = Baseline_first | Candidate_first
type pair = { baseline_ns : int64; candidate_ns : int64; order : order }
type interval = { low : float; high : float }
type analysis = { ratio : float; interval : interval option; verdict : string }

type metadata = {
  workload : string;
  invocation : string list;
  revision : string;
  runtime : string;
  os : string;
  arch : string;
  profile : string option;
}

type report = {
  config : config;
  batch_size : int64;
  pairs : pair list;
  analysis : analysis;
  metadata : metadata option;
}

type batch = int64 -> (int64, string) result
type clock = unit -> int64

let default_config =
  {
    pairs = 40;
    confidence_percent = 95;
    minimum_factor = 1.05;
    seed = 0L;
    target_ns = 100_000L;
    max_batch = 65_536L;
  }

let ( let* ) = Result.bind
let ( >>= ) = Result.bind

let validate_config (config : config) =
  if config.pairs < 1 || config.pairs > 10_000 then
    Error "pairs must be from 1 to 10000"
  else if config.confidence_percent < 1 || config.confidence_percent > 99 then
    Error "confidence must be from 1 to 99"
  else if
    (not (Float.is_finite config.minimum_factor))
    || config.minimum_factor <= 1.0
  then Error "factor must be finite and greater than 1"
  else if config.target_ns <= 0L || config.max_batch <= 0L then
    Error "target and max batch must be positive"
  else Ok ()

let analyze pairs confidence factor =
  if pairs = [] then Error "at least one pair is required"
  else if List.length pairs > 10_000 then
    Error "at most 10000 pairs are supported"
  else if confidence < 1 || confidence > 99 then
    Error "confidence must be from 1 to 99"
  else if (not (Float.is_finite factor)) || factor <= 1.0 then
    Error "factor must be finite and greater than 1"
  else
    let rec ratios acc = function
      | [] -> Ok (Array.of_list (List.rev acc))
      | pair :: _rest when pair.baseline_ns <= 0L || pair.candidate_ns <= 0L ->
          Error "pair durations must be positive"
      | pair :: rest ->
          let ratio =
            Int64.to_float pair.candidate_ns /. Int64.to_float pair.baseline_ns
          in
          if not (Float.is_finite ratio) then Error "pair ratio is not finite"
          else ratios (ratio :: acc) rest
    in
    let* samples = ratios [] pairs in
    let* center = median samples in
    let* ranks = median_interval samples confidence in
    let interval =
      Option.map
        (fun (value : Rank_statistics.interval) ->
          { low = value.low; high = value.high })
        ranks
    in
    let verdict =
      match interval with
      | None -> "inconclusive"
      | Some ci when ci.high < 1.0 /. factor -> "faster"
      | Some ci when ci.low > factor -> "slower"
      | Some ci when ci.low >= 1.0 /. factor && ci.high <= factor ->
          "equivalent"
      | Some _ -> "inconclusive"
    in
    Ok { ratio = center; interval; verdict }

let checked_multiply_nonnegative value multiplier =
  if multiplier < 0L then Error "operation count must be nonnegative"
  else if value = 0L || multiplier = 0L then Ok 0L
  else
    let product = Int64.mul value multiplier in
    if Int64.div product multiplier <> value then
      Error "signed 64-bit checksum overflow"
    else Ok product

let collect (config : config) expected_checksum baseline candidate clock =
  let* () = validate_config config in
  let expected operations =
    checked_multiply_nonnegative expected_checksum operations
  in
  let verify function_ operations =
    let* want = expected operations in
    let* got = function_ operations in
    if got = want then Ok () else Error "checksum mismatch"
  in
  let* () = verify baseline 1L in
  let* () = verify candidate 1L in
  let previous = ref None in
  let read_clock () =
    try
      let current = clock () in
      match !previous with
      | Some last when current < last -> Error "clock regressed"
      | _ when current < 0L -> Error "clock returned a negative tick"
      | _ ->
          previous := Some current;
          Ok current
    with exn -> Error ("clock failed: " ^ Printexc.to_string exn)
  in
  let measure function_ operations calibration =
    let* start = read_clock () in
    let measured = function_ operations in
    let ending = read_clock () in
    let* actual = measured in
    let* finish = ending in
    let* want = expected operations in
    if actual <> want then Error "checksum mismatch"
    else if finish <= start && not calibration then
      Error "measured duration must be positive"
    else Ok (Int64.sub finish start)
  in
  let rec calibrate size =
    let* baseline_ns = measure baseline size true in
    let* candidate_ns = measure candidate size true in
    if baseline_ns >= config.target_ns && candidate_ns >= config.target_ns then
      Ok size
    else if size >= config.max_batch then Error "calibration limit exhausted"
    else
      calibrate
        (if size > Int64.div config.max_batch 2L then config.max_batch
         else Int64.mul size 2L)
  in
  let* batch_size = calibrate 1L in
  let schedule = Array.init config.pairs (fun index -> index mod 2 = 0) in
  let random_state = ref config.seed in
  for index = Array.length schedule - 1 downto 1 do
    random_state :=
      Int64.add
        (Int64.mul !random_state 6_364_136_223_846_793_005L)
        1_442_695_040_888_963_407L;
    let divisor = Int64.of_int (index + 1) in
    let chosen = Int64.to_int (Int64.unsigned_rem !random_state divisor) in
    let value = schedule.(index) in
    schedule.(index) <- schedule.(chosen);
    schedule.(chosen) <- value
  done;
  let measure_pair baseline_first =
    let first = if baseline_first then baseline else candidate in
    let second = if baseline_first then candidate else baseline in
    let* first_ns = measure first batch_size false in
    let* second_ns = measure second batch_size false in
    let base_ns, candidate_ns =
      if baseline_first then (first_ns, second_ns) else (second_ns, first_ns)
    in
    Ok
      {
        baseline_ns = base_ns;
        candidate_ns;
        order = (if baseline_first then Baseline_first else Candidate_first);
      }
  in
  let rec gather index observations =
    if index = Array.length schedule then Ok (List.rev observations)
    else
      let* pair = measure_pair schedule.(index) in
      gather (index + 1) (pair :: observations)
  in
  let* pairs = gather 0 [] in
  let* analysis =
    analyze pairs config.confidence_percent config.minimum_factor
  in
  Ok { config; batch_size; pairs; analysis; metadata = None }

let default_metadata invocation revision runtime os arch =
  {
    workload = "indexed-array-sum-4096";
    invocation;
    revision;
    runtime;
    os;
    arch;
    profile = None;
  }

let json_i64 value = `Intlit (Int64.to_string value)

let json_order = function
  | Baseline_first -> `String "baseline-first"
  | Candidate_first -> `String "candidate-first"

let config_json (config : config) =
  `Assoc
    [
      ("pairs", `Int config.pairs);
      ("confidence_percent", `Int config.confidence_percent);
      ("minimum_factor", `Float config.minimum_factor);
      ("seed", json_i64 config.seed);
      ("target_ns", json_i64 config.target_ns);
      ("max_batch", json_i64 config.max_batch);
    ]

let pair_json (pair : pair) =
  `Assoc
    [
      ("baseline_ns", json_i64 pair.baseline_ns);
      ("candidate_ns", json_i64 pair.candidate_ns);
      ("order", json_order pair.order);
    ]

let analysis_json (analysis : analysis) =
  let interval =
    match analysis.interval with
    | None -> `Null
    | Some ci -> `List [ `Float ci.low; `Float ci.high ]
  in
  `Assoc
    [
      ("ratio", `Float analysis.ratio);
      ("interval", interval);
      ("verdict", `String analysis.verdict);
    ]

let report_json (report : report) =
  let core =
    [
      ("config", config_json report.config);
      ("batch_size", json_i64 report.batch_size);
      ("pairs", `List (List.map pair_json report.pairs));
      ("analysis", analysis_json report.analysis);
    ]
  in
  let metadata =
    match report.metadata with
    | None -> []
    | Some value -> (
        [
          ("workload", `String value.workload);
          ( "invocation",
            `List (List.map (fun item -> `String item) value.invocation) );
          ("revision", `String value.revision);
          ("runtime", `String value.runtime);
          ("os", `String value.os);
          ("arch", `String value.arch);
        ]
        @
        match value.profile with
        | None -> []
        | Some profile -> [ ("profile", `String profile) ])
  in
  `Assoc (core @ metadata)

let encode_report report = Yojson.Safe.to_string (report_json report)
let json_size_limit = 4 * 1024 * 1024

let reject_too_deep source =
  let depth = ref 0
  and in_string = ref false
  and escaped = ref false
  and failure = ref None in
  String.iter
    (fun ch ->
      if !in_string then
        if !escaped then escaped := false
        else if ch = '\\' then escaped := true
        else if ch = '"' then in_string := false
        else if ch = '"' then in_string := true
        else if ch = '{' || ch = '[' then (
          incr depth;
          if !depth > 128 then failure := Some "report exceeds nesting limit")
        else if ch = '}' || ch = ']' then decr depth)
    source;
  match !failure with Some error -> Error error | None -> Ok ()

let rec validate_tree depth count json =
  if depth > 128 then Error "report exceeds nesting limit"
  else if count > 65_536 then Error "report exceeds JSON value limit"
  else
    let children =
      match json with
      | `Assoc fields -> List.map snd fields
      | `List values -> values
      | _ -> []
    in
    let rec walk current = function
      | [] -> Ok current
      | child :: rest ->
          let* updated = validate_tree (depth + 1) (current + 1) child in
          walk updated rest
    in
    walk count children

let rec reject_duplicates = function
  | `Assoc fields ->
      let rec keys seen = function
        | [] -> Ok ()
        | (key, value) :: rest ->
            if String_set.mem key seen then Error ("duplicate JSON key: " ^ key)
            else
              let* () = reject_duplicates value in
              keys (String_set.add key seen) rest
      in
      keys String_set.empty fields
  | `List values ->
      List.fold_left
        (fun result value ->
          let* () = result in
          reject_duplicates value)
        (Ok ()) values
  | _ -> Ok ()

let object_fields label = function
  | `Assoc fields -> Ok fields
  | _ -> Error (label ^ " must be an object")

let reject_unknown label allowed fields =
  match List.find_opt (fun (key, _) -> not (List.mem key allowed)) fields with
  | None -> Ok ()
  | Some (key, _) -> Error ("unknown " ^ label ^ " field: " ^ key)

let required name fields =
  match List.assoc_opt name fields with
  | None -> Error ("missing field: " ^ name)
  | Some value -> Ok value

let text label = function
  | `String value -> Ok value
  | _ -> Error (label ^ " must be a string")

let workload_text label value =
  let* result = text label value in
  let ascii_space = function
    | ' ' | '\t' | '\n' | '\011' | '\012' | '\r' -> true
    | _ -> false
  in
  if
    result = ""
    || String.contains result '\000'
    || String.for_all ascii_space result
  then Error (label ^ " must be nonblank and contain no NUL")
  else Ok result

let text_without_nul label value =
  let* result = text label value in
  if String.contains result '\000' then Error (label ^ " must not contain NUL")
  else Ok result

let nonempty_text label value =
  let* result = text_without_nul label value in
  if result = "" then Error (label ^ " must not be empty") else Ok result

let array label = function
  | `List values -> Ok values
  | _ -> Error (label ^ " must be an array")

let int64 label = function
  | `Int value -> Ok (Int64.of_int value)
  | `Intlit value -> (
      try Ok (Int64.of_string value)
      with Failure _ ->
        Error (label ^ " must be a signed 64-bit integer token"))
  | _ -> Error (label ^ " must be an integer JSON token")

let positive_or_nonnegative_int label allow_zero value =
  let* exact = int64 label value in
  if (exact < if allow_zero then 0L else 1L) || exact > Int64.of_int max_int
  then Error (label ^ " is out of range")
  else Ok (Int64.to_int exact)

let finite_float label = function
  | `Float value when Float.is_finite value -> Ok value
  | `Int value -> Ok (float_of_int value)
  | `Intlit value -> (
      try
        let parsed = float_of_string value in
        if Float.is_finite parsed then Ok parsed
        else Error (label ^ " must be finite")
      with Failure _ -> Error (label ^ " must be a number"))
  | _ -> Error (label ^ " must be a finite number")

let decode_config json =
  let* fields = object_fields "config" json in
  let* () =
    reject_unknown "config"
      [
        "pairs";
        "confidence_percent";
        "minimum_factor";
        "seed";
        "target_ns";
        "max_batch";
      ]
      fields
  in
  let* pairs =
    required "pairs" fields >>= positive_or_nonnegative_int "pairs" true
  in
  let* confidence_percent =
    required "confidence_percent" fields
    >>= positive_or_nonnegative_int "confidence_percent" true
  in
  let* minimum_factor =
    required "minimum_factor" fields >>= finite_float "minimum_factor"
  in
  let* seed = required "seed" fields >>= int64 "seed" in
  let* target_ns = required "target_ns" fields >>= int64 "target_ns" in
  let* max_batch = required "max_batch" fields >>= int64 "max_batch" in
  let config =
    { pairs; confidence_percent; minimum_factor; seed; target_ns; max_batch }
  in
  let* () = validate_config config in
  Ok config

let decode_pair json =
  let* fields = object_fields "pair" json in
  let* () =
    reject_unknown "pair" [ "baseline_ns"; "candidate_ns"; "order" ] fields
  in
  let* baseline_ns = required "baseline_ns" fields >>= int64 "baseline_ns" in
  let* candidate_ns = required "candidate_ns" fields >>= int64 "candidate_ns" in
  let* raw_order = required "order" fields >>= text "order" in
  let* order =
    match raw_order with
    | "baseline-first" -> Ok Baseline_first
    | "candidate-first" -> Ok Candidate_first
    | _ -> Error "invalid pair order"
  in
  if baseline_ns <= 0L || candidate_ns <= 0L then
    Error "pair durations must be positive"
  else Ok { baseline_ns; candidate_ns; order }

let decode_analysis json =
  let* fields = object_fields "analysis" json in
  let* () =
    reject_unknown "analysis" [ "ratio"; "interval"; "verdict" ] fields
  in
  let* ratio = required "ratio" fields >>= finite_float "ratio" in
  let* raw_interval = required "interval" fields in
  let* interval =
    match raw_interval with
    | `Null -> Ok None
    | `List [ low; high ] ->
        let* low = finite_float "interval low" low in
        let* high = finite_float "interval high" high in
        if low > high then Error "interval endpoints are reversed"
        else Ok (Some { low; high })
    | _ -> Error "interval must be null or contain two values"
  in
  let* verdict = required "verdict" fields >>= text "verdict" in
  if List.mem verdict [ "faster"; "slower"; "equivalent"; "inconclusive" ] then
    Ok { ratio; interval; verdict }
  else Error "invalid verdict"

let decode_metadata fields =
  let names =
    [ "workload"; "invocation"; "revision"; "runtime"; "os"; "arch" ]
  in
  let present = List.filter (fun name -> List.mem_assoc name fields) names in
  let profile_present = List.mem_assoc "profile" fields in
  if present = [] then
    if profile_present then Error "profile requires standard metadata"
    else Ok None
  else if List.length present <> List.length names then
    Error "standard metadata must be complete"
  else
    let* workload = required "workload" fields >>= workload_text "workload" in
    let* invocation_json =
      required "invocation" fields >>= array "invocation"
    in
    let* invocation =
      List.fold_left
        (fun result item ->
          let* sofar = result in
          let* item = text_without_nul "invocation entry" item in
          Ok (item :: sofar))
        (Ok []) invocation_json
    in
    let invocation = List.rev invocation in
    let* revision = required "revision" fields >>= nonempty_text "revision" in
    let* runtime = required "runtime" fields >>= nonempty_text "runtime" in
    let* os = required "os" fields >>= nonempty_text "os" in
    let* arch = required "arch" fields >>= nonempty_text "arch" in
    let* profile =
      if profile_present then
        required "profile" fields >>= nonempty_text "profile"
        |> Result.map Option.some
      else Ok None
    in
    Ok (Some { workload; invocation; revision; runtime; os; arch; profile })

let decode_report json =
  let* fields = object_fields "report" json in
  let* () =
    reject_unknown "report"
      [
        "config";
        "batch_size";
        "pairs";
        "analysis";
        "workload";
        "invocation";
        "revision";
        "runtime";
        "os";
        "arch";
        "profile";
      ]
      fields
  in
  let* config = required "config" fields >>= decode_config in
  let* batch_size = required "batch_size" fields >>= int64 "batch_size" in
  let* pair_json = required "pairs" fields >>= array "pairs" in
  if List.length pair_json <> config.pairs then
    Error "pair count does not match config"
  else
    let* pairs =
      List.fold_left
        (fun result item ->
          let* sofar = result in
          let* parsed = decode_pair item in
          Ok (parsed :: sofar))
        (Ok []) pair_json
    in
    let pairs = List.rev pairs in
    if batch_size <= 0L || batch_size > config.max_batch then
      Error "batch_size outside configured range"
    else
      let* analysis = required "analysis" fields >>= decode_analysis in
      let* metadata = decode_metadata fields in
      let baseline_first =
        List.length
          (List.filter (fun item -> item.order = Baseline_first) pairs)
      in
      if baseline_first <> (config.pairs + 1) / 2 then
        Error "pair order is not balanced"
      else
        let* computed =
          analyze pairs config.confidence_percent config.minimum_factor
        in
        if computed <> analysis then
          Error "reported analysis disagrees with raw pairs"
        else Ok { config; batch_size; pairs; analysis; metadata }

let valid_surrogate_escapes source =
  let length = String.length source in
  let hex_value = function
    | '0' .. '9' as digit -> Some (Char.code digit - Char.code '0')
    | 'a' .. 'f' as digit -> Some (Char.code digit - Char.code 'a' + 10)
    | 'A' .. 'F' as digit -> Some (Char.code digit - Char.code 'A' + 10)
    | _ -> None
  in
  let code_unit_at index =
    if index + 6 > length || source.[index] <> '\\' || source.[index + 1] <> 'u'
    then None
    else
      let value = ref 0 in
      let rec read offset =
        if offset = 6 then Some !value
        else
          match hex_value source.[index + offset] with
          | None -> None
          | Some digit ->
              value := (!value lsl 4) lor digit;
              read (offset + 1)
      in
      read 2
  in
  let rec scan index in_string =
    if index = length then not in_string
    else if not in_string then
      if source.[index] = '"' then scan (index + 1) true
      else scan (index + 1) false
    else
      match source.[index] with
      | '"' -> scan (index + 1) false
      | '\\' when index + 1 < length -> (
          if source.[index + 1] <> 'u' then scan (index + 2) true
          else
            match code_unit_at index with
            | None -> false
            | Some code when code >= 0xd800 && code <= 0xdbff -> (
                match code_unit_at (index + 6) with
                | Some low when low >= 0xdc00 && low <= 0xdfff ->
                    scan (index + 12) true
                | _ -> false)
            | Some code when code >= 0xdc00 && code <= 0xdfff -> false
            | Some _ -> scan (index + 6) true)
      | '\\' -> false
      | _ -> scan (index + 1) true
  in
  scan 0 false

let replay_report source =
  if String.length source > json_size_limit then
    Error "report exceeds 4 MiB JSON limit"
  else if not (valid_surrogate_escapes source) then
    Error "invalid Unicode surrogate escape"
  else
    let* () = reject_too_deep source in
    try
      let json = Yojson.Safe.from_string source in
      let* () = validate_tree 0 1 json |> Result.map (fun _ -> ()) in
      let* () = reject_duplicates json in
      decode_report json
    with Yojson.Json_error message -> Error ("invalid JSON: " ^ message)
