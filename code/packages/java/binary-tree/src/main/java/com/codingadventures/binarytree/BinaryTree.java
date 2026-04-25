package com.codingadventures.binarytree;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.LinkedList;
import java.util.List;
import java.util.Objects;
import java.util.Queue;

public final class BinaryTree<T> {
    private final BinaryTreeNode<T> root;

    public BinaryTree() {
        this(null);
    }

    public BinaryTree(BinaryTreeNode<T> root) {
        this.root = root;
    }

    public static <T> BinaryTree<T> empty() {
        return new BinaryTree<>();
    }

    public static <T> BinaryTree<T> singleton(T value) {
        return new BinaryTree<>(new BinaryTreeNode<>(value));
    }

    public static <T> BinaryTree<T> withRoot(BinaryTreeNode<T> root) {
        return new BinaryTree<>(root);
    }

    public static <T> BinaryTree<T> fromLevelOrder(List<T> values) {
        Objects.requireNonNull(values, "values");
        return new BinaryTree<>(buildFromLevelOrder(values, 0));
    }

    public BinaryTreeNode<T> root() {
        return root;
    }

    public BinaryTreeNode<T> find(T value) {
        return find(root, value);
    }

    public BinaryTreeNode<T> leftChild(T value) {
        BinaryTreeNode<T> node = find(value);
        return node == null ? null : node.left();
    }

    public BinaryTreeNode<T> rightChild(T value) {
        BinaryTreeNode<T> node = find(value);
        return node == null ? null : node.right();
    }

    public boolean isFull() {
        return isFull(root);
    }

    public boolean isComplete() {
        return isComplete(root);
    }

    public boolean isPerfect() {
        return isPerfect(root);
    }

    public int height() {
        return height(root);
    }

    public int size() {
        return size(root);
    }

    public List<T> inorder() {
        return inorder(root);
    }

    public List<T> preorder() {
        return preorder(root);
    }

    public List<T> postorder() {
        return postorder(root);
    }

    public List<T> levelOrder() {
        return levelOrder(root);
    }

    public List<T> toArray() {
        return toArray(root);
    }

    public String toAscii() {
        return toAscii(root);
    }

    @Override
    public String toString() {
        String rootValue = root == null ? "null" : String.valueOf(root.value());
        return "BinaryTree(root=" + rootValue + ", size=" + size() + ")";
    }

    public static <T> BinaryTreeNode<T> find(BinaryTreeNode<T> root, T value) {
        if (root == null) {
            return null;
        }
        if (Objects.equals(root.value(), value)) {
            return root;
        }
        BinaryTreeNode<T> left = find(root.left(), value);
        return left != null ? left : find(root.right(), value);
    }

    public static <T> BinaryTreeNode<T> leftChild(BinaryTreeNode<T> root, T value) {
        BinaryTreeNode<T> node = find(root, value);
        return node == null ? null : node.left();
    }

    public static <T> BinaryTreeNode<T> rightChild(BinaryTreeNode<T> root, T value) {
        BinaryTreeNode<T> node = find(root, value);
        return node == null ? null : node.right();
    }

    public static <T> boolean isFull(BinaryTreeNode<T> root) {
        if (root == null) {
            return true;
        }
        if (root.left() == null && root.right() == null) {
            return true;
        }
        if (root.left() == null || root.right() == null) {
            return false;
        }
        return isFull(root.left()) && isFull(root.right());
    }

    public static <T> boolean isComplete(BinaryTreeNode<T> root) {
        Queue<BinaryTreeNode<T>> queue = new LinkedList<>();
        queue.add(root);
        boolean seenNull = false;

        while (!queue.isEmpty()) {
            BinaryTreeNode<T> node = queue.remove();
            if (node == null) {
                seenNull = true;
                continue;
            }
            if (seenNull) {
                return false;
            }
            queue.add(node.left());
            queue.add(node.right());
        }

        return true;
    }

    public static <T> boolean isPerfect(BinaryTreeNode<T> root) {
        int treeHeight = height(root);
        if (treeHeight < 0) {
            return size(root) == 0;
        }
        return size(root) == (1 << (treeHeight + 1)) - 1;
    }

