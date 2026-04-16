package com.codingadventures.hashmap

class HashMap<K, V>(values: Map<K, V> = emptyMap()) {
    private val delegate = linkedMapOf<K, V>().apply { putAll(values) }

    val size: Int get() = delegate.size
    fun isEmpty(): Boolean = delegate.isEmpty()
    fun has(key: K): Boolean = delegate.containsKey(key)
    operator fun get(key: K): V? = delegate[key]
    fun getOrDefault(key: K, defaultValue: V): V = delegate.getOrDefault(key, defaultValue)
    fun set(key: K, value: V): HashMap<K, V> {
        delegate[key] = value
        return this
    }
    fun delete(key: K): Boolean = delegate.remove(key) != null
    fun clear() = delegate.clear()
    fun keys(): List<K> = delegate.keys.toList()
    fun values(): List<V> = delegate.values.toList()
    fun entries(): List<Pair<K, V>> = delegate.entries.map { it.key to it.value }
    fun copy(): HashMap<K, V> = HashMap(delegate)

    override fun equals(other: Any?): Boolean = other is HashMap<*, *> && delegate == other.delegate
    override fun hashCode(): Int = delegate.hashCode()
    override fun toString(): String = delegate.toString()
}
