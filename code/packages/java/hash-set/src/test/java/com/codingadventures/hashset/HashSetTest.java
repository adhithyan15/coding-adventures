package com.codingadventures.hashset;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class HashSetTest {
    @Test
    void supportsMembershipAndRemoval() {
        HashSet<String> set = new HashSet<>();
        set.add("alpha").add("beta");

        assertTrue(set.contains("alpha"));
        assertEquals(2, set.size());
        assertTrue(set.remove("alpha"));
        assertFalse(set.contains("alpha"));
    }

    @Test
    void supportsSetAlgebra() {
        HashSet<String> left = new HashSet<>();
        left.add("alpha").add("beta");
        HashSet<String> right = new HashSet<>();
        right.add("beta").add("gamma");

        assertEquals(3, left.union(right).size());
        assertEquals(1, left.intersection(right).size());
        assertTrue(left.difference(right).contains("alpha"));
    }
}
