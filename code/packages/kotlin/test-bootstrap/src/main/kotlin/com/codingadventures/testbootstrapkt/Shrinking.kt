package com.codingadventures.testbootstrapkt

import com.codingadventures.testbootstrap.pbt.Shrink
import java.math.BigInteger
import java.util.function.Predicate

/*
 * Direct shrinking entry points.  In every one, `holds(x)` returns true when the
 * property HOLDS for x (the same polarity as Runner.check); the value passed in
 * must falsify it, otherwise the engine throws NotFalsifyingException, which
 * propagates unchanged.
 */

/**
 * Shrinks a falsifying integer toward [target] inside `[min, max]`.  Steps counts
 * accepted moves; the target probe is one move.
 */
fun shrinkInt(
    initial: BigInteger,
    target: BigInteger,
    min: BigInteger,
    max: BigInteger,
    maxSteps: Int = 1000,
    holds: (BigInteger) -> Boolean,
): Shrunk<BigInteger> = Shrink.shrinkInt(initial, target, min, max, Predicate { holds(it) }, maxSteps).toKotlin()

/** Shrinks a falsifying list: empty probe, halves, single deletion, then elements toward [elemTarget]. */
fun shrinkList(
    initial: List<BigInteger>,
    elemTarget: BigInteger,
    maxSteps: Int = 1000,
    holds: (List<BigInteger>) -> Boolean,
): Shrunk<List<BigInteger>> = Shrink.shrinkList(initial, elemTarget, Predicate { holds(it) }, maxSteps).toKotlin()

/** Shrinks falsifying bytes toward `0x00`, deleting where it can. */
fun shrinkBytes(
    initial: ByteArray,
    maxSteps: Int = 1000,
    holds: (ByteArray) -> Boolean,
): Shrunk<ByteArray> = Shrink.shrinkBytes(initial, Predicate { holds(it) }, maxSteps).toKotlin()

/** The first failing draw of a replay and what it shrinks to. */
data class Replay(val initialFail: BigInteger, val minimal: BigInteger)

/**
 * Replays a failure from nothing but [seed]: the first of up to 1000 draws that
 * falsifies [holds] is shrunk toward 0 over `[0, 1000]`.
 */
fun simulateReplay(seed: ULong, holds: (BigInteger) -> Boolean): Replay {
    val r = Shrink.simulateReplay(seed.toEngineSeed(), Predicate { holds(it) })
    return Replay(r.initialFail(), r.minimal())
}
