package com.codingadventures.paintvmascii;

/** The outcome of {@link PaintVmAscii#render}: either rendered text, or an error. */
public sealed interface PaintVmAsciiResult {
    record Ok(String text) implements PaintVmAsciiResult {}

    record Err(PaintVmAsciiError error) implements PaintVmAsciiResult {}
}
