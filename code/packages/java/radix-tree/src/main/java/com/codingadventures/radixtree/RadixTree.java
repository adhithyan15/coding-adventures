package com.codingadventures.radixtree;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;

public final class RadixTree<V> {
    private final Node<V> root = new Node<>();
    private int size = 0;

    public void insert(String key, V value) {
        if (insertRecursive(root, key, value)) {
            size++;
        }
    }

    public V search(String key) {
        Node<V> node = root;
        String remaining = key;

        while (!remaining.isEmpty()) {
            Edge<V> edge = node.children.get(firstChar(remaining));
            if (edge == null) {
                return null;
            }
            int commonLength = commonPrefixLength(remaining, edge.label);
            if (commonLength < edge.label.length()) {
                return null;
            }
            remaining = remaining.substring(commonLength);
            node = edge.child;
        }

        return node.isEnd ? node.value : null;
    }

    public boolean containsKey(String key) {
        return search(key) != null || keyExists(key);
    }

    public boolean delete(String key) {
        DeleteResult result = deleteRecursive(root, key);
        if (result.deleted) {
            size--;
        }
        return result.deleted;
    }

    public boolean startsWith(String prefix) {
        if (prefix.isEmpty()) {
            return size > 0;
        }

        Node<V> node = root;
        String remaining = prefix;

        while (!remaining.isEmpty()) {
            Edge<V> edge = node.children.get(firstChar(remaining));
            if (edge == null) {
                return false;
            }
            int commonLength = commonPrefixLength(remaining, edge.label);
            if (commonLength == remaining.length()) {
                return true;
            }
            if (commonLength < edge.label.length()) {
                return false;
            }
            remaining = remaining.substring(commonLength);
            node = edge.child;
        }

        return node.isEnd || !node.children.isEmpty();
    }

    public List<String> wordsWithPrefix(String prefix) {
        Node<V> node = root;
        String remaining = prefix;
        StringBuilder path = new StringBuilder();

        if (remaining.isEmpty()) {
            ArrayList<String> results = new ArrayList<>();
            collectKeys(root, "", results);
            return results;
        }

        while (!remaining.isEmpty()) {
            Edge<V> edge = node.children.get(firstChar(remaining));
            if (edge == null) {
                return List.of();
            }
            int commonLength = commonPrefixLength(remaining, edge.label);
            if (commonLength == remaining.length()) {
                if (commonLength == edge.label.length()) {
                    path.append(edge.label);
                    node = edge.child;
                    remaining = "";
                } else {
                    ArrayList<String> results = new ArrayList<>();
                    collectKeys(edge.child, path + edge.label, results);
                    return results;
                }
            } else if (commonLength < edge.label.length()) {
                return List.of();
            } else {
                path.append(edge.label);
                remaining = remaining.substring(commonLength);
                node = edge.child;
            }
        }

        ArrayList<String> results = new ArrayList<>();
        collectKeys(node, path.toString(), results);
        return results;
    }

    public String longestPrefixMatch(String key) {
        Node<V> node = root;
        String remaining = key;
        int consumed = 0;
        String best = node.isEnd ? "" : null;

        while (!remaining.isEmpty()) {
            Edge<V> edge = node.children.get(firstChar(remaining));
            if (edge == null) {
                break;
            }
            int commonLength = commonPrefixLength(remaining, edge.label);
            if (commonLength < edge.label.length()) {
                break;
            }
            consumed += commonLength;
            remaining = remaining.substring(commonLength);
            node = edge.child;
            if (node.isEnd) {
                best = key.substring(0, consumed);
            }
        }

        return best;
    }

    public Map<String, V> toMap() {
        TreeMap<String, V> result = new TreeMap<>();
        collectValues(root, "", result);
        return result;
    }

    public List<String> keys() {
        ArrayList<String> results = new ArrayList<>();
        collectKeys(root, "", results);
        return results;
    }

    public int len() {
        return size;
    }

    public boolean isEmpty() {
        return size == 0;
    }

    public int nodeCount() {
        return countNodes(root);
    }

    @Override
    public String toString() {
        return "RadixTree(" + size + " keys: " + toMap().entrySet().stream().limit(5).toList() + ")";
    }

    private boolean keyExists(String key) {
        Node<V> node = root;
        String remaining = key;
        while (!remaining.isEmpty()) {
            Edge<V> edge = node.children.get(firstChar(remaining));
            if (edge == null) {
                return false;
            }
            int commonLength = commonPrefixLength(remaining, edge.label);
            if (commonLength < edge.label.length()) {
                return false;
            }
            remaining = remaining.substring(commonLength);
            node = edge.child;
        }
        return node.isEnd;
    }

