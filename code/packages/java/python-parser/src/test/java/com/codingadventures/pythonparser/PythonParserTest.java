package com.codingadventures.pythonparser;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class PythonParserTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("python-parser", new PythonParser().ping());
    }
}
