package com.codingadventures.skiplist;

import java.util.AbstractMap;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.Objects;

public final class SkipList<K, V> implements Iterable<K> {
    @FunctionalInterface
    public interface Comparator<T> {
        int compare(T left, T right);
    }

    private final Comparator<K> comparator;
    private final int maxLevel;
    private final double probability;
    private final ArrayList<Map.Entry<K, V>> items = new ArrayList<>();

    public SkipList() {
        this(null, 32, 0.5);
    }

    public SkipList(Comparator<K> comparator) {
        this(comparator, 32, 0.5);
    }

    public SkipList(Comparator<K> comparator, int maxLevel, double probability) {
        this.comparator = comparator == null ? SkipList::defaultCompare : comparator;
        this.maxLevel = Math.max(1, maxLevel);
        this.probability = Double.isFinite(probability) && probability > 0 && probability < 1 ? probability : 0.5;
    }

    public static <K, V> SkipList<K, V> withParams(int maxLevel, double probability, Comparator<K> comparator) {
        return new SkipList<>(comparator, maxLevel, probability);
    }

    public void insert(K key, V value) {
        int index = findInsertIndex(key);
        if (index < items.size() && comparator.compare(items.get(index).getKey(), key) == 0) {
            items.set(index, entry(key, value));
            return;
        }
        items.add(index, entry(key, value));
    }

    public boolean delete(K key) {
        int index = findIndex(key);
        if (index < 0) {
            return false;
        }
        items.remove(index);
        return true;
    }

    public V search(K key) {
        int index = findIndex(key);
        return index < 0 ? null : items.get(index).getValue();
    }

    public boolean contains(K key) {
        return findIndex(key) >= 0;
    }

    public boolean containsKey(K key) {
        return contains(key);
    }

    public Integer rank(K key) {
        int index = findIndex(key);
        return index < 0 ? null : index;
    }

    public K byRank(int rank) {
        if (rank < 0 || rank >= items.size()) {
            return null;
        }
        return items.get(rank).getKey();
    }

    public List<Map.Entry<K, V>> rangeQuery(K low, K high, boolean inclusive) {
        return range(low, high, inclusive);
    }

    public List<Map.Entry<K, V>> range(K low, K high, boolean inclusive) {
        if (comparator.compare(low, high) > 0) {
            return List.of();
        }
        ArrayList<Map.Entry<K, V>> result = new ArrayList<>();
        for (Map.Entry<K, V> item : items) {
            int lower = comparator.compare(item.getKey(), low);
            int upper = comparator.compare(item.getKey(), high);
            boolean lowerOk = inclusive ? lower >= 0 : lower > 0;
            boolean upperOk = inclusive ? upper <= 0 : upper < 0;
            if (lowerOk && upperOk) {
                result.add(entry(item.getKey(), item.getValue()));
            }
        }
        return result;
    }

    public List<K> toList() {
        ArrayList<K> result = new ArrayList<>(items.size());
        for (Map.Entry<K, V> item : items) {
            result.add(item.getKey());
        }
        return result;
    }

    public List<Map.Entry<K, V>> entriesList() {
        ArrayList<Map.Entry<K, V>> result = new ArrayList<>(items.size());
        for (Map.Entry<K, V> item : items) {
            result.add(entry(item.getKey(), item.getValue()));
        }
        return result;
    }

    public List<Map.Entry<K, V>> entries() {
        return entriesList();
    }

    public K min() {
        return items.isEmpty() ? null : items.getFirst().getKey();
    }

    public K max() {
        return items.isEmpty() ? null : items.getLast().getKey();
    }

    public int len() {
        return items.size();
    }

    public int size() {
        return len();
    }

    public boolean isEmpty() {
        return items.isEmpty();
    }

    public int maxLevel() {
        return maxLevel;
    }

    public double probability() {
        return probability;
    }

    public int currentMax() {
        if (items.isEmpty()) {
            return 1;
        }
        int levels = (int) Math.ceil(Math.log(items.size()) / Math.log(1.0 / probability));
        return Math.min(maxLevel, Math.max(1, levels));
    }

    @Override
    public Iterator<K> iterator() {
        return toList().iterator();
    }

    private int findIndex(K key) {
        int low = 0;
        int high = items.size() - 1;
        while (low <= high) {
            int mid = (low + high) >>> 1;
            int comparison = comparator.compare(items.get(mid).getKey(), key);
            if (comparison == 0) {
                return mid;
            }
            if (comparison < 0) {
                low = mid + 1;
            } else {
                high = mid - 1;
            }
        }
        return -1;
    }

    private int findInsertIndex(K key) {
        int low = 0;
        int high = items.size();
        while (low < high) {
            int mid = (low + high) >>> 1;
            if (comparator.compare(items.get(mid).getKey(), key) < 0) {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        return low;
    }

    private static <K, V> Map.Entry<K, V> entry(K key, V value) {
        return new AbstractMap.SimpleImmutableEntry<>(key, value);
    }

    @SuppressWarnings("unchecked")
    private static <T> int defaultCompare(T left, T right) {
        Objects.requireNonNull(left, "left");
        Objects.requireNonNull(right, "right");
        return ((Comparable<? super T>) left).compareTo(right);
    }
}
