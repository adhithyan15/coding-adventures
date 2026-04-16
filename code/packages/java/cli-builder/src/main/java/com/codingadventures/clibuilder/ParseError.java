package com.codingadventures.clibuilder;

import java.util.List;

/** One parse-time diagnostic. */
public record ParseError(
        String errorType,
        String message,
        String suggestion,
        List<String> context
) {
    public ParseError {
        context = context == null ? List.of() : List.copyOf(context);
    }

    public String format() {
        StringBuilder builder = new StringBuilder("error[")
                .append(errorType)
                .append("]: ")
                .append(message);
        if (suggestion != null && !suggestion.isBlank()) {
            builder.append("\n  Did you mean: ").append(suggestion);
        }
        if (!context.isEmpty()) {
            builder.append("\n  Context: ").append(String.join(" ", context));
        }
        return builder.toString();
    }
}
