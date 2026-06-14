// ============================================================================
// SiliconException.java — error type for the silicon simulation stack JNI
// ============================================================================
//
// Inherits from RuntimeException so callers are not forced to catch it, but
// native method signatures declare "throws SiliconException" to document
// that the error can occur.

package com.codingadventures.silicon;

/**
 * Thrown by {@link SiliconSim} native methods when the Rust implementation
 * returns an error.  Causes include invalid parameters (negative doping,
 * unknown device type, malformed cross-section wire) and injection attempts
 * (material names containing '|' or ':').
 */
public class SiliconException extends RuntimeException {
    public SiliconException(String message) {
        super(message);
    }
}
