package com.codingadventures.csharpparser;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class CsharpParserTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("csharp-parser", new CsharpParser().ping());
    }
}
