// ============================================================================
// TrieTest.java — Unit Tests for Trie
// ============================================================================

package com.codingadventures.trie;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

import java.util.List;
import java.util.Optional;

class TrieTest {

    // =========================================================================
    // 1. insert / get
    // =========================================================================

    @Test
    void insertAndGet() {
        Trie<Integer> t = new Trie<>();
        t.insert("apple", 1);
        assertEquals(Optional.of(1), t.get("apple"));
    }

    @Test
    void getAbsentKey() {
        Trie<Integer> t = new Trie<>();
        t.insert("apple", 1);
        assertEquals(Optional.empty(), t.get("app"));
        assertEquals(Optional.empty(), t.get("apples"));
        assertEquals(Optional.empty(), t.get("banana"));
    }

    @Test
    void insertOverwritesValue() {
        Trie<Integer> t = new Trie<>();
        t.insert("apple", 1);
        t.insert("apple", 42);
        assertEquals(Optional.of(42), t.get("apple"));
        assertEquals(1, t.size()); // size unchanged on overwrite
    }

    @Test
    void insertEmptyKey() {
        Trie<String> t = new Trie<>();
        t.insert("", "empty");
        assertEquals(Optional.of("empty"), t.get(""));
        assertEquals(1, t.size());
    }

    @Test
    void insertRejectsNullKey() {
        Trie<Integer> t = new Trie<>();
        assertThrows(IllegalArgumentException.class, () -> t.insert(null, 1));
    }

    @Test
    void insertNullValue() {
        // null values are allowed; get() returns Optional.empty() for them
        // because Optional.ofNullable(null) == Optional.empty()
        Trie<Integer> t = new Trie<>();
        t.insert("key", null);
        assertTrue(t.contains("key"));
        assertEquals(Optional.empty(), t.get("key")); // null value → empty Optional
    }

    // =========================================================================
    // 2. contains
    // =========================================================================

    @Test
    void containsPresent() {
        Trie<Integer> t = new Trie<>();
        t.insert("app", 1);
        t.insert("apple", 2);
        assertTrue(t.contains("app"));
        assertTrue(t.contains("apple"));
    }

    @Test
    void containsAbsent() {
        Trie<Integer> t = new Trie<>();
        t.insert("apple", 1);
        assertFalse(t.contains("app"));   // prefix, not inserted as key
        assertFalse(t.contains("apples"));
        assertFalse(t.contains("banana"));
    }

    // =========================================================================
    // 3. startsWith
    // =========================================================================

    @Test
    void startsWithTrue() {
        Trie<Integer> t = new Trie<>();
        t.insert("apple", 1);
        t.insert("apply", 2);
        assertTrue(t.startsWith("app"));
        assertTrue(t.startsWith("appl"));
        assertTrue(t.startsWith("apple"));
        assertTrue(t.startsWith(""));
    }

    @Test
    void startsWithFalse() {
        Trie<Integer> t = new Trie<>();
        t.insert("apple", 1);
        assertFalse(t.startsWith("b"));
        assertFalse(t.startsWith("applez"));
        assertFalse(t.startsWith("z"));
    }

    // =========================================================================
    // 4. delete
    // =========================================================================

    @Test
    void deleteExistingKey() {
        Trie<Integer> t = new Trie<>();
        t.insert("apple", 1);
        t.insert("app", 2);
        assertTrue(t.delete("apple"));
        assertFalse(t.contains("apple"));
        assertTrue(t.contains("app"));   // sibling survives
        assertEquals(1, t.size());
    }

    @Test
    void deleteAbsentKeyReturnsFalse() {
        Trie<Integer> t = new Trie<>();
        t.insert("apple", 1);
        assertFalse(t.delete("app"));   // prefix but not a complete key
        assertFalse(t.delete("banana"));
        assertEquals(1, t.size());
    }

    @Test
    void deleteThenInsertSameKey() {
        Trie<Integer> t = new Trie<>();
        t.insert("apple", 1);
        t.delete("apple");
        t.insert("apple", 99);
        assertEquals(Optional.of(99), t.get("apple"));
        assertEquals(1, t.size());
    }

