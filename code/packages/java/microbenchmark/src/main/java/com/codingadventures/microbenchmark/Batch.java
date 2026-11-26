package com.codingadventures.microbenchmark;

/** Runs a batch and returns its signed 64-bit whole-batch checksum. */
@FunctionalInterface
public interface Batch { long run(long operations) throws Exception; }
