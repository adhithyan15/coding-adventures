package com.codingadventures.clibuilder;

import java.util.List;

/** Result of validating a spec without throwing. */
public record ValidationResult(boolean valid, List<String> errors) {
    public ValidationResult {
        errors = errors == null ? List.of() : List.copyOf(errors);
    }
}
