package com.codingadventures.heap;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class MinHeapTest {
    @Test
    void popsValuesInSortedOrder() {
        MinHeap<Integer> heap = new MinHeap<>();
        heap.push(5);
        heap.push(1);
        heap.push(3);

        assertEquals(1, heap.pop());
        assertEquals(3, heap.pop());
        assertEquals(5, heap.pop());
        assertNull(heap.pop());
    }
}
