package com.codingadventures.testbootstrapkt

import com.codingadventures.testbootstrap.pbt.CiMode as JCiMode
import com.codingadventures.testbootstrap.pbt.FailureIds
import com.codingadventures.testbootstrap.pbt.Options as JOptions
import com.codingadventures.testbootstrap.pbt.Runner
import com.codingadventures.testbootstrap.pbt.Verdict as JVerdict
import com.codingadventures.testbootstrap.prng.SplitMix64
import java.util.function.Predicate

/**
 * Whether output is public CI output.  [UNSET], the default, redacts: the engine
 * fails closed because a seed plus a property is a working exploit reproducer.
 */
enum class CiMode { UNSET, ON, OFF }

/** Configures a run.  `numTests = 0` is legal and is a failure: a property that never ran proves nothing. */
data class Options(
    val numTests: Int = 100,
    val maxShrinkSteps: Int = 1000,
    val ci: CiMode = CiMode.UNSET,
) {
    @JvmSynthetic
    internal fun toEngine(): JOptions = JOptions(numTests, maxShrinkSteps, JCiMode.valueOf(ci.name))
}

/**
 * Decides redaction from an environment lookup (pass `System::getenv`): `ON`
 * when `CI` is set to anything but empty, "0" or "false", otherwise `OFF`.
 */
fun ciModeFrom(lookup: (String) -> String?): CiMode =
    if (JOptions.ciFromLookup { lookup(it) }) CiMode.ON else CiMode.OFF

/** The closed failure vocabulary of VER05 (the engine owns the list). */
object FailureId {
    const val COUNTEREXAMPLE: String = "property-counterexample"
    const val EXHAUSTED: String = "property-exhausted"
    const val TIMEOUT: String = "property-timeout"
    const val PANIC: String = "property-panic"

    /** All four registered ids. */
    val ALL: List<String> get() = FailureIds.ALL

    /** True when [candidate] is one of the registered ids. */
    fun isValid(candidate: String?): Boolean = FailureIds.isPbtFailureId(candidate)
}

/**
 * The outcome of a run.  In redacted mode a [Failed] verdict has no [seed],
 * [counterexample] or [minimal]; only the message, iteration, step count and
 * [digest] remain.
 */
sealed class Verdict<out T> {
    /** True for [Passed]. */
    abstract val passed: Boolean

    /** The negation of [passed]. */
    val failed: Boolean get() = !passed

    /** All [casesRun] cases held. */
    data class Passed(val casesRun: Int) : Verdict<Nothing>() {
        override val passed: Boolean get() = true
    }

    /** The property did not hold, or the run could not be honest about it (see [failureId]). */
    data class Failed<out T>(
        val failureId: String,
        val message: String,
        val casesRun: Int,
        val iteration: Int,
        val shrinkSteps: Int,
        val seed: ULong?,
        val counterexample: T?,
        val minimal: T?,
        val digest: String?,
    ) : Verdict<T>() {
        override val passed: Boolean get() = false
    }

    /** Throws an `AssertionError` carrying the failure id unless this is a [Passed]. */
    fun assertPassed() {
        if (this is Failed) throw AssertionError("[$failureId] $message")
    }
}

@Suppress("UNCHECKED_CAST")
private fun <T> JVerdict.toKotlin(): Verdict<T> =
    if (passed()) {
        Verdict.Passed(casesRun())
    } else {
        Verdict.Failed(
            failureId = failureId(),
            message = message(),
            casesRun = casesRun(),
            iteration = iteration(),
            shrinkSteps = shrinkSteps(),
            seed = seed()?.toULong(),
            counterexample = counterexample() as T?,
            minimal = minimal() as T?,
            digest = digest(),
        )
    }

/**
 * Runs [property] over `options.numTests` candidates drawn from [seed].
 * [property] returns true when the property HOLDS.  An explicit [shrinker]
 * overrides the generator's own (it exists so a defective shrinker can be tested).
 *
 * Invalid options throw `IllegalArgumentException`; no verdict is produced.
 */
fun <T> checkProperty(
    seed: ULong,
    gen: Gen<T>,
    options: Options = Options(),
    shrinker: ShrinkFn<T>? = null,
    property: (T) -> Boolean,
): Verdict<T> {
    return Runner.check(seed.toEngineSeed(), gen.handle, Predicate { property(it) }, options.toEngine(), shrinker?.toEngine())
        .toKotlin()
}

/** `forAll(gen) { x -> ... }`: [checkProperty] with the seed first-class and a trailing property. */
fun <T> forAll(
    gen: Gen<T>,
    seed: ULong = 0uL,
    options: Options = Options(),
    property: (T) -> Boolean,
): Verdict<T> = checkProperty(seed, gen, options, null, property)

/** `SplitMix64(seed ^ fnv1a64(id)).nextU64()`: give each property its own sub-seed. */
fun subSeed(runSeed: ULong, id: String): ULong = SplitMix64.subSeed(runSeed.toLong(), id).toULong()
