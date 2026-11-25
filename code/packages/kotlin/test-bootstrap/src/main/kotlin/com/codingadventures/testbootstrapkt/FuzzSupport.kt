package com.codingadventures.testbootstrapkt

import com.codingadventures.testbootstrap.fuzz.Bitmap
import com.codingadventures.testbootstrap.fuzz.FailureIds
import com.codingadventures.testbootstrap.fuzz.Signature
import com.codingadventures.testbootstrap.tfv.Tfv

/*
 * Kotlin wrappers for the non-mutator corpus operations of fuzz-v1 (VER06g
 * section 2.3): admission, crash dedup, failure ids, the public TAP line and the
 * strict corpus-text check.  Each delegates to the Java engine; the only Kotlin
 * additions are types (ULong edge ids, a named result) and the test seams.
 */

/** Whether a trace found something new, and how many unique new edges. */
data class Novelty(val admitted: Boolean, val newEdges: Int)

private fun List<ULong>.toLongArray(): LongArray = LongArray(size) { this[it].toLong() }

/**
 * Corpus admission by edge-set difference over full 64-bit ids.
 *
 * ```
 *   corpusAdmit([1, 2, 3], [2, 4]) -> Novelty(true, 1)     (edge 4 is new)
 *   corpusAdmit([1, 2, 3], [1, 2]) -> Novelty(false, 0)
 * ```
 */
fun corpusAdmit(existing: List<ULong>, trace: List<ULong>): Novelty {
    if (Seams.admitEverything) return Novelty(true, 1)
    val n = Bitmap.corpusAdmit(existing.toLongArray(), trace.toLongArray())
    return Novelty(n.admitted(), n.newEdges())
}

/** The power schedule: how many mutation slots a seed earns (VER06 section 4.3). */
fun calculateEnergy(baseEnergy: Int, seedLen: Int, newEdges: Int, totalEdges: Int): Int =
    Bitmap.calculateEnergy(baseEnergy, seedLen, newEdges, totalEdges)

/** The 8-lowercase-hex signature of a crash kind and its frames (`lower32(fnv1a64(kind:frames))`). */
fun crashSignature(kind: String, frames: List<String>): String = Signature.crashSignature(kind, frames)

/** True when two crashes have the same signature. */
fun isDuplicateCrash(kindA: String, framesA: List<String>, kindB: String, framesB: List<String>): Boolean {
    val same = Signature.isDuplicateCrash(kindA, framesA, kindB, framesB)
    return if (Seams.invertDedup) !same else same
}

/** The closed fuzz failure vocabulary (VER06 section 6); the engine owns the list. */
object FuzzFailureId {
    const val CRASH: String = "fuzz-crash"
    const val TIMEOUT: String = "fuzz-timeout"
    const val OOM: String = "fuzz-oom"

    /** Registered, but this engine cannot produce it. */
    const val LEAK: String = "fuzz-leak"

    /** Registered, but this engine cannot produce it. */
    const val PANIC: String = "fuzz-panic"

    /** All five registered ids. */
    val ALL: List<String> get() = FailureIds.ALL

    /** True when [candidate] is one of the registered ids. */
    fun isValid(candidate: String?): Boolean = Seams.acceptAnyFailureId || FailureIds.isFuzzFailureId(candidate)
}

/**
 * The only public failure line: `not ok N - target [failure-id:hex8]`.  It never
 * carries input bytes or a seed.  Throws `IllegalArgumentException` for an
 * unregistered id or a hash that is not exactly 8 lowercase hex digits.
 */
fun formatTapFailure(testNum: Int, target: String, failureId: String, sigHash: String): String {
    val line = Signature.formatTapFailure(testNum, target, failureId, sigHash)
    return if (Seams.leakTapTarget) "$line $target" else line
}

/** True when [text] is a valid strict corpus document (the `corpus_reject` check). */
fun tfvIsValid(text: String): Boolean = Tfv.isValid(text)
