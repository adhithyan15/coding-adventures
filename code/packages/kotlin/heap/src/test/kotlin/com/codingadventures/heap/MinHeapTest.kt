package com.codingadventures.heap

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class MinHeapTest {
    @Test
    fun popsValuesInSortedOrder() {
        val heap = MinHeap<Int>()
        heap.push(5)
        heap.push(1)
        heap.push(3)

        assertEquals(1, heap.pop())
        assertEquals(3, heap.pop())
        assertEquals(5, heap.pop())
        assertNull(heap.pop())
    }
}
