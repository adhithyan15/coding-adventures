package com.codingadventures.statemachine;

import java.util.List;

/** A pushdown transition and the resulting stack snapshot. */
public record PdaTraceEntry(
        String source,
        String event,
        String stackRead,
        String target,
        List<String> stackPush,
        List<String> stackAfter) {
    public PdaTraceEntry {
        stackPush = List.copyOf(stackPush);
        stackAfter = List.copyOf(stackAfter);
    }
}