    @Test
    void deleteDoesNotRemoveSharedPrefix() {
        Trie<Integer> t = new Trie<>();
        t.insert("apple", 1);
        t.insert("apply", 2);
        t.insert("app", 3);
        t.delete("apple");
        // "apply" and "app" must survive
        assertTrue(t.contains("apply"));
        assertTrue(t.contains("app"));
        assertFalse(t.contains("apple"));
        assertEquals(2, t.size());
    }

    // =========================================================================
    // 5. keysWithPrefix
    // =========================================================================

    @Test
    void keysWithPrefixBasic() {
        Trie<Integer> t = new Trie<>();
        t.insert("app", 1);
        t.insert("apple", 2);
        t.insert("apply", 3);
        t.insert("apt", 4);
        t.insert("banana", 5);

        List<String> results = t.keysWithPrefix("app");
        assertEquals(3, results.size());
        assertTrue(results.contains("app"));
        assertTrue(results.contains("apple"));
        assertTrue(results.contains("apply"));
        assertFalse(results.contains("apt"));
        assertFalse(results.contains("banana"));
    }

    @Test
    void keysWithPrefixEmpty() {
        Trie<Integer> t = new Trie<>();
        t.insert("app", 1);
        t.insert("banana", 2);
        List<String> all = t.keysWithPrefix("");
        assertEquals(2, all.size());
        assertTrue(all.contains("app"));
        assertTrue(all.contains("banana"));
    }

    @Test
    void keysWithPrefixNoMatch() {
        Trie<Integer> t = new Trie<>();
        t.insert("apple", 1);
        assertTrue(t.keysWithPrefix("z").isEmpty());
    }

    @Test
    void keysWithPrefixExactMatch() {
        Trie<Integer> t = new Trie<>();
        t.insert("apple", 1);
        List<String> results = t.keysWithPrefix("apple");
        assertEquals(List.of("apple"), results);
    }

    // =========================================================================
    // 6. keys()
    // =========================================================================

    @Test
    void keysAll() {
        Trie<Integer> t = new Trie<>();
        t.insert("cat", 1);
        t.insert("car", 2);
        t.insert("dog", 3);
        List<String> all = t.keys();
        assertEquals(3, all.size());
        assertTrue(all.contains("cat"));
        assertTrue(all.contains("car"));
        assertTrue(all.contains("dog"));
    }

    // =========================================================================
    // 7. size / isEmpty
    // =========================================================================

    @Test
    void sizeEmpty() {
        assertEquals(0, new Trie<>().size());
        assertTrue(new Trie<>().isEmpty());
    }

    @Test
    void sizeAfterInserts() {
        Trie<Integer> t = new Trie<>();
        t.insert("a", 1);
        t.insert("ab", 2);
        t.insert("abc", 3);
        assertEquals(3, t.size());
        assertFalse(t.isEmpty());
    }

    @Test
    void sizeAfterDeleteAll() {
        Trie<Integer> t = new Trie<>();
        t.insert("a", 1);
        t.insert("b", 2);
        t.delete("a");
        t.delete("b");
        assertEquals(0, t.size());
        assertTrue(t.isEmpty());
    }

    // =========================================================================
    // 8. Unicode / special characters
    // =========================================================================

    @Test
    void unicodeKeys() {
        Trie<Integer> t = new Trie<>();
        t.insert("café", 1);
        t.insert("cafés", 2);
        assertTrue(t.contains("café"));
        assertTrue(t.startsWith("caf"));
        assertEquals(2, t.size());
    }

    @Test
    void singleCharKeys() {
        Trie<Integer> t = new Trie<>();
        for (char c = 'a'; c <= 'z'; c++) {
            t.insert(String.valueOf(c), (int) c);
        }
        assertEquals(26, t.size());
        for (char c = 'a'; c <= 'z'; c++) {
            assertEquals(Optional.of((int) c), t.get(String.valueOf(c)));
        }
    }

    // =========================================================================
    // 9. Large dataset smoke test
    // =========================================================================

    @Test
    void largeDataset() {
        Trie<Integer> t = new Trie<>();
        int n = 1000;
        for (int i = 0; i < n; i++) {
            t.insert("key" + i, i);
        }
        assertEquals(n, t.size());
        for (int i = 0; i < n; i++) {
            assertEquals(Optional.of(i), t.get("key" + i));
        }
        // All keys start with "key"
        assertEquals(n, t.keysWithPrefix("key").size());
    }
}
