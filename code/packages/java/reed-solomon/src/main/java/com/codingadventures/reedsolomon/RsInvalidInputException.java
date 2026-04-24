package com.codingadventures.reedsolomon;

/**
 * Thrown when encode/decode receives invalid parameters.
 *
 * <p>Invalid conditions:
 * <ul>
 *   <li>{@code nCheck} is 0 or an odd number (must be a positive even integer)</li>
 *   <li>Total codeword length ({@code message.length + nCheck}) exceeds 255</li>
 *   <li>Received codeword is shorter than {@code nCheck}</li>
 * </ul>
 */
public class RsInvalidInputException extends IllegalArgumentException {
    /**
     * Creates an InvalidInput exception with a descriptive message.
     *
     * @param message a human-readable description of what was invalid
     */
    public RsInvalidInputException(String message) {
        super("reed-solomon: invalid input — " + message);
    }
}
