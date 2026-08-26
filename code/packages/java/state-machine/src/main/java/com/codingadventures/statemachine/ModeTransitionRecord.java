package com.codingadventures.statemachine;

/** One modal transition. */
public record ModeTransitionRecord(String fromMode, String trigger, String toMode) {}
