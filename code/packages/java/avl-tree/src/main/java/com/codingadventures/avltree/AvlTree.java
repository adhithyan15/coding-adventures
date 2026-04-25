package com.codingadventures.avltree;

import java.util.ArrayList;
import java.util.List;
import java.util.Objects;

public final class AvlTree<T extends Comparable<? super T>> {
    private final AvlNode<T> root;

    public AvlTree() {
        this(null);
    }

    public AvlTree(AvlNode<T> root) {
        this.root = root;
    }

    public static <T extends Comparable<? super T>> AvlTree<T> empty() {
        return new AvlTree<>();
    }

    public static <T extends Comparable<? super T>> AvlTree<T> fromValues(Iterable<T> values) {
        Objects.requireNonNull(values, "values");
        AvlTree<T> tree = empty();
        for (T value : values) {
            tree = tree.insert(value);
        }
        return tree;
    }

    public AvlNode<T> root() {
        return root;
    }

    public AvlTree<T> insert(T value) {
        return new AvlTree<>(insertNode(root, value));
    }

    public AvlTree<T> delete(T value) {
        return new AvlTree<>(deleteNode(root, value));
    }

    public AvlNode<T> search(T value) {
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

    public boolean isValidBst() {
        return isValidBst(root);
    }

    public boolean isValidAvl() {
        return isValidAvl(root);
    }

    public int balanceFactor(AvlNode<T> node) {
        return balanceFactorNode(node);
    }

    public int height() {
        return AvlNode.height(root);
    }

    public int size() {
        return AvlNode.size(root);
    }

    @Override
    public String toString() {
        String rootValue = root == null ? "null" : String.valueOf(root.value());
        return "AvlTree(root=" + rootValue + ", size=" + size() + ", height=" + height() + ")";
    }

    public static <T extends Comparable<? super T>> AvlNode<T> searchNode(AvlNode<T> root, T value) {
        AvlNode<T> current = root;
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

    public static <T extends Comparable<? super T>> AvlNode<T> insertNode(AvlNode<T> root, T value) {
        if (root == null) {
            return new AvlNode<>(value);
        }

        int comparison = value.compareTo(root.value());
        if (comparison < 0) {
            return rebalance(new AvlNode<>(root.value(), insertNode(root.left(), value), root.right()));
        }
        if (comparison > 0) {
            return rebalance(new AvlNode<>(root.value(), root.left(), insertNode(root.right(), value)));
        }
        return root;
    }

    public static <T extends Comparable<? super T>> AvlNode<T> deleteNode(AvlNode<T> root, T value) {
        if (root == null) {
            return null;
        }

        int comparison = value.compareTo(root.value());
        if (comparison < 0) {
            return rebalance(new AvlNode<>(root.value(), deleteNode(root.left(), value), root.right()));
        }
        if (comparison > 0) {
            return rebalance(new AvlNode<>(root.value(), root.left(), deleteNode(root.right(), value)));
        }

        if (root.left() == null) {
            return root.right();
        }
        if (root.right() == null) {
            return root.left();
        }

        Extracted<T> extracted = extractMin(root.right());
        return rebalance(new AvlNode<>(extracted.minimum(), root.left(), extracted.root()));
    }

    public static <T extends Comparable<? super T>> T minValue(AvlNode<T> root) {
        AvlNode<T> current = root;
        while (current != null && current.left() != null) {
            current = current.left();
        }
        return current == null ? null : current.value();
    }

    public static <T extends Comparable<? super T>> T maxValue(AvlNode<T> root) {
        AvlNode<T> current = root;
        while (current != null && current.right() != null) {
            current = current.right();
        }
        return current == null ? null : current.value();
    }

    public static <T extends Comparable<? super T>> T predecessor(AvlNode<T> root, T value) {
        AvlNode<T> current = root;
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

    public static <T extends Comparable<? super T>> T successor(AvlNode<T> root, T value) {
        AvlNode<T> current = root;
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

    public static <T extends Comparable<? super T>> T kthSmallest(AvlNode<T> root, int k) {
        if (root == null || k <= 0) {
            return null;
        }

        int leftSize = AvlNode.size(root.left());
        if (k == leftSize + 1) {
            return root.value();
        }
        if (k <= leftSize) {
            return kthSmallest(root.left(), k);
        }
        return kthSmallest(root.right(), k - leftSize - 1);
    }

    public static <T extends Comparable<? super T>> int rank(AvlNode<T> root, T value) {
        if (root == null) {
            return 0;
        }

        int comparison = value.compareTo(root.value());
        if (comparison < 0) {
            return rank(root.left(), value);
        }
        if (comparison > 0) {
            return AvlNode.size(root.left()) + 1 + rank(root.right(), value);
        }
        return AvlNode.size(root.left());
    }

    public static <T extends Comparable<? super T>> List<T> toSortedArray(AvlNode<T> root) {
        List<T> output = new ArrayList<>();
        inOrder(root, output);
        return output;
    }

    public static <T extends Comparable<? super T>> boolean isValidBst(AvlNode<T> root) {
        return validateBst(root, null, null);
    }

    public static <T extends Comparable<? super T>> boolean isValidAvl(AvlNode<T> root) {
        return validateAvl(root, null, null) != null;
    }

    public static <T> int balanceFactorNode(AvlNode<T> node) {
        return node == null ? 0 : AvlNode.height(node.left()) - AvlNode.height(node.right());
    }

    private static <T extends Comparable<? super T>> AvlNode<T> rebalance(AvlNode<T> node) {
        int balance = balanceFactorNode(node);

        if (balance > 1) {
            AvlNode<T> left = node.left();
            if (left != null && balanceFactorNode(left) < 0) {
                left = rotateLeft(left);
            }
            return rotateRight(new AvlNode<>(node.value(), left, node.right()));
        }

        if (balance < -1) {
            AvlNode<T> right = node.right();
            if (right != null && balanceFactorNode(right) > 0) {
                right = rotateRight(right);
            }
            return rotateLeft(new AvlNode<>(node.value(), node.left(), right));
        }

        return node;
    }

    private static <T extends Comparable<? super T>> AvlNode<T> rotateLeft(AvlNode<T> root) {
        AvlNode<T> right = root.right();
        if (right == null) {
            return root;
        }

        AvlNode<T> newLeft = new AvlNode<>(root.value(), root.left(), right.left());
        return new AvlNode<>(right.value(), newLeft, right.right());
    }

    private static <T extends Comparable<? super T>> AvlNode<T> rotateRight(AvlNode<T> root) {
        AvlNode<T> left = root.left();
        if (left == null) {
            return root;
        }

        AvlNode<T> newRight = new AvlNode<>(root.value(), left.right(), root.right());
        return new AvlNode<>(left.value(), left.left(), newRight);
    }

    private static <T extends Comparable<? super T>> Extracted<T> extractMin(AvlNode<T> root) {
        if (root.left() == null) {
            return new Extracted<>(root.right(), root.value());
        }

        Extracted<T> extracted = extractMin(root.left());
        return new Extracted<>(
                rebalance(new AvlNode<>(root.value(), extracted.root(), root.right())),
                extracted.minimum());
    }

    private static <T extends Comparable<? super T>> void inOrder(AvlNode<T> root, List<T> output) {
        if (root == null) {
            return;
        }
        inOrder(root.left(), output);
        output.add(root.value());
        inOrder(root.right(), output);
    }

    private static <T extends Comparable<? super T>> boolean validateBst(AvlNode<T> root, T minimum, T maximum) {
        if (root == null) {
            return true;
        }
        if (minimum != null && root.value().compareTo(minimum) <= 0) {
            return false;
        }
        if (maximum != null && root.value().compareTo(maximum) >= 0) {
            return false;
        }
        return validateBst(root.left(), minimum, root.value())
                && validateBst(root.right(), root.value(), maximum);
    }

    private static <T extends Comparable<? super T>> Validation validateAvl(AvlNode<T> root, T minimum, T maximum) {
        if (root == null) {
            return new Validation(-1, 0);
        }
        if (minimum != null && root.value().compareTo(minimum) <= 0) {
            return null;
        }
        if (maximum != null && root.value().compareTo(maximum) >= 0) {
            return null;
        }

        Validation left = validateAvl(root.left(), minimum, root.value());
        Validation right = validateAvl(root.right(), root.value(), maximum);
        if (left == null || right == null) {
            return null;
        }

        int expectedHeight = 1 + Math.max(left.height(), right.height());
        int expectedSize = 1 + left.size() + right.size();
        if (root.height() != expectedHeight
                || root.size() != expectedSize
                || Math.abs(left.height() - right.height()) > 1) {
            return null;
        }

        return new Validation(expectedHeight, expectedSize);
    }

    private record Extracted<T>(AvlNode<T> root, T minimum) {
    }

    private record Validation(int height, int size) {
    }
}
