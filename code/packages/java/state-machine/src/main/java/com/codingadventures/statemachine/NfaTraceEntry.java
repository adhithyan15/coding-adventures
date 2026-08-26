package com.codingadventures.statemachine;

import java.util.Set;

/** The active NFA state set before and after consuming an event. */
public record NfaTraceEntry(Set<String> source, String event, Set<String> target) {
    public NfaTraceEntry {
        source = Set.copyOf(source);
        target = Set.copyOf(target);
    }
}
