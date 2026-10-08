package com.codingadventures.bezier2d;

/** The two immutable subcurves produced by de Casteljau subdivision. */
public record Split<T>(T left, T right) {}
