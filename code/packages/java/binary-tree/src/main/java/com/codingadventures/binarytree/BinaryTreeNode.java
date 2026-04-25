package com.codingadventures.binarytree;

public record BinaryTreeNode<T>(T value, BinaryTreeNode<T> left, BinaryTreeNode<T> right) {
    public BinaryTreeNode(T value) {
        this(value, null, null);
    }
}
