package com.codingadventures.graph;

public final class EdgeNotFoundError extends RuntimeException {
    public EdgeNotFoundError(String leftNode, String rightNode) {
        super("Edge not found: \"" + leftNode + "\" -- \"" + rightNode + "\"");
    }
}
