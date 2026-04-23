package com.codingadventures.skiplist

class SkipList<K, V>(
    private val comparator: ((K, K) -> Int)? = null,
    private val maxLevel: Int = 32,
    private val probability: Double = 0.5,
) : Iterable<K> {
    private val compare: (K, K) -> Int = comparator ?: { left, right ->
        @Suppress("UNCHECKED_CAST")
        (left as Comparable<K>).compareTo(right)
    }
    private val boundedMaxLevel = maxLevel.coerceAtLeast(1)
    private val boundedProbability = probability.takeIf { it.isFinite() && it > 0.0 && it < 1.0 } ?: 0.5
    private val items = mutableListOf<Pair<K, V>>()

    fun insert(key: K, value: V) {
        val index = findInsertIndex(key)
        if (index < items.size && compare(items[index].first, key) == 0) {
            items[index] = key to value
            return
        }
        items.add(index, key to value)
    }

    fun delete(key: K): Boolean {
        val index = findIndex(key)
        if (index < 0) return false
        items.removeAt(index)
        return true
    }

    fun search(key: K): V? = findIndex(key).takeIf { it >= 0 }?.let { items[it].second }
    fun contains(key: K): Boolean = findIndex(key) >= 0
    fun containsKey(key: K): Boolean = contains(key)
    fun rank(key: K): Int? = findIndex(key).takeIf { it >= 0 }
    fun byRank(rank: Int): K? = items.getOrNull(rank)?.first
    fun rangeQuery(low: K, high: K, inclusive: Boolean): List<Pair<K, V>> = range(low, high, inclusive)

    fun range(low: K, high: K, inclusive: Boolean): List<Pair<K, V>> {
        if (compare(low, high) > 0) return emptyList()
        return items.filter { (key, _) ->
            val lower = compare(key, low)
            val upper = compare(key, high)
            val lowerOk = if (inclusive) lower >= 0 else lower > 0
            val upperOk = if (inclusive) upper <= 0 else upper < 0
            lowerOk && upperOk
        }
    }

    fun toList(): List<K> = items.map { it.first }
    fun entriesList(): List<Pair<K, V>> = items.toList()
    fun entries(): List<Pair<K, V>> = entriesList()
    fun min(): K? = items.firstOrNull()?.first
    fun max(): K? = items.lastOrNull()?.first
    fun len(): Int = items.size
    fun size(): Int = len()
    fun isEmpty(): Boolean = items.isEmpty()
    fun maxLevel(): Int = boundedMaxLevel
    fun probability(): Double = boundedProbability

    fun currentMax(): Int {
        if (items.isEmpty()) return 1
        val levels = kotlin.math.ceil(kotlin.math.ln(items.size.toDouble()) / kotlin.math.ln(1.0 / boundedProbability)).toInt()
        return levels.coerceAtLeast(1).coerceAtMost(boundedMaxLevel)
    }

    override fun iterator(): Iterator<K> = toList().iterator()

    companion object {
        fun <K, V> withParams(maxLevel: Int = 32, probability: Double = 0.5, comparator: ((K, K) -> Int)? = null): SkipList<K, V> =
            SkipList(comparator, maxLevel, probability)
    }

    private fun findIndex(key: K): Int {
        var low = 0
        var high = items.lastIndex
        while (low <= high) {
            val mid = (low + high) ushr 1
            when (val comparison = compare(items[mid].first, key)) {
                0 -> return mid
                in Int.MIN_VALUE..-1 -> low = mid + 1
                else -> high = mid - 1
            }
        }
        return -1
    }

    private fun findInsertIndex(key: K): Int {
        var low = 0
        var high = items.size
        while (low < high) {
            val mid = (low + high) ushr 1
            if (compare(items[mid].first, key) < 0) low = mid + 1 else high = mid
        }
        return low
    }
}
