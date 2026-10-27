---
category: Testing & coverage
---

# The agent container's JAVA_TOOL_OPTIONS banner makes every lang-aot JVM cell fail locally; unset it before running lang_matrix

**What went wrong.** I was checking a vm-core change against its dependents.
`cargo test -p lang-aot` reported `487 passed; 64 failed`. Every one of the
64 was a JVM cell (61 ALGOL, 3 Twig, plus the matrix roll-ups), failing with
`parsing the entry result the launcher printed to stderr failed — this is a
REAL failure, not a skip`. The launcher's stderr began with
`Picked up JAVA_TOOL_OPTIONS: -Djavax.net.ssl.trustStore=… -Dhttps.proxyHost=…`
and ended with the expected `42`. The agent container sets `JAVA_TOOL_OPTIONS`
for its HTTPS proxy, and the JVM prints that banner on stderr. The JVM column
reads its exit value from stderr, so the banner broke the parse. The change
under test had nothing to do with it.

**The fix.** Rerun with the variable unset:
`env -u JAVA_TOOL_OPTIONS cargo test -p lang-aot --test lang_matrix -- --exact <names>`.
All 64 passed. CI runners do not set the variable, so CI is unaffected.

**Do differently.** Before running `lang-aot` (or any test that parses a JVM
launcher's stderr) in an agent container, check `echo $JAVA_TOOL_OPTIONS`, and
unset it for the run. When a whole backend column fails at once, group the
failure messages (`grep -A1 "panicked at" | sort | uniq -c`) before you start
debugging. One shared message across every cell points at the environment,
not at the change. Budget for the time, too: the full `lang_matrix` run took
about 13 minutes here, and the 64-test rerun took about 30.
