package com.codingadventures.binarysearchtree;

import java.util.ArrayList;
import java.util.List;
import java.util.Objects;

public final class BinarySearchTree<T extends Comparable<? super T>> {
    private final BstNode<T> root;

    public BinarySearchTree() {
        this(null);
    }

    public BinarySearchTree(BstNode<T> root) {
        this.root = root;
    }

    public static <T extends Comparable<? super T>> BinarySearchTree<T> empty() {
        return new BinarySearchTree<>();
    }

    public static <T extends Comparable<? super T>> BinarySearchTree<T> fromSortedArray(List<T> values) {
        Objects.requireNonNull(values, "values");
        return new BinarySearchTree<>(buildBalanced(values, 0, values.size()));
    }

    public BstNode<T> root() {
        return root;
    }

    public BinarySearchTree<T> insert(T value) {
        return new BinarySearchTree<>(insertNode(root, value));
    }

    public BinarySearchTree<T> delete(T value) {
        return new BinarySearchTree<>(deleteNode(root, value));
    }

    public BstNode<T> search(T value) {
        return searchNode(root, value);
    }

    public boolean contains(T value) {
        return search(value) != null;
    }

    public T minValue() {
        return minValue(root);
    }

    public T maxValue() {
        return maxValue(root);
    }

    public T predecessor(T value) {
        return predecessor(root, value);
    }

    public T successor(T value) {
        return successor(root, value);
    }

    public T kthSmallest(int k) {
        return kthSmallest(root, k);
    }

    public int rank(T value) {
        return rank(root, value);
    }

    public List<T> toSortedArray() {
        return toSortedArray(root);
    }

    public boolean isValid() {
        return isValid(root);
    }

    public int height() {
        return height(root);
    }

    public int size() {
        return BstNode.size(root);
    }

    @Override
    public String toString() {
        String rootValue = root == null ? "null" : String.valueOf(root.value());
        return "BinarySearchTree(root=" + rootValue + ", size=" + size() + ")";
    }

    public static <T extends Comparable<? super T>> BstNode<T> searchNode(BstNode<T> root, T value) {
        BstNode<T> current = root;
        while (current != null) {
            int comparison = value.compareTo(current.value());
            if (comparison < 0) {
                current = current.left();
            } else if (comparison > 0) {
                current = current.right();
            } else {
                return current;
            }
        }
        return null;
    }

    public static <T extends Comparable<? super T>> BstNode<T> insertNode(BstNode<T> root, T value) {
        if (root == null) {
            return new BstNode<>(value);
        }

        int comparison = value.compareTo(root.value());
        if (comparison < 0) {
            return new BstNode<>(root.value(), insertNode(root.left(), value), root.right());
        }
        if (comparison > 0) {
            return new BstNode<>(root.value(), root.left(), insertNode(root.right(), value));
        }
        return root;
    }

    public static <T extends Comparable<? super T>> BstNode<T> deleteNode(BstNode<T> root, T value) {
        if (root == null) {
            return null;
        }

        int comparison = value.compareTo(root.value());
        if (comparison < 0) {
            return new BstNode<>(root.value(), deleteNode(root.left(), value), root.right());
        }
        if (comparison > 0) {
            return new BstNode<>(root.value(), root.left(), deleteNode(root.right(), value));
        }

        if (root.left() == null) {
            return root.right();
        }
        if (root.right() == null) {
            return root.left();
        }

        Extracted<T> extracted = extractMin(root.right());
        return new BstNode<>(extracted.minimum(), root.left(), extracted.root());
    }

    public static <T extends Comparable<? super T>> T minValue(BstNode<T> root) {
        BstNode<T> current = root;
        while (current != null && current.left() != null) {
            current = current.left();
        }
        return current == null ? null : current.value();
    }

    public static <T extends Comparable<? super T>> T maxValue(BstNode<T> root) {
        BstNode<T> current = root;
        while (current != null && current.right() != null) {
            current = current.right();
        }
        return current == null ? null : current.value();
    }

