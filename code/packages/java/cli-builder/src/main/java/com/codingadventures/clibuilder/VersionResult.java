package com.codingadventures.clibuilder;

/** Result returned when version text should be shown. */
public record VersionResult(String version) implements ParseOutcome {
}
