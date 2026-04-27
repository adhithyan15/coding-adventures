package com.codingadventures.latticeparser

import kotlin.test.Test
import kotlin.test.assertEquals

class LatticeParserTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("lattice-parser", LatticeParser().ping())
    }
}
