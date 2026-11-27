#!/bin/sh
set -eu

temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT HUP INT TERM
opam install . --yes --with-test --with-dev-setup

cat >"$temporary/dune-project" <<'EOF'
(lang dune 3.16)
(name benchmark_consumer_smoke)
EOF
cat >"$temporary/dune" <<'EOF'
(executable
 (name main)
 (libraries coding-adventures-microbenchmark))
EOF
cat >"$temporary/main.ml" <<'EOF'
open Microbenchmark

let () =
  match analyze [ { baseline_ns = 100L; candidate_ns = 100L; order = Baseline_first } ] 95 1.05 with
  | Ok result when result.ratio = 1. -> print_endline "installed OCaml consumer passed"
  | _ -> failwith "installed OCaml consumer returned unexpected analysis"
EOF
opam exec -- dune exec --root "$temporary" ./main.exe

report="$temporary/native-report.json"
env GIT_REVISION= GITHUB_SHA= opam exec -- coding-adventures-array-microbenchmark --pairs 40 >"$report"
opam exec -- coding-adventures-array-microbenchmark --replay "$report"
truncate -s 4194305 "$temporary/oversized-report.json"
if opam exec -- coding-adventures-array-microbenchmark --replay "$temporary/oversized-report.json"; then
  echo 'replay adapter accepted a report above 4 MiB' >&2
  exit 1
fi
