package com.codingadventures.tomlparser;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class TomlParserTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("toml-parser", new TomlParser().ping());
    }
}
