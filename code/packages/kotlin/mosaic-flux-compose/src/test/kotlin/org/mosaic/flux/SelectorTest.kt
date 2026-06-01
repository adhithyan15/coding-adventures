package org.mosaic.flux

import kotlin.test.Test
import kotlin.test.assertEquals

private data class SelS(val a: Int = 0, val b: Int = 0, val label: String = "")

class SelectorTest {
    @Test
    fun singleInputRecomputesOnChange() {
        var calls = 0
        val doubled = createSelector<SelS, Int, Int>(
            { it.a },
            { a -> calls++; a * 2 },
        )
        assertEquals(10, doubled(SelS(a = 5)))
        assertEquals(14, doubled(SelS(a = 7)))
        assertEquals(2, calls)
    }

    @Test
    fun singleInputCachesOnStable() {
        var calls = 0
        val doubled = createSelector<SelS, Int, Int>(
            { it.a },
            { a -> calls++; a * 2 },
        )
        val s = SelS(a = 5)
        doubled(s); doubled(s); doubled(s)
        assertEquals(1, calls)
    }

    @Test
    fun singleInputCachesAcrossStateRefs() {
        var calls = 0
        val doubled = createSelector<SelS, Int, Int>(
            { it.a },
            { a -> calls++; a * 2 },
        )
        doubled(SelS(a = 5, b = 0))
        doubled(SelS(a = 5, b = 999, label = "different"))
        assertEquals(1, calls)
    }

    @Test
    fun twoInputRecomputesWhenEitherChanges() {
        var calls = 0
        val sum = createSelector<SelS, Int, Int, Int>(
            { it.a },
            { it.b },
            { a, b -> calls++; a + b },
        )
        assertEquals(3, sum(SelS(a = 1, b = 2)))
        assertEquals(6, sum(SelS(a = 1, b = 5)))
        assertEquals(9, sum(SelS(a = 4, b = 5)))
        assertEquals(3, calls)
    }

    @Test
    fun threeInputRecomputesWhenAnyChanges() {
        var calls = 0
        val fmt = createSelector<SelS, Int, Int, String, String>(
            { it.a },
            { it.b },
            { it.label },
            { a, b, lbl -> calls++; "$lbl:${a + b}" },
        )
        assertEquals("x:3", fmt(SelS(a = 1, b = 2, label = "x")))
        assertEquals("x:3", fmt(SelS(a = 1, b = 2, label = "x")))
        assertEquals("y:3", fmt(SelS(a = 1, b = 2, label = "y")))
        assertEquals(2, calls)
    }
}
