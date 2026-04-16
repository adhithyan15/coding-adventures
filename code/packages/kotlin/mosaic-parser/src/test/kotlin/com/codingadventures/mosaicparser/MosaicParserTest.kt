package com.codingadventures.mosaicparser

import kotlin.test.Test
import kotlin.test.assertEquals

class MosaicParserTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("mosaic-parser", MosaicParser().ping())
    }
}
