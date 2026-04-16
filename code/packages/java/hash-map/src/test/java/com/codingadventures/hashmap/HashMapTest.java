package com.codingadventures.hashmap;

import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class HashMapTest {
    @Test
    void storesAndDeletesValues() {
        HashMap<String, Integer> map = new HashMap<>();
        map.set("alpha", 1).set("beta", 2);

        assertEquals(2, map.size());
        assertTrue(map.has("alpha"));
        assertEquals(2, map.get("beta"));
        assertTrue(map.delete("alpha"));
        assertFalse(map.has("alpha"));
    }

    @Test
    void preservesInsertionOrder() {
        HashMap<String, Integer> map = new HashMap<>();
        map.set("alpha", 1).set("beta", 2).set("gamma", 3);

        assertEquals(List.of("alpha", "beta", "gamma"), map.keys());
    }
}
