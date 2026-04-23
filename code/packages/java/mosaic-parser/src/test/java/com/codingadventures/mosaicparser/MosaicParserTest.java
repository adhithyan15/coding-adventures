package com.codingadventures.mosaicparser;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class MosaicParserTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("mosaic-parser", new MosaicParser().ping());
    }
}
