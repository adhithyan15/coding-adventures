package com.codingadventures.binarysearchtree;

public record BstNode<T>(T value, BstNode<T> left, BstNode<T> right, int size) {
    public BstNode(T value) {
        this(value, null, null, 1);
    }

    public BstNode(T value, BstNode<T> left, BstNode<T> right) {
        this(value, left, right, 1 + size(left) + size(right));
    }

    public static <T> int size(BstNode<T> node) {
        return node == null ? 0 : node.size();
    }
}
