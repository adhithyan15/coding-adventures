let () =
  Alcotest.run "coding-adventures-my-pkg"
    [
      ( "metadata",
        [
          Alcotest.test_case "version" `Quick (fun () ->
              Alcotest.(check string)
                "version" "0.1.0"
                (Coding_adventures_my_pkg.version ()));
        ] );
    ]
