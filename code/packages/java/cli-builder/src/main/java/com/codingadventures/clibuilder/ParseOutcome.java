package com.codingadventures.clibuilder;

/** Base type for the three parser outcomes. */
public sealed interface ParseOutcome permits ParseResult, HelpResult, VersionResult {
}
