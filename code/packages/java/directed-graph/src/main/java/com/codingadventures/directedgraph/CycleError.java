package com.codingadventures.directedgraph;

import java.util.List;

public final class CycleError extends RuntimeException {
    private final List<String> cycle;

    public CycleError(String message, List<String> cycle) {
        super(message);
        this.cycle = List.copyOf(cycle);
    }

    public List<String> cycle() {
        return cycle;
    }
}
