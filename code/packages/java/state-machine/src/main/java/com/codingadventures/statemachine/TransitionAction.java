package com.codingadventures.statemachine;

/** A named deterministic-transition side effect. */
public record TransitionAction(String name, TransitionEffect effect) {
    public TransitionAction {
        if (name == null || name.isBlank() || effect == null) {
            throw new IllegalArgumentException("actions require a name and effect");
        }
    }
}
