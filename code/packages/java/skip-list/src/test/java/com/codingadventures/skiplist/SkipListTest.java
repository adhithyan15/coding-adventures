package com.codingadventures.skiplist;

import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

class SkipListTest {
    @Test
    void insertsSearchesAndDeletesInSortedOrder() {
        SkipList<Integer, String> list = new SkipList<>();
        list.insert(20, "b");
        list.insert(10, "a");
        list.insert(30, "c");

        assertEquals(List.of(10, 20, 30), list.toList());
        assertEquals("b", list.search(20));
        assertTrue(list.delete(20));
        assertNull(list.search(20));
        assertEquals(List.of(10, 30), list.toList());
    }

    @Test
    void computesRankAndByRankConsistently() {
        SkipList<Integer, String> list = new SkipList<>();
        for (int key : List.of(50, 10, 30, 20)) {
            list.insert(key, Integer.toString(key));
        }

        assertEquals(0, list.rank(10));
        assertEquals(1, list.rank(20));
        assertEquals(30, list.byRank(2));

        SkipList<String, Integer> strings = new SkipList<>();
        strings.insert("alpha", 1);
        assertNull(strings.byRank(10));
    }

    @Test
    void returnsBoundedRanges() {
        SkipList<Integer, String> list = new SkipList<>();
        for (int key : List.of(10, 20, 30, 40, 50)) {
            list.insert(key, Integer.toString(key));
        }

        assertEquals(
            List.of(Map.entry(20, "20"), Map.entry(30, "30"), Map.entry(40, "40")),
            list.range(15, 45, true)
        );
        assertEquals(
            List.of(Map.entry(20, "20"), Map.entry(30, "30")),
            list.range(10, 40, false)
        );
    }

    @Test
    void supportsCustomComparatorsForCompositeKeys() {
        SkipList.Comparator<ScoreMember> comparator = (left, right) -> {
            int byScore = Double.compare(left.score(), right.score());
            return byScore != 0 ? byScore : left.member().compareTo(right.member());
        };

        SkipList<ScoreMember, String> list = new SkipList<>(comparator);
        list.insert(new ScoreMember(10, "b"), "b");
        list.insert(new ScoreMember(10, "a"), "a");
        list.insert(new ScoreMember(5, "z"), "z");

        assertEquals(
            List.of(new ScoreMember(5, "z"), new ScoreMember(10, "a"), new ScoreMember(10, "b")),
            list.toList()
        );
    }

    @Test
    void exposesHelpersAndBoundaryBehaviors() {
        SkipList<Integer, String> list = SkipList.withParams(-1, 2, null);
        SkipList<String, Integer> emptyStrings = new SkipList<>();
        assertTrue(list.isEmpty());
        assertEquals(1, list.maxLevel());
        assertEquals(0.5, list.probability());
        assertEquals(1, list.currentMax());
        assertNull(emptyStrings.min());
        assertNull(emptyStrings.max());
        assertEquals(List.of(), list.rangeQuery(10, 1, true));

        list.insert(1, "one");
        list.insert(2, "two");
        list.insert(2, "two-updated");
        list.insert(3, "three");

        assertTrue(list.contains(2));
        assertTrue(list.containsKey(3));
        assertFalse(list.contains(99));
        assertEquals("two-updated", list.search(2));
        assertEquals(3, list.len());
        assertEquals(3, list.size());
        assertEquals(1, list.min());
        assertEquals(3, list.max());
        assertEquals(1, list.byRank(0));
        assertNull(list.rank(99));
        assertTrue(list.currentMax() >= 1);
        assertEquals(
            List.of(Map.entry(1, "one"), Map.entry(2, "two-updated"), Map.entry(3, "three")),
            list.entries()
        );
        assertTrue(list.delete(3));
        assertFalse(list.delete(3));
    }

    private record ScoreMember(double score, String member) {
    }
}
