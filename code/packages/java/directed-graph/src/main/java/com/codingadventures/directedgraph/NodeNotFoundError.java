package com.codingadventures.directedgraph;

public final class NodeNotFoundError extends RuntimeException {
    private final String node;

    public NodeNotFoundError(String node) {
        super("Node not found: \"" + node + "\"");
        this.node = node;
    }

    public String node() {
        return node;
    }
}
