package com.codingadventures.testbootstrap.pbt;

/**
 * The result of shrinking: the smallest value found that still falsifies, and
 * how many <em>accepted</em> moves it took (the target probe is one move).
 */
public record Shrunk<T>(T value, int steps) {}
