package com.codingadventures.reedsolomon;

/**
 * Thrown when decoding fails because more errors occurred than the code's
 * correction capacity {@code t = nCheck / 2}.
 *
 * <p>This is a checked RuntimeException: callers that decode arbitrary received
 * bytes must be prepared to handle this case when the codeword is unrecoverable.
 *
 * <p>Possible causes:
 * <ul>
 *   <li>More than {@code t} byte positions were corrupted.</li>
 *   <li>The Forney algorithm's denominator evaluated to zero (indicates
 *       more errors than the code can handle).</li>
 *   <li>The Chien search found a different number of positions than
 *       the Berlekamp-Massey error count (severe corruption).</li>
 * </ul>
 */
public class RsTooManyErrorsException extends RuntimeException {
    /** Creates a TooManyErrors exception with a fixed message. */
    public RsTooManyErrorsException() {
        super("reed-solomon: too many errors — codeword is unrecoverable");
    }
}