    public static <T extends Comparable<? super T>> T predecessor(BstNode<T> root, T value) {
        BstNode<T> current = root;
        T best = null;
        while (current != null) {
            if (value.compareTo(current.value()) <= 0) {
                current = current.left();
            } else {
                best = current.value();
                current = current.right();
            }
        }
        return best;
    }

    public static <T extends Comparable<? super T>> T successor(BstNode<T> root, T value) {
        BstNode<T> current = root;
        T best = null;
        while (current != null) {
            if (value.compareTo(current.value()) >= 0) {
                current = current.right();
            } else {
                best = current.value();
                current = current.left();
            }
        }
        return best;
    }

    public static <T extends Comparable<? super T>> T kthSmallest(BstNode<T> root, int k) {
        if (root == null || k <= 0) {
            return null;
        }

        int leftSize = BstNode.size(root.left());
        if (k == leftSize + 1) {
            return root.value();
        }
        if (k <= leftSize) {
            return kthSmallest(root.left(), k);
        }
        return kthSmallest(root.right(), k - leftSize - 1);
    }

    public static <T extends Comparable<? super T>> int rank(BstNode<T> root, T value) {
        if (root == null) {
            return 0;
        }

        int comparison = value.compareTo(root.value());
        if (comparison < 0) {
            return rank(root.left(), value);
        }
        if (comparison > 0) {
            return BstNode.size(root.left()) + 1 + rank(root.right(), value);
        }
        return BstNode.size(root.left());
    }

    public static <T extends Comparable<? super T>> List<T> toSortedArray(BstNode<T> root) {
        List<T> output = new ArrayList<>();
        inOrder(root, output);
        return output;
    }

    public static <T extends Comparable<? super T>> boolean isValid(BstNode<T> root) {
        return validate(root, null, null) != null;
    }

    public static <T> int height(BstNode<T> root) {
        return root == null ? -1 : 1 + Math.max(height(root.left()), height(root.right()));
    }

    private static <T extends Comparable<? super T>> Extracted<T> extractMin(BstNode<T> root) {
        if (root.left() == null) {
            return new Extracted<>(root.right(), root.value());
        }

        Extracted<T> extracted = extractMin(root.left());
        return new Extracted<>(
                new BstNode<>(root.value(), extracted.root(), root.right()),
                extracted.minimum());
    }

    private static <T extends Comparable<? super T>> BstNode<T> buildBalanced(List<T> values, int start, int end) {
        if (start >= end) {
            return null;
        }
        int mid = start + (end - start) / 2;
        return new BstNode<>(
                values.get(mid),
                buildBalanced(values, start, mid),
                buildBalanced(values, mid + 1, end));
    }

    private static <T extends Comparable<? super T>> void inOrder(BstNode<T> root, List<T> output) {
        if (root == null) {
            return;
        }
        inOrder(root.left(), output);
        output.add(root.value());
        inOrder(root.right(), output);
    }

    private static <T extends Comparable<? super T>> Validation validate(BstNode<T> root, T minimum, T maximum) {
        if (root == null) {
            return new Validation(-1, 0);
        }
        if (minimum != null && root.value().compareTo(minimum) <= 0) {
            return null;
        }
        if (maximum != null && root.value().compareTo(maximum) >= 0) {
            return null;
        }

        Validation left = validate(root.left(), minimum, root.value());
        Validation right = validate(root.right(), root.value(), maximum);
        if (left == null || right == null) {
            return null;
        }

        int expectedSize = 1 + left.size() + right.size();
        if (root.size() != expectedSize) {
            return null;
        }
        return new Validation(1 + Math.max(left.height(), right.height()), expectedSize);
    }

    private record Extracted<T>(BstNode<T> root, T minimum) {
    }

    private record Validation(int height, int size) {
    }
}
