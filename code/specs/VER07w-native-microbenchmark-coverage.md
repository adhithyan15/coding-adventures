# VER07w — Native coverage instrumentation and evidence

Complete measurable source coverage for the existing native microbenchmark and
rank-statistics implementations without substituting coverage for correctness.
Preserve their independent oracles, properties, bounds, native clock and artifact
checks. Instrument the same production sources and identify the exact revisions,
toolchains, platforms, included files and excluded dependencies in the evidence.

OCaml libraries and CLI must explicitly opt into Bisect instrumentation in their
Dune stanzas; passing `--instrument-with` alone does not instrument unmarked
targets. Keep instrumentation optional in normal builds. Package test dependencies
and BUILD commands must install/enable the backend, retain raw coverage data and
produce a quantitative report. Test substantive uncovered behavior when needed;
do not add test-only production exports or mask uncovered branches with filters.
The rank-statistics project must also provide its own ocamlformat configuration:
a successful `@fmt` invocation with source formatting disabled is not a lint check.

Measure C/C++ with available native compiler coverage support, Rust with its
matching LLVM tools, Lua with LuaCov and Perl with Devel::Cover where practical.
Report line/statement, branch/condition or expression coverage according to each
tool's actual metric; they are not interchangeable percentages. Document missing
tool/platform evidence explicitly. Native C extensions require their own metrics
instead of being silently included in interpreter-only coverage claims.

Recheck installed Lua package consumers using a task-local explicit LuaRocks
configuration rather than treating source-copy staging as an actual rock install.
Do not change global tool configuration or terminate unrelated processes.
Any reported build/install failures must retain their actual diagnosis and limits.

Instrumentation changes must preserve ordinary tests, native default-setting report
and replay, and downstream package consumption. Run lint/vet and validate lessons,
then obtain independent review before publishing. The full suite remains incomplete
until guest lanes, required CI and outstanding validation obligations are resolved.
