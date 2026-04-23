package com.codingadventures.rubyparser;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class RubyParserTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("ruby-parser", new RubyParser().ping());
    }
}
