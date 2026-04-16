package com.codingadventures.javaparser;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class JavaParserTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("java-parser", new JavaParser().ping());
    }
}
