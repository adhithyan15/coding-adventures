package com.codingadventures.sqllexer;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class SqlLexerTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("sql-lexer", new SqlLexer().ping());
    }
}
