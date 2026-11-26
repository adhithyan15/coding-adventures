package com.codingadventures.testbootstrap.pbt;

/**
 * Whether output is public CI output. The default, {@link #UNSET}, redacts: the
 * engine fails closed, because a seed plus a property is a working exploit
 * reproducer (VER00 section 7.3) and this package cannot read the environment
 * to find out where it is running.
 */
public enum CiMode {
  /** Treated as CI: witnesses and seeds are redacted. */
  UNSET,
  /** Redacts. */
  ON,
  /** Local development: messages carry the seed and the witness. */
  OFF
}
