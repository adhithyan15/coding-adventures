package com.codingadventures.skiplist

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class SkipListTest {
    @Test
    fun insertsSearchesAndDeletesInSortedOrder() {
        val list = SkipList<Int, String>()
        list.insert(20, "b")
        list.insert(10, "a")
        list.insert(30, "c")

        assertEquals(listOf(10, 20, 30), list.toList())
        assertEquals("b", list.search(20))
        assertTrue(list.delete(20))
        assertNull(list.search(20))
        assertEquals(listOf(10, 30), list.toList())
    }

    @Test
    fun computesRankAndByRankConsistently() {
        val list = SkipList<Int, String>()
        listOf(50, 10, 30, 20).forEach { key -> list.insert(key, key.toString()) }

        assertEquals(0, list.rank(10))
        assertEquals(1, list.rank(20))
        assertEquals(30, list.byRank(2))

        val strings = SkipList<String, Int>()
        strings.insert("alpha", 1)
        assertNull(strings.byRank(10))
    }

    @Test
    fun returnsBoundedRanges() {
        val list = SkipList<Int, String>()
        listOf(10, 20, 30, 40, 50).forEach { key -> list.insert(key, key.toString()) }

        assertEquals(listOf(20 to "20", 30 to "30", 40 to "40"), list.range(15, 45, true))
        assertEquals(listOf(20 to "20", 30 to "30"), list.range(10, 40, false))
    }

    @Test
    fun supportsCustomComparatorsForCompositeKeys() {
        val list = SkipList<ScoreMember, String>({ left, right ->
            compareValuesBy(left, right, ScoreMember::score, ScoreMember::member)
        })
        list.insert(ScoreMember(10.0, "b"), "b")
        list.insert(ScoreMember(10.0, "a"), "a")
        list.insert(ScoreMember(5.0, "z"), "z")

        assertEquals(
            listOf(ScoreMember(5.0, "z"), ScoreMember(10.0, "a"), ScoreMember(10.0, "b")),
            list.toList(),
        )
    }

    @Test
    fun exposesHelpersAndBoundaryBehaviors() {
        val list = SkipList.withParams<Int, String>(maxLevel = -1, probability = 2.0)
        val emptyStrings = SkipList<String, Int>()
        assertTrue(list.isEmpty())
        assertEquals(1, list.maxLevel())
        assertEquals(0.5, list.probability())
        assertEquals(1, list.currentMax())
        assertNull(emptyStrings.min())
        assertNull(emptyStrings.max())
        assertEquals(emptyList(), list.rangeQuery(10, 1, true))

        list.insert(1, "one")
        list.insert(2, "two")
        list.insert(2, "two-updated")
        list.insert(3, "three")

        assertTrue(list.contains(2))
        assertTrue(list.containsKey(3))
        assertFalse(list.contains(99))
        assertEquals("two-updated", list.search(2))
        assertEquals(3, list.len())
        assertEquals(3, list.size())
        assertEquals(1, list.min())
        assertEquals(3, list.max())
        assertEquals(1, list.byRank(0))
        assertNull(list.rank(99))
        assertTrue(list.currentMax() >= 1)
        assertEquals(listOf(1 to "one", 2 to "two-updated", 3 to "three"), list.entries())
        assertTrue(list.delete(3))
        assertFalse(list.delete(3))
    }

    private data class ScoreMember(val score: Double, val member: String)
}
