---
category: Testing & coverage
---

# A parallel JVM's hsperfdata lock warning lands on stdout; pass -XX:-UsePerfData when tests compare java's output

lang-aot's op_conformance failed intermittently in CI: a case expected stdout
`7` but got `[0.001s][warning][perf,memops] Cannot use file
/tmp/hsperfdata_runner/82868 because it is locked by another process` first.

HotSpot writes perf counters to `/tmp/hsperfdata_<user>/<pid>`. Under heavy
parallelism (build tool across packages, plus test threads) a recycled PID's
file can still be locked; the JVM then logs a unified-logging warning, and
unified logging writes to STDOUT by default. Tests that compare stdout
byte-for-byte see the warning as program output.

Fix: pass `-XX:-UsePerfData` to every `java` invocation that runs a program and
inspects its output. No file, no lock, no warning. Do not filter the warning
out of stdout: that hides the symptom while leaving the race. `java -version`
availability probes only check the exit status and are unaffected.
