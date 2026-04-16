package com.codingadventures.hashset

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class HashSetTest {
    @Test
    fun supportsMembershipAndSetAlgebra() {
        val left = HashSet<String>().add("alpha").add("beta")
        val right = HashSet<String>().add("beta").add("gamma")

        assertTrue(left.contains("alpha"))
        assertTrue(left.remove("alpha"))
        assertFalse(left.contains("alpha"))
        assertEquals(2, left.union(right).size)
        assertTrue(left.intersection(right).contains("beta"))
    }
}
