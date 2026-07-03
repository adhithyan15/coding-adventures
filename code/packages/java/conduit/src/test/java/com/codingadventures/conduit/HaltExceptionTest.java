package com.codingadventures.conduit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Map;
import org.junit.jupiter.api.Test;

/** Pure-JVM tests for {@link HaltException} (no native lib). */
class HaltExceptionTest {

    @Test
    void storesStatusAndBody() {
        HaltException e = new HaltException(503, "Maintenance");
        assertEquals(503, e.status());
        assertEquals("Maintenance", e.body());
        assertTrue(e.headersEncoded().isEmpty());
    }

    @Test
    void storesHeaders() {
        HaltException e = new HaltException(301, "", Map.of("location", "/new"));
        assertEquals(301, e.status());
        assertTrue(e.headersEncoded().startsWith("location="));
    }

    @Test
    void nullBodyBecomesEmpty() {
        assertEquals("", new HaltException(404, null).body());
    }

    @Test
    void isRuntimeExceptionAndThrowable() {
        HaltException e = assertThrows(HaltException.class, () -> {
            throw new HaltException(418, "teapot");
        });
        assertEquals(418, e.status());
        assertEquals("teapot", e.body());
    }

    @Test
    void messageIncludesStatus() {
        assertTrue(new HaltException(500, "x").getMessage().contains("500"));
    }
}
