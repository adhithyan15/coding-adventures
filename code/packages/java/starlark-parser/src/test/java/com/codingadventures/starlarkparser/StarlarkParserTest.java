package com.codingadventures.starlarkparser;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class StarlarkParserTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("starlark-parser", new StarlarkParser().ping());
    }
}
