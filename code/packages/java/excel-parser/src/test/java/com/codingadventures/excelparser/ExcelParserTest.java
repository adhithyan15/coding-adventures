package com.codingadventures.excelparser;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class ExcelParserTest {
    @Test
    void pingReturnsPackageName() {
        assertEquals("excel-parser", new ExcelParser().ping());
    }
}
