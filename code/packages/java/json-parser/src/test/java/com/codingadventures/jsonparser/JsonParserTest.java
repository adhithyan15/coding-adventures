package com.codingadventures.jsonparser;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class JsonParserTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("json-parser", new JsonParser().ping());
    }
}
