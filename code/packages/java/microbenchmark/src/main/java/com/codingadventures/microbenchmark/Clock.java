package com.codingadventures.microbenchmark;

/** Monotonic, nonnegative signed 64-bit nanosecond source. */
@FunctionalInterface
public interface Clock { long now(); }
