package com.codingadventures.paintvmascii;

import java.util.Objects;

/** How scene coordinates map to terminal character cells. */
public final class AsciiOptions {
    public final int scaleX;
    public final int scaleY;

    /** The cross-language default: cells eight scene units wide, sixteen tall. */
    public static final AsciiOptions DEFAULT = new AsciiOptions(8, 16);

    public AsciiOptions(int scaleX, int scaleY) {
        this.scaleX = scaleX;
        this.scaleY = scaleY;
    }

    @Override
    public String toString() {
        return "AsciiOptions{scaleX=" + scaleX + ", scaleY=" + scaleY + "}";
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (!(obj instanceof AsciiOptions other)) return false;
        return scaleX == other.scaleX && scaleY == other.scaleY;
    }

    @Override
    public int hashCode() {
        return Objects.hash(scaleX, scaleY);
    }
}
