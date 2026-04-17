package com.codingadventures.radixtree;

import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class RadixTreeTest {
    @Test
    void insertAndSearchCoverSplitCases() {
        RadixTree<Integer> tree = new RadixTree<>();
        tree.insert("application", 1);
        tree.insert("apple", 2);
        tree.insert("app", 3);
        tree.insert("apt", 4);
        assertEquals(1, tree.search("application"));
        assertEquals(2, tree.search("apple"));
        assertEquals(3, tree.search("app"));
        assertEquals(4, tree.search("apt"));
        assertNull(tree.search("appl"));
    }

    @Test
    void deletePrunesAndMerges() {
        RadixTree<Integer> tree = new RadixTree<>();
        tree.insert("app", 1);
        tree.insert("apple", 2);
        assertEquals(3, tree.nodeCount());
        assertTrue(tree.delete("app"));
        assertNull(tree.search("app"));
        assertEquals(2, tree.search("apple"));
        assertEquals(2, tree.nodeCount());
    }

    @Test
    void supportsPrefixQueriesAndMatches() {
        RadixTree<Integer> tree = new RadixTree<>();
        tree.insert("search", 1);
        tree.insert("searcher", 2);
        tree.insert("searching", 3);
        tree.insert("banana", 4);

        assertTrue(tree.startsWith("sear"));
        assertFalse(tree.startsWith("seek"));
        assertEquals(List.of("search", "searcher", "searching"), tree.wordsWithPrefix("search"));
        assertEquals("search", tree.longestPrefixMatch("search-party"));
        assertNull(tree.longestPrefixMatch("xyz"));
    }

    @Test
    void supportsEmptyStringAndSortedKeys() {
        RadixTree<Integer> tree = new RadixTree<>();
        tree.insert("", 1);
        tree.insert("banana", 2);
        tree.insert("apple", 3);
        tree.insert("apricot", 4);
        tree.insert("app", 5);

        assertEquals(1, tree.search(""));
        assertEquals("", tree.longestPrefixMatch("xyz"));
        assertTrue(tree.delete(""));
        assertNull(tree.search(""));
        assertEquals(List.of("app", "apple", "apricot", "banana"), tree.keys());
        assertEquals(4, tree.len());
        assertFalse(tree.isEmpty());
    }
}
