package com.codingadventures.hashset;

import com.codingadventures.hashmap.HashMap;

import java.util.List;
import java.util.Objects;

public final class HashSet<T> {
    private final HashMap<T, Boolean> map;

    public HashSet() {
        this.map = new HashMap<>();
    }

    private HashSet(HashMap<T, Boolean> map) {
        this.map = map;
    }

    public int size() {
        return map.size();
    }

    public boolean isEmpty() {
        return map.isEmpty();
    }

    public boolean contains(T value) {
        return map.has(value);
    }

    public HashSet<T> add(T value) {
        map.set(value, Boolean.TRUE);
        return this;
    }

    public boolean remove(T value) {
        return map.delete(value);
    }

    public List<T> items() {
        return map.keys();
    }

    public HashSet<T> union(HashSet<T> other) {
        HashSet<T> result = copy();
        for (T value : other.items()) {
            result.add(value);
        }
        return result;
    }

    public HashSet<T> intersection(HashSet<T> other) {
        HashSet<T> result = new HashSet<>();
        for (T value : items()) {
            if (other.contains(value)) {
                result.add(value);
            }
        }
        return result;
    }

    public HashSet<T> difference(HashSet<T> other) {
        HashSet<T> result = new HashSet<>();
        for (T value : items()) {
            if (!other.contains(value)) {
                result.add(value);
            }
        }
        return result;
    }

    public HashSet<T> copy() {
        return new HashSet<>(map.copy());
    }

    @Override
    public boolean equals(Object other) {
        if (!(other instanceof HashSet<?> that)) {
            return false;
        }
        return Objects.equals(map, that.map);
    }

    @Override
    public int hashCode() {
        return map.hashCode();
    }

    @Override
    public String toString() {
        return items().toString();
    }
}