    public static <T> int height(BinaryTreeNode<T> root) {
        return root == null ? -1 : 1 + Math.max(height(root.left()), height(root.right()));
    }

    public static <T> int size(BinaryTreeNode<T> root) {
        return root == null ? 0 : 1 + size(root.left()) + size(root.right());
    }

    public static <T> List<T> inorder(BinaryTreeNode<T> root) {
        List<T> output = new ArrayList<>();
        inorder(root, output);
        return output;
    }

    public static <T> List<T> preorder(BinaryTreeNode<T> root) {
        List<T> output = new ArrayList<>();
        preorder(root, output);
        return output;
    }

    public static <T> List<T> postorder(BinaryTreeNode<T> root) {
        List<T> output = new ArrayList<>();
        postorder(root, output);
        return output;
    }

    public static <T> List<T> levelOrder(BinaryTreeNode<T> root) {
        List<T> output = new ArrayList<>();
        if (root == null) {
            return output;
        }

        Queue<BinaryTreeNode<T>> queue = new ArrayDeque<>();
        queue.add(root);
        while (!queue.isEmpty()) {
            BinaryTreeNode<T> node = queue.remove();
            output.add(node.value());
            if (node.left() != null) {
                queue.add(node.left());
            }
            if (node.right() != null) {
                queue.add(node.right());
            }
        }

        return output;
    }

    public static <T> List<T> toArray(BinaryTreeNode<T> root) {
        int treeHeight = height(root);
        if (treeHeight < 0) {
            return List.of();
        }

        int length = (1 << (treeHeight + 1)) - 1;
        List<T> output = new ArrayList<>(length);
        for (int i = 0; i < length; i++) {
            output.add(null);
        }
        fillArray(root, 0, output);
        return output;
    }

    public static <T> String toAscii(BinaryTreeNode<T> root) {
        if (root == null) {
            return "";
        }

        StringBuilder builder = new StringBuilder();
        renderAscii(root, "", true, builder);
        return builder.toString().stripTrailing();
    }

    private static <T> BinaryTreeNode<T> buildFromLevelOrder(List<T> values, int index) {
        if (index >= values.size()) {
            return null;
        }
        T value = values.get(index);
        if (value == null) {
            return null;
        }
        return new BinaryTreeNode<>(
                value,
                buildFromLevelOrder(values, (2 * index) + 1),
                buildFromLevelOrder(values, (2 * index) + 2));
    }

    private static <T> void inorder(BinaryTreeNode<T> root, List<T> output) {
        if (root == null) {
            return;
        }
        inorder(root.left(), output);
        output.add(root.value());
        inorder(root.right(), output);
    }

    private static <T> void preorder(BinaryTreeNode<T> root, List<T> output) {
        if (root == null) {
            return;
        }
        output.add(root.value());
        preorder(root.left(), output);
        preorder(root.right(), output);
    }

    private static <T> void postorder(BinaryTreeNode<T> root, List<T> output) {
        if (root == null) {
            return;
        }
        postorder(root.left(), output);
        postorder(root.right(), output);
        output.add(root.value());
    }

    private static <T> void fillArray(BinaryTreeNode<T> root, int index, List<T> output) {
        if (root == null || index >= output.size()) {
            return;
        }
        output.set(index, root.value());
        fillArray(root.left(), (2 * index) + 1, output);
        fillArray(root.right(), (2 * index) + 2, output);
    }

    private static <T> void renderAscii(BinaryTreeNode<T> node, String prefix, boolean isTail, StringBuilder output) {
        output.append(prefix)
                .append(isTail ? "`-- " : "|-- ")
                .append(node.value())
                .append(System.lineSeparator());

        List<BinaryTreeNode<T>> children = new ArrayList<>(2);
        if (node.left() != null) {
            children.add(node.left());
        }
        if (node.right() != null) {
            children.add(node.right());
        }

        String nextPrefix = prefix + (isTail ? "    " : "|   ");
        for (int i = 0; i < children.size(); i++) {
            renderAscii(children.get(i), nextPrefix, i + 1 == children.size(), output);
        }
    }
}
