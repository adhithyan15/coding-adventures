package com.codingadventures.hashset

import com.codingadventures.hashmap.HashMap

class HashSet<T>(private val map: HashMap<T, Boolean> = HashMap()) {
    val size: Int get() = map.size
    fun isEmpty(): Boolean = map.isEmpty()
    fun contains(value: T): Boolean = map.has(value)
    fun add(value: T): HashSet<T> {
        map.set(value, true)
        return this
    }
    fun remove(value: T): Boolean = map.delete(value)
    fun items(): List<T> = map.keys()
    fun union(other: HashSet<T>): HashSet<T> = copy().also { result -> other.items().forEach(result::add) }
    fun intersection(other: HashSet<T>): HashSet<T> = HashSet<T>().also { result -> items().filter(other::contains).forEach(result::add) }
    fun difference(other: HashSet<T>): HashSet<T> = HashSet<T>().also { result -> items().filterNot(other::contains).forEach(result::add) }
    fun copy(): HashSet<T> = HashSet(map.copy())
    override fun equals(other: Any?): Boolean = other is HashSet<*> && map == other.map
    override fun hashCode(): Int = map.hashCode()
    override fun toString(): String = items().toString()
}
