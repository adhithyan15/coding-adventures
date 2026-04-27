package com.codingadventures.heap

class MinHeap<T : Comparable<T>> {
    private val values = mutableListOf<T>()

    val size: Int get() = values.size
    fun isEmpty(): Boolean = values.isEmpty()

    fun push(value: T) {
        values += value
        siftUp(values.lastIndex)
    }

    fun peek(): T? = values.firstOrNull()

    fun pop(): T? {
        if (values.isEmpty()) return null
        val result = values.first()
        val last = values.removeAt(values.lastIndex)
        if (values.isNotEmpty()) {
            values[0] = last
            siftDown(0)
        }
        return result
    }

    private fun siftUp(start: Int) {
        var index = start
        while (index > 0) {
            val parent = (index - 1) / 2
            if (values[parent] <= values[index]) return
            values[parent] = values[index].also { values[index] = values[parent] }
            index = parent
        }
    }

    private fun siftDown(start: Int) {
        var index = start
        while (true) {
            val left = index * 2 + 1
            val right = left + 1
            var smallest = index
            if (left < values.size && values[left] < values[smallest]) smallest = left
            if (right < values.size && values[right] < values[smallest]) smallest = right
            if (smallest == index) return
            values[index] = values[smallest].also { values[smallest] = values[index] }
            index = smallest
        }
    }
}
