// Selector.kt — memoised derived-state combinator.
//
// For derived state computed from multiple slots of the same state,
// recomputing on every selector call is wasteful.  createSelector
// memoises by input equality: if every input returned the same value
// as last call, the cached output is reused.
//
// v0.1.0 provides 1-, 2-, and 3-input variants.

package org.mosaic.flux

/**
 * Build a single-input memoised selector.  Combiner runs only when
 * the input changes (by ==).
 */
fun <S, A, R> createSelector(
    inputA: (S) -> A,
    combine: (A) -> R,
): (S) -> R {
    var lastInput: A? = null
    var hasLast = false
    var lastResult: R? = null
    return { state ->
        val a = inputA(state)
        @Suppress("UNCHECKED_CAST")
        if (hasLast && lastInput == a) {
            lastResult as R
        } else {
            lastInput = a
            hasLast = true
            val r = combine(a)
            lastResult = r
            r
        }
    }
}

/**
 * Build a two-input memoised selector.
 */
fun <S, A, B, R> createSelector(
    inputA: (S) -> A,
    inputB: (S) -> B,
    combine: (A, B) -> R,
): (S) -> R {
    var lastA: A? = null
    var lastB: B? = null
    var hasLast = false
    var lastResult: R? = null
    return { state ->
        val a = inputA(state)
        val b = inputB(state)
        @Suppress("UNCHECKED_CAST")
        if (hasLast && lastA == a && lastB == b) {
            lastResult as R
        } else {
            lastA = a
            lastB = b
            hasLast = true
            val r = combine(a, b)
            lastResult = r
            r
        }
    }
}

/**
 * Build a three-input memoised selector.
 */
fun <S, A, B, C, R> createSelector(
    inputA: (S) -> A,
    inputB: (S) -> B,
    inputC: (S) -> C,
    combine: (A, B, C) -> R,
): (S) -> R {
    var lastA: A? = null
    var lastB: B? = null
    var lastC: C? = null
    var hasLast = false
    var lastResult: R? = null
    return { state ->
        val a = inputA(state)
        val b = inputB(state)
        val c = inputC(state)
        @Suppress("UNCHECKED_CAST")
        if (hasLast && lastA == a && lastB == b && lastC == c) {
            lastResult as R
        } else {
            lastA = a
            lastB = b
            lastC = c
            hasLast = true
            val r = combine(a, b, c)
            lastResult = r
            r
        }
    }
}
