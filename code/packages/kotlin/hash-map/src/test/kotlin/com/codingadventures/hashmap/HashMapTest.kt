package com.codingadventures.hashmap

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class HashMapTest {
    @Test
    fun storesAndDeletesValues() {
        val map = HashMap<String, Int>()
        map.set("alpha", 1).set("beta", 2)

        assertEquals(2, map.size)
        assertTrue(map.has("alpha"))
        assertEquals(2, map["beta"])
        assertTrue(map.delete("alpha"))
        assertFalse(map.has("alpha"))
    }
}
