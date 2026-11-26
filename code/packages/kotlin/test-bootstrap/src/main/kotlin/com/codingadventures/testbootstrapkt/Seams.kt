package com.codingadventures.testbootstrapkt

/**
 * Test-only sabotage seams for the Kotlin layer (VER05f section 4).
 *
 * The Java engine has its own sabotage hooks, but they only prove that the Java
 * corpus test notices a broken Java engine.  These seams break the *Kotlin
 * wrapper* instead, so a test can show that replaying the corpus through the
 * Kotlin API really exercises the Kotlin layer: if a wrapper could be wrong
 * without the corpus noticing, the corpus would be testing Java twice.
 *
 * Production code never writes these fields.  They are `internal` and
 * `@JvmSynthetic`, because Kotlin `internal` is not JVM privacy: without the
 * annotation Java callers could flip them.
 */
internal object Seams {
    /** Added to every seed the wrapper hands to the engine (wrapping, as seeds do). */
    @get:JvmSynthetic @set:JvmSynthetic
    @Volatile
    internal var seedOffset: Long = 0L

    /** Added to every shrink step count the wrapper reports. */
    @get:JvmSynthetic @set:JvmSynthetic
    @Volatile
    internal var shrinkStepDelta: Int = 0

    /** When true, `genBounded` swaps its bounds before delegating. */
    @get:JvmSynthetic @set:JvmSynthetic
    @Volatile
    internal var swapBounds: Boolean = false

    /** VER06g: added to the offset every fuzz mutator wrapper passes on. */
    @get:JvmSynthetic @set:JvmSynthetic
    @Volatile
    internal var fuzzOffsetDelta: Long = 0L

    /** VER06g: multi-byte mutator wrappers flip the byte order. */
    @get:JvmSynthetic @set:JvmSynthetic
    @Volatile
    internal var swapEndian: Boolean = false

    /** VER06g: `isDuplicateCrash` returns the opposite answer. */
    @get:JvmSynthetic @set:JvmSynthetic
    @Volatile
    internal var invertDedup: Boolean = false

    /** VER06g: `formatTapFailure` appends the target name a second time (a leak). */
    @get:JvmSynthetic @set:JvmSynthetic
    @Volatile
    internal var leakTapTarget: Boolean = false

    /** VER06g: `corpusAdmit` always reports a novelty. */
    @get:JvmSynthetic @set:JvmSynthetic
    @Volatile
    internal var admitEverything: Boolean = false

    /** VER06g: `FuzzFailureId.isValid` accepts everything. */
    @get:JvmSynthetic @set:JvmSynthetic
    @Volatile
    internal var acceptAnyFailureId: Boolean = false

    /** Restores production behaviour. */
    @JvmSynthetic
    internal fun reset() {
        seedOffset = 0L
        shrinkStepDelta = 0
        swapBounds = false
        fuzzOffsetDelta = 0L
        swapEndian = false
        invertDedup = false
        leakTapTarget = false
        admitEverything = false
        acceptAnyFailureId = false
    }
}
