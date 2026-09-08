open Coding_adventures_capability_analyzer

let exit_with_code ~code = Stdlib.exit code

let () =
  let dir = ref "." and verbose = ref false in
  let options =
    [
      ("--dir", Arg.Set_string dir, "Package directory to analyze");
      ("--verbose", Arg.Set verbose, "Print every detected capability");
    ]
  in
  let usage =
    "coding-adventures-capability-analyzer [--dir PATH] [--verbose]"
  in
  Arg.parse options
    (fun argument -> raise (Arg.Bad ("unexpected argument: " ^ argument)))
    usage;
  let exit_code =
    run ~dir:!dir ~verbose:!verbose ~stdout:(output_string stdout)
      ~stderr:(output_string stderr)
  in
  exit_with_code ~code:exit_code
