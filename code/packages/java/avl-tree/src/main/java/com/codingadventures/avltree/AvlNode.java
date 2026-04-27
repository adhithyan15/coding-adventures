package com.codingadventures.avltree;

public record AvlNode<T>(T value, AvlNode<T> left, AvlNode<T> right, int height, int size) {
    public AvlNode(T value) {
        this(value, null, null, 0, 1);
    }

    public AvlNode(T value, AvlNode<T> left, AvlNode<T> right) {
        this(value, left, right, 1 + Math.max(height(left), height(right)), 1 + size(left) + size(right));
    }

    public static <T> int height(AvlNode<T> node) {
        return node == null ? -1 : node.height();
    }

    public static <T> int size(AvlNode<T> node) {
        return node == null ? 0 : node.size();
    }
}
