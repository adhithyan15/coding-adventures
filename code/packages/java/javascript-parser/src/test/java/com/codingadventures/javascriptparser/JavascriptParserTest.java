package com.codingadventures.javascriptparser;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class JavascriptParserTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("javascript-parser", new JavascriptParser().ping());
    }
}
