open Coding_adventures_test_bootstrap

let () =
  let r = Corpus.evaluate_text Corpus_data.json in
  List.iter print_endline r.problems;
  Printf.printf "pbt-v1: %d/25\n" r.evaluated
