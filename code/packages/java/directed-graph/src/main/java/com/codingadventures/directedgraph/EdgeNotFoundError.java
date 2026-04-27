package com.codingadventures.directedgraph;

public final class EdgeNotFoundError extends RuntimeException {
    private final String fromNode;
    private final String toNode;

    public EdgeNotFoundError(String fromNode, String toNode) {
        super("Edge not found: \"" + fromNode + "\" -> \"" + toNode + "\"");
        this.fromNode = fromNode;
        this.toNode = toNode;
    }

    public String fromNode() {
        return fromNode;
    }

    public String toNode() {
        return toNode;
    }
}
