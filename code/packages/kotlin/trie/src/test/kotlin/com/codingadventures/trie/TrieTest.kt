// ============================================================================
// TrieTest.kt — Unit Tests for Trie
// ============================================================================

package com.codingadventures.trie

import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class TrieTest {

    // =========================================================================
    // 1. insert / get
    // =========================================================================

    @Test
    fun insertAndGet() {
        val t = Trie<Int>()
        t.insert("apple", 1)
        assertEquals(1, t["apple"])
    }

    @Test
    fun getAbsentKey() {
        val t = Trie<Int>()
        t.insert("apple", 1)
        assertNull(t["app"])
        assertNull(t["apples"])
        assertNull(t["banana"])
    }

    @Test
    fun insertOverwritesValue() {
        val t = Trie<Int>()
        t.insert("apple", 1)
        t.insert("apple", 42)
        assertEquals(42, t["apple"])
        assertEquals(1, t.size)  // size unchanged on overwrite
    }

    @Test
    fun insertEmptyKey() {
        val t = Trie<String>()
        t.insert("", "empty")
        assertEquals("empty", t[""])
        assertEquals(1, t.size)
    }

    @Test
    fun insertNullValue() {
        // null values are permitted; contains() is the authoritative test for presence
        val t = Trie<Int>()
        t.insert("key", null)
        assertTrue(t.contains("key"))
        assertNull(t["key"])
    }

    // =========================================================================
    // 2. contains
    // =========================================================================

    @Test
    fun containsPresent() {
        val t = Trie<Int>()
        t.insert("app", 1)
        t.insert("apple", 2)
        assertTrue(t.contains("app"))
        assertTrue(t.contains("apple"))
    }

    @Test
    fun containsAbsent() {
        val t = Trie<Int>()
        t.insert("apple", 1)
        assertFalse(t.contains("app"))    // prefix, not inserted as key
        assertFalse(t.contains("apples"))
        assertFalse(t.contains("banana"))
    }

    // =========================================================================
    // 3. startsWith
    // =========================================================================

    @Test
    fun startsWithTrue() {
        val t = Trie<Int>()
        t.insert("apple", 1)
        t.insert("apply", 2)
        assertTrue(t.startsWith("app"))
        assertTrue(t.startsWith("appl"))
        assertTrue(t.startsWith("apple"))
        assertTrue(t.startsWith(""))
    }

    @Test
    fun startsWithFalse() {
        val t = Trie<Int>()
        t.insert("apple", 1)
        assertFalse(t.startsWith("b"))
        assertFalse(t.startsWith("applez"))
        assertFalse(t.startsWith("z"))
    }

    // =========================================================================
    // 4. delete
    // =========================================================================

    @Test
    fun deleteExistingKey() {
        val t = Trie<Int>()
        t.insert("apple", 1)
        t.insert("app", 2)
        assertTrue(t.delete("apple"))
        assertFalse(t.contains("apple"))
        assertTrue(t.contains("app"))   // sibling survives
        assertEquals(1, t.size)
    }

    @Test
    fun deleteAbsentKeyReturnsFalse() {
        val t = Trie<Int>()
        t.insert("apple", 1)
        assertFalse(t.delete("app"))    // prefix but not a complete key
        assertFalse(t.delete("banana"))
        assertEquals(1, t.size)
    }

    @Test
    fun deleteThenInsertSameKey() {
        val t = Trie<Int>()
        t.insert("apple", 1)
        t.delete("apple")
        t.insert("apple", 99)
        assertEquals(99, t["apple"])
        assertEquals(1, t.size)
    }

    @Test
    fun deleteDoesNotRemoveSharedPrefix() {
        val t = Trie<Int>()
        t.insert("apple", 1)
        t.insert("apply", 2)
        t.insert("app", 3)
        t.delete("apple")
        assertTrue(t.contains("apply"))
        assertTrue(t.contains("app"))
        assertFalse(t.contains("apple"))
        assertEquals(2, t.size)
    }

    // =========================================================================
    // 5. keysWithPrefix
    // =========================================================================

    @Test
    fun keysWithPrefixBasic() {
        val t = Trie<Int>()
        t.insert("app", 1)
        t.insert("apple", 2)
        t.insert("apply", 3)
        t.insert("apt", 4)
        t.insert("banana", 5)
        val results = t.keysWithPrefix("app")
        assertEquals(3, results.size)
        assertTrue("app" in results)
        assertTrue("apple" in results)
        assertTrue("apply" in results)
        assertFalse("apt" in results)
        assertFalse("banana" in results)
    }

    @Test
    fun keysWithPrefixEmpty() {
        val t = Trie<Int>()
        t.insert("app", 1)
        t.insert("banana", 2)
        val all = t.keysWithPrefix("")
        assertEquals(2, all.size)
        assertTrue("app" in all)
        assertTrue("banana" in all)
    }

    @Test
    fun keysWithPrefixNoMatch() {
        val t = Trie<Int>()
        t.insert("apple", 1)
        assertTrue(t.keysWithPrefix("z").isEmpty())
    }

    @Test
    fun keysWithPrefixExactMatch() {
        val t = Trie<Int>()
        t.insert("apple", 1)
        assertEquals(listOf("apple"), t.keysWithPrefix("apple"))
    }

    // =========================================================================
    // 6. keys()
    // =========================================================================

    @Test
    fun keysAll() {
        val t = Trie<Int>()
        t.insert("cat", 1)
        t.insert("car", 2)
        t.insert("dog", 3)
        val all = t.keys()
        assertEquals(3, all.size)
        assertTrue("cat" in all)
        assertTrue("car" in all)
        assertTrue("dog" in all)
    }

    // =========================================================================
    // 7. size / isEmpty
    // =========================================================================

    @Test
    fun sizeEmpty() {
        val t = Trie<Int>()
        assertEquals(0, t.size)
        assertTrue(t.isEmpty)
    }

    @Test
    fun sizeAfterInserts() {
        val t = Trie<Int>()
        t.insert("a", 1)
        t.insert("ab", 2)
        t.insert("abc", 3)
        assertEquals(3, t.size)
        assertFalse(t.isEmpty)
    }

    @Test
    fun sizeAfterDeleteAll() {
        val t = Trie<Int>()
        t.insert("a", 1)
        t.insert("b", 2)
        t.delete("a")
        t.delete("b")
        assertEquals(0, t.size)
        assertTrue(t.isEmpty)
    }

    // =========================================================================
    // 8. Unicode / special characters
    // =========================================================================

    @Test
    fun unicodeKeys() {
        val t = Trie<Int>()
        t.insert("café", 1)
        t.insert("cafés", 2)
        assertTrue(t.contains("café"))
        assertTrue(t.startsWith("caf"))
        assertEquals(2, t.size)
    }

    @Test
    fun singleCharKeys() {
        val t = Trie<Int>()
        for (c in 'a'..'z') {
            t.insert(c.toString(), c.code)
        }
        assertEquals(26, t.size)
        for (c in 'a'..'z') {
            assertEquals(c.code, t[c.toString()])
        }
    }

    // =========================================================================
    // 9. Large dataset smoke test
    // =========================================================================

    @Test
    fun largeDataset() {
        val t = Trie<Int>()
        val n = 1000
        for (i in 0 until n) t.insert("key$i", i)
        assertEquals(n, t.size)
        for (i in 0 until n) assertEquals(i, t["key$i"])
        assertEquals(n, t.keysWithPrefix("key").size)
    }
}
