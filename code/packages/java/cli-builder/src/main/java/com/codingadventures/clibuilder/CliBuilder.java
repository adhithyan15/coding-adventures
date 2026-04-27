package com.codingadventures.clibuilder;

import java.nio.file.Path;
import java.util.List;

/** Entry-point helpers for spec validation. */
public final class CliBuilder {
    private CliBuilder() {
    }

    public static ValidationResult validateSpec(String specFilePath) {
        return validateSpec(Path.of(specFilePath));
    }

    public static ValidationResult validateSpec(Path specFilePath) {
        try {
            new SpecLoader().load(specFilePath);
            return new ValidationResult(true, List.of());
        } catch (SpecError error) {
            return new ValidationResult(false, List.of(error.getMessage()));
        }
    }

    public static ValidationResult validateSpecString(String json) {
        try {
            new SpecLoader().loadString(json);
            return new ValidationResult(true, List.of());
        } catch (SpecError error) {
            return new ValidationResult(false, List.of(error.getMessage()));
        }
    }
}
