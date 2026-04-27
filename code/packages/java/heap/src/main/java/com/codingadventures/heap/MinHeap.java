package com.codingadventures.heap;

import java.util.ArrayList;
import java.util.List;

public final class MinHeap<T extends Comparable<? super T>> {
    private final List<T> values = new ArrayList<>();

    public int size() {
        return values.size();
    }

    public boolean isEmpty() {
        return values.isEmpty();
    }

    public void push(T value) {
        values.add(value);
        siftUp(values.size() - 1);
    }

    public T peek() {
        return values.isEmpty() ? null : values.getFirst();
    }

    public T pop() {
        if (values.isEmpty()) {
            return null;
        }
        T result = values.getFirst();
        T last = values.remove(values.size() - 1);
        if (!values.isEmpty()) {
            values.set(0, last);
            siftDown(0);
        }
        return result;
    }

    private void siftUp(int index) {
        while (index > 0) {
            int parent = (index - 1) / 2;
            if (values.get(parent).compareTo(values.get(index)) <= 0) {
                return;
            }
            swap(parent, index);
            index = parent;
        }
    }

    private void siftDown(int index) {
        int size = values.size();
        while (true) {
            int left = index * 2 + 1;
            int right = left + 1;
            int smallest = index;
            if (left < size && values.get(left).compareTo(values.get(smallest)) < 0) {
                smallest = left;
            }
            if (right < size && values.get(right).compareTo(values.get(smallest)) < 0) {
                smallest = right;
            }
            if (smallest == index) {
                return;
            }
            swap(index, smallest);
            index = smallest;
        }
    }

    private void swap(int left, int right) {
        T temp = values.get(left);
        values.set(left, values.get(right));
        values.set(right, temp);
    }
}
