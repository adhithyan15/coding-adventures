package com.codingadventures.testbootstrap.pbt;

/**
 * A shrinker was handed a start value that satisfies the property. Shrinking a
 * value that does not fail is a caller bug (VER05e section 4).
 */
public final class NotFalsifyingException extends IllegalArgumentException {
  private static final long serialVersionUID = 1L;

  public NotFalsifyingException() {
    super("initial value does not falsify the property");
  }
}
