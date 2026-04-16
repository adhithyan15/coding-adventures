package com.codingadventures.clibuilder;

import java.util.List;

/** Result returned when help text should be shown. */
public record HelpResult(String text, List<String> commandPath) implements ParseOutcome {
    public HelpResult {
        commandPath = List.copyOf(commandPath);
    }
}
