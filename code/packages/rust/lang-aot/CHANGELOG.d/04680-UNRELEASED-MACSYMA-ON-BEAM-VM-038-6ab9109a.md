## Unreleased — Macsyma on BEAM (VM-038)

Extend the existing 21-program arithmetic/assignment conformance corpus with
real Erlang execution. Assert successful process exit and full signed integer
results; retain explicit missing-tool skips and unconditional corpus emission
checks. Declare the Elixir toolchain in BUILD so hosted LANG PRs install Erlang.
No production lowering change was required.

