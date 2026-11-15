package com.codingadventures.gradientdescent;

/**
 * Every error id, error message and placeholder used by the gradient-descent
 * package, collected in one place (Standing Rule #20: zero string literals in
 * operational logic).
 *
 * <p>All messages are <em>static</em> and <em>payload-blind</em>: they never
 * mention lengths, indices, weights, gradients or the learning rate. A model's
 * weights can be sensitive (they encode its training data), so an error that
 * escapes into a log must not leak them (CWE-209).
 */
public final class GradientDescentMessages {
    private GradientDescentMessages() {}

    // ---- Stable, machine-readable error ids (shared across every language lane).
    public static final String ERROR_NULL_INPUT = "null-input";
    public static final String ERROR_LENGTH_MISMATCH = "length-mismatch";
    public static final String ERROR_EMPTY_INPUT = "empty-input";
    public static final String ERROR_INPUT_TOO_LARGE = "input-too-large";
    public static final String ERROR_NON_FINITE_LEARNING_RATE = "non-finite-learning-rate";
    public static final String ERROR_NON_FINITE_INPUT = "non-finite-input";
    public static final String ERROR_NUMERIC_OVERFLOW = "numeric-overflow";

    // ---- Human-readable, static messages (one per error id).
    public static final String MESSAGE_NULL_INPUT = "weights and gradients must not be null";
    public static final String MESSAGE_LENGTH_MISMATCH = "weights and gradients must have the same length";
    public static final String MESSAGE_EMPTY_INPUT = "weights and gradients must not be empty";
    public static final String MESSAGE_INPUT_TOO_LARGE = "input vectors exceed the maximum supported length";
    public static final String MESSAGE_NON_FINITE_LEARNING_RATE = "learning rate must be a finite number";
    public static final String MESSAGE_NON_FINITE_INPUT = "weights and gradients must be finite numbers";
    public static final String MESSAGE_NUMERIC_OVERFLOW = "updated weight is not a finite number";

    // ---- Placeholders for any future diagnostic that must stand in for a payload.
    public static final String REDACTED_PLACEHOLDER = "<redacted>";
    public static final String INVALID_PLACEHOLDER = "<invalid>";
}
