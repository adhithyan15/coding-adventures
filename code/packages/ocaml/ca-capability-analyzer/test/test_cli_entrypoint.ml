let run ocamlrun executable arguments =
  ocamlrun :: executable :: arguments |> String.concat " " |> Sys.command

let with_temp_directory callback =
  let directory = Filename.temp_file "ocaml-capability-cli-" "-test" in
  Sys.remove directory;
  Sys.mkdir directory 0o700;
  Fun.protect
    ~finally:(fun () -> Sys.rmdir directory)
    (fun () -> callback directory)

let test_cli ocamlrun executable () =
  Alcotest.(check int) "help" 0 (run ocamlrun executable [ "--help" ]);
  Alcotest.(check int)
    "bad argument" 2
    (run ocamlrun executable [ "unexpected" ]);
  with_temp_directory (fun directory ->
      Alcotest.(check int)
        "clean package" 0
        (run ocamlrun executable [ "--dir"; directory; "--verbose" ]))

let () =
  let ocamlrun = Sys.getenv "CAPABILITY_TEST_OCAMLRUN"
  and executable = Sys.getenv "CAPABILITY_TEST_CLI" in
  Alcotest.run "OCaml capability analyzer CLI"
    [
      ( "entrypoint",
        [
          Alcotest.test_case "arguments and exit" `Quick
            (test_cli ocamlrun executable);
        ] );
    ]
