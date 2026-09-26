### Added -- an execution acceptance for the Compose host

`tests/compose_effect_completion.rs` emits the host, compiles it with `kotlinc`
against the real JNA and kotlinx-serialization jars, and runs it against the
conformance runtime -- one JVM per scenario, each with its own state file,
because the host reads `MOSAIC_APP_STATE_PATH` once at load and the JVM cannot
change its own environment. Seven scenarios: an unanswered await, an answered
one, a fully-answered chaining batch, a partly-answered batch, a throwing
handler, an unconvertible result, a runaway chain, and
defer-then-answer-from-another-thread.

The runaway case pins the 64-round settle bound, which is what stops a handler
that answers every effect by minting another from spinning inside a
`@Synchronized` method while holding the monitor. The bound has to both stop
**and** report: giving up quietly would leave the app looking settled while the
runtime still waits.

`consume` now bounds the native length below as well as above. `MosaicSizeT` is
an unsigned `IntegerType`, so a 64-bit `size_t` with the high bit set arrives as
a negative `Long` and sailed past the `<=` test into `getByteArray` with a
negative count. JNA rejects that, so this was never an out-of-bounds read -- but
the guard read as though it checked, and did not.

It skips when `kotlinc`, `java` or the jars are absent, with environment
overrides (`MOSAIC_JNA_JAR`, `MOSAIC_KOTLINX_JSON_JAR`,
`MOSAIC_KOTLINX_CORE_JAR`, `MOSAIC_KOTLIN_STDLIB_JAR`) to point it at them.
The stdlib is resolved separately because `kotlinc` supplies it at compile time
and `java` does not at run time -- a host that compiles cleanly still dies with
`NoClassDefFoundError: kotlin/Result` without it.

