package org.mosaic.flux

import kotlin.test.Test
import kotlin.test.assertEquals

private data class MwS(val v: Int = 0)

private class MwBump : MosaicAction<MwS> {
    override fun apply(state: MwS): MwS = state.copy(v = state.v + 1)
}

class MiddlewareTest {
    @Test
    fun emptyComposeIsNoOp() {
        val m = composeMiddleware<MwS>(emptyList())
        m(MwBump(), MwS(), MwS(v = 1))
    }

    @Test
    fun singleMiddlewareReturnedVerbatim() {
        val m: Middleware<MwS> = { _, _, _ -> }
        assertEquals(m, composeMiddleware(listOf(m)))
    }

    @Test
    fun runsInOrder() {
        val calls = mutableListOf<String>()
        val composed = composeMiddleware<MwS>(listOf(
            { _, _, _ -> calls.add("a") },
            { _, _, _ -> calls.add("b") },
            { _, _, _ -> calls.add("c") },
        ))
        composed(MwBump(), MwS(), MwS(v = 1))
        assertEquals(listOf("a", "b", "c"), calls)
    }

    @Test
    fun isolatesThrows() {
        val calls = mutableListOf<String>()
        val composed = composeMiddleware<MwS>(listOf(
            { _, _, _ -> calls.add("a") },
            { _, _, _ -> throw RuntimeException("boom") },
            { _, _, _ -> calls.add("c") },
        ))
        composed(MwBump(), MwS(), MwS(v = 1))
        assertEquals(listOf("a", "c"), calls)
    }

    @Test
    fun loggerMiddlewareDoesNotThrow() {
        val m = loggerMiddleware<MwS>()
        m(MwBump(), MwS(), MwS(v = 1))
    }
}
