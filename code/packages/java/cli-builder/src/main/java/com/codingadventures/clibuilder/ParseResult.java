package com.codingadventures.clibuilder;

import java.util.List;
import java.util.Map;

/** A fully parsed CLI invocation. */
public record ParseResult(
        String program,
        List<String> commandPath,
        Map<String, Object> flags,
        Map<String, Object> arguments,
        List<String> explicitFlags
) implements ParseOutcome {
    public ParseResult {
        commandPath = List.copyOf(commandPath);
        flags = Map.copyOf(flags);
        arguments = Map.copyOf(arguments);
        explicitFlags = List.copyOf(explicitFlags);
    }
}
