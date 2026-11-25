open Microbenchmark

let ( let* ) = Result.bind

let usage () =
  print_endline
    "Usage: array_microbenchmark [--pairs N] [--confidence N] [--factor F] \
     [--seed I64] [--target-ns I64] [--max-batch I64] [--revision TEXT] | \
     --replay FILE";
  flush stdout

let nonblank value = String.trim value <> ""

let choose_revision = function
  | Some value when nonblank value -> value
  | _ -> (
      match Sys.getenv_opt "GIT_REVISION" with
      | Some value when nonblank value -> value
      | _ -> (
          match Sys.getenv_opt "GITHUB_SHA" with
          | Some value when nonblank value -> value
          | _ -> "unknown"))

let parse_i64 name value =
  try Ok (Int64.of_string value)
  with Failure _ -> Error ("invalid signed 64-bit integer for " ^ name)

let parse_int name value =
  try Ok (int_of_string value)
  with Failure _ -> Error ("invalid integer for " ^ name)

let parse_float name value =
  try Ok (float_of_string value)
  with Failure _ -> Error ("invalid number for " ^ name)

let rec parse_options (config : config) revision = function
  | [] -> Ok (config, revision)
  | "--revision" :: value :: _ when not (nonblank value) ->
      Error "--revision must not be blank"
  | "--revision" :: [] -> Error "missing value for --revision"
  | "--revision" :: value :: rest -> parse_options config (Some value) rest
  | flag :: value :: rest ->
      let update field = parse_int flag value |> Result.map field in
      let update_i64 field = parse_i64 flag value |> Result.map field in
      let updated =
        match flag with
        | "--pairs" -> update (fun pairs -> { config with pairs })
        | "--confidence" ->
            update (fun confidence_percent ->
                { config with confidence_percent })
        | "--factor" ->
            parse_float flag value
            |> Result.map (fun minimum_factor -> { config with minimum_factor })
        | "--seed" -> update_i64 (fun seed -> { config with seed })
        | "--target-ns" ->
            update_i64 (fun target_ns -> { config with target_ns })
        | "--max-batch" ->
            update_i64 (fun max_batch -> { config with max_batch })
        | _ -> Error ("unknown option: " ^ flag)
      in
      let* config = updated in
      parse_options config revision rest
  | [ flag ] -> Error ("missing value for " ^ flag)

let replay path =
  try
    let channel = open_in_bin path in
    let length = in_channel_length channel in
    if length > 4 * 1024 * 1024 then (
      close_in channel;
      Error "report exceeds 4 MiB JSON limit")
    else
      let content = really_input_string channel length in
      close_in channel;
      let* report = replay_report content in
      Printf.printf "replay valid: %d pairs, ratio=%g\n"
        (List.length report.pairs) report.analysis.ratio;
      flush stdout;
      Ok ()
  with Sys_error message -> Error message

let run argv =
  match argv with
  | [ "--help" ] ->
      usage ();
      Ok ()
  | [ "--replay"; path ] -> replay path
  | _ ->
      let* config, explicit_revision = parse_options default_config None argv in
      let values = Array_workload.make_array () in
      let* report =
        collect config Array_workload.expected_checksum
          (Array_workload.indexed_batch values)
          (Array_workload.iteration_batch values)
          Native_clock.now_ns
      in
      let metadata =
        default_metadata (Array.to_list Sys.argv)
          (choose_revision explicit_revision)
          ("OCaml " ^ Sys.ocaml_version)
          (Native_clock.os_name ())
          (Native_clock.architecture ())
      in
      let output = encode_report { report with metadata = Some metadata } in
      print_endline output;
      flush stdout;
      Ok ()

let () =
  match run (Array.to_list Sys.argv |> List.tl) with
  | Ok () -> ()
  | Error message ->
      prerr_endline ("error: " ^ message);
      exit 1
