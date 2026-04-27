package com.codingadventures.sqlparser;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class SqlParserTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("sql-parser", new SqlParser().ping());
    }
}
