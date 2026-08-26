package com.codingadventures.statemachine;

/** One deterministic transition, including the optional named action. */
public record TransitionRecord(String source, String event, String target, String actionName) {}
