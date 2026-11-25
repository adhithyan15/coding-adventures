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

    /** Restores production behaviour. */
    @JvmSynthetic
    internal fun reset() {
        seedOffset = 0L
        shrinkStepDelta = 0
        swapBounds = false
    }
}
