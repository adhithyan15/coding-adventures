package com.codingadventures.rng

import kotlin.test.Test
import kotlin.test.assertEquals

class RngTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("rng", Rng().ping())
    }
}