    private static <V> boolean insertRecursive(Node<V> node, String key, V value) {
        if (key.isEmpty()) {
            boolean added = !node.isEnd;
            node.isEnd = true;
            node.value = value;
            return added;
        }

        char first = firstChar(key);
        Edge<V> edge = node.children.remove(first);
        if (edge == null) {
            node.children.put(first, new Edge<>(key, Node.leaf(value)));
            return true;
        }

        int commonLength = commonPrefixLength(key, edge.label);
        if (commonLength == edge.label.length()) {
            boolean added = insertRecursive(edge.child, key.substring(commonLength), value);
            node.children.put(first, edge);
            return added;
        }

        String common = edge.label.substring(0, commonLength);
        String labelRest = edge.label.substring(commonLength);
        String keyRest = key.substring(commonLength);
        Node<V> splitNode = new Node<>();
        splitNode.children.put(firstChar(labelRest), new Edge<>(labelRest, edge.child));

        if (keyRest.isEmpty()) {
            splitNode.isEnd = true;
            splitNode.value = value;
        } else {
            splitNode.children.put(firstChar(keyRest), new Edge<>(keyRest, Node.leaf(value)));
        }

        node.children.put(firstChar(common), new Edge<>(common, splitNode));
        return true;
    }

    private static <V> DeleteResult deleteRecursive(Node<V> node, String key) {
        if (key.isEmpty()) {
            if (!node.isEnd) {
                return new DeleteResult(false, false);
            }
            node.isEnd = false;
            node.value = null;
            return new DeleteResult(true, !node.isEnd && node.children.size() == 1);
        }

        char first = firstChar(key);
        Edge<V> edge = node.children.remove(first);
        if (edge == null) {
            return new DeleteResult(false, false);
        }

        int commonLength = commonPrefixLength(key, edge.label);
        if (commonLength < edge.label.length()) {
            node.children.put(first, edge);
            return new DeleteResult(false, false);
        }

        DeleteResult result = deleteRecursive(edge.child, key.substring(commonLength));
        if (!result.deleted) {
            node.children.put(first, edge);
            return result;
        }

        if (result.childMergeable) {
            Map.Entry<Character, Edge<V>> grandchildEntry = edge.child.children.firstEntry();
            Edge<V> grandchild = grandchildEntry.getValue();
            String mergedLabel = edge.label + grandchild.label;
            node.children.put(firstChar(mergedLabel), new Edge<>(mergedLabel, grandchild.child));
        } else if (!edge.child.isEnd && edge.child.children.isEmpty()) {
            // Drop the empty child edge.
        } else {
            node.children.put(first, edge);
        }

        return new DeleteResult(true, !node.isEnd && node.children.size() == 1);
    }

    private static <V> void collectKeys(Node<V> node, String current, List<String> results) {
        if (node.isEnd) {
            results.add(current);
        }
        for (Edge<V> edge : node.children.values()) {
            collectKeys(edge.child, current + edge.label, results);
        }
    }

    private static <V> void collectValues(Node<V> node, String current, Map<String, V> result) {
        if (node.isEnd) {
            result.put(current, node.value);
        }
        for (Edge<V> edge : node.children.values()) {
            collectValues(edge.child, current + edge.label, result);
        }
    }

    private static <V> int countNodes(Node<V> node) {
        int count = 1;
        for (Edge<V> edge : node.children.values()) {
            count += countNodes(edge.child);
        }
        return count;
    }

    private static int commonPrefixLength(String left, String right) {
        int index = 0;
        int limit = Math.min(left.length(), right.length());
        while (index < limit && left.charAt(index) == right.charAt(index)) {
            index++;
        }
        return index;
    }

    private static char firstChar(String value) {
        return value.charAt(0);
    }

    private static final class Node<V> {
        private boolean isEnd;
        private V value;
        private final TreeMap<Character, Edge<V>> children = new TreeMap<>();

        private static <V> Node<V> leaf(V value) {
            Node<V> node = new Node<>();
            node.isEnd = true;
            node.value = value;
            return node;
        }
    }

    private record Edge<V>(String label, Node<V> child) {
    }

    private record DeleteResult(boolean deleted, boolean childMergeable) {
    }
}
