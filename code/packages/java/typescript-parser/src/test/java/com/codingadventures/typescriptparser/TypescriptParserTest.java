package com.codingadventures.typescriptparser;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class TypescriptParserTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("typescript-parser", new TypescriptParser().ping());
    }
}
