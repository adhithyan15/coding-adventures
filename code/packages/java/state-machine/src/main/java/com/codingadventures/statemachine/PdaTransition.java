package com.codingadventures.statemachine;

import java.util.List;

/** A deterministic pushdown transition; a null input denotes epsilon. */
public record PdaTransition(
        String source,
        String event,
        String stackRead,
        String target,
        List<String> stackPush) {
    public PdaTransition {
        if (source == null || stackRead == null || target == null || stackPush == null) {
            throw new IllegalArgumentException("PDA transition fields must be non-null except event");
        }
        stackPush = List.copyOf(stackPush);
    }
}
