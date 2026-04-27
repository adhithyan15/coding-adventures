package com.codingadventures.javaparser

import kotlin.test.Test
import kotlin.test.assertEquals

class JavaParserTest {
    @Test
    fun pingReturnsPackageName() {
        assertEquals("java-parser", JavaParser().ping())
    }
}
