package com.codingadventures.clibuilder;

import java.util.List;
import java.util.stream.Collectors;

/** Raised when parsing collects one or more user-facing errors. */
public final class ParseErrors extends CliBuilderError {
    private final List<ParseError> errors;

    public ParseErrors(List<ParseError> errors) {
        super(errors.size() + " parse error(s) found");
        this.errors = List.copyOf(errors);
    }

    public List<ParseError> errors() {
        return errors;
    }

    @Override
    public String getMessage() {
        return errors.stream().map(ParseError::format).collect(Collectors.joining("\n\n"));
    }
}
