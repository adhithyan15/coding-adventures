package com.codingadventures.testbootstrap.pbt;

/**
 * A generator could not produce a value, for example a filter that rejected
 * every candidate. The runner reports it as {@code property-exhausted}.
 */
public final class ExhaustedException extends RuntimeException {
  private static final long serialVersionUID = 1L;

  public ExhaustedException(String message) {
    super(message);
  }
}
