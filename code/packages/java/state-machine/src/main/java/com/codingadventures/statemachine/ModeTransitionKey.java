package com.codingadventures.statemachine;

/** A modal-state-machine mode and trigger lookup key. */
public record ModeTransitionKey(String mode, String trigger) {
    public ModeTransitionKey {
        if (mode == null || trigger == null) {
            throw new IllegalArgumentException("mode and trigger must be non-null");
        }
    }
}
