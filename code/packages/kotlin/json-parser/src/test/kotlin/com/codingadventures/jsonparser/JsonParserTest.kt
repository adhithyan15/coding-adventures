package com.codingadventures.jsonparser

import kotlin.test.Test
import kotlin.test.assertEquals

class JsonParserTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("json-parser", JsonParser().ping())
    }
}
