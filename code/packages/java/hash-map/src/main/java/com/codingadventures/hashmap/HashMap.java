package com.codingadventures.hashmap;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;

public final class HashMap<K, V> {
    private final LinkedHashMap<K, V> delegate;

    public HashMap() {
        this.delegate = new LinkedHashMap<>();
    }

    public HashMap(Map<K, V> values) {
        this.delegate = new LinkedHashMap<>(values);
    }

    public int size() {
        return delegate.size();
    }

    public boolean isEmpty() {
        return delegate.isEmpty();
    }

    public boolean has(K key) {
        return delegate.containsKey(key);
    }

    public V get(K key) {
        return delegate.get(key);
    }

    public V getOrDefault(K key, V defaultValue) {
        return delegate.getOrDefault(key, defaultValue);
    }

    public HashMap<K, V> set(K key, V value) {
        delegate.put(key, value);
        return this;
    }

    public boolean delete(K key) {
        return delegate.remove(key) != null;
    }

    public void clear() {
        delegate.clear();
    }

    public List<K> keys() {
        return new ArrayList<>(delegate.keySet());
    }

    public List<V> values() {
        return new ArrayList<>(delegate.values());
    }

    public List<Map.Entry<K, V>> entries() {
        List<Map.Entry<K, V>> entries = new ArrayList<>();
        for (Map.Entry<K, V> entry : delegate.entrySet()) {
            entries.add(Map.entry(entry.getKey(), entry.getValue()));
        }
        return entries;
    }

    public HashMap<K, V> copy() {
        return new HashMap<>(delegate);
    }

    @Override
    public boolean equals(Object other) {
        if (!(other instanceof HashMap<?, ?> that)) {
            return false;
        }
        return Objects.equals(delegate, that.delegate);
    }

    @Override
    public int hashCode() {
        return delegate.hashCode();
    }

    @Override
    public String toString() {
        return delegate.toString();
    }
}
