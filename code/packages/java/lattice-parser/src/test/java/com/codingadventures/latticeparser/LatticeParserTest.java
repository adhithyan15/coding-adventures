package com.codingadventures.latticeparser;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class LatticeParserTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("lattice-parser", new LatticeParser().ping());
    }
}
