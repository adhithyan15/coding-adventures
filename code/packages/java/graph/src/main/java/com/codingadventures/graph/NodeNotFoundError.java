package com.codingadventures.graph;

public final class NodeNotFoundError extends RuntimeException {
    public NodeNotFoundError(String node) {
        super("Node not found: \"" + node + "\"");
    }
}
