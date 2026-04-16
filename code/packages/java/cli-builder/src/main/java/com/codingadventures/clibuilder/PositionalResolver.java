package com.codingadventures.clibuilder;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

final class PositionalResolver {
    private final List<ArgumentDef> argumentDefs;

    PositionalResolver(List<ArgumentDef> argumentDefs) {
        this.argumentDefs = List.copyOf(argumentDefs);
    }

    Resolution resolve(List<String> tokens, Map<String, Object> parsedFlags, List<String> context) {
        Map<String, Object> result = new LinkedHashMap<>();
        List<ParseError> errors = new ArrayList<>();

        if (argumentDefs.isEmpty()) {
            if (!tokens.isEmpty()) {
                errors.add(new ParseError(
                        "too_many_arguments",
                        "Expected no positional arguments, but got " + tokens.size() + ": " + tokens,
                        null,
                        context
                ));
            }
            return new Resolution(result, errors);
        }

        int variadicIndex = -1;
        for (int index = 0; index < argumentDefs.size(); index += 1) {
            if (argumentDefs.get(index).variadic()) {
                variadicIndex = index;
                break;
            }
        }

        if (variadicIndex < 0) {
            resolveFixed(tokens, parsedFlags, context, result, errors);
        } else {
            resolveVariadic(tokens, variadicIndex, parsedFlags, context, result, errors);
        }

        for (ArgumentDef argument : argumentDefs) {
            if (!result.containsKey(argument.id())) {
                if (argument.defaultValue() != null) {
                    result.put(argument.id(), argument.defaultValue());
                } else if (argument.variadic()) {
                    result.put(argument.id(), List.of());
                } else {
                    result.put(argument.id(), null);
                }
            }
        }

        return new Resolution(result, errors);
    }

    static CoercionResult coerceValue(
            String raw,
            String argType,
            List<String> enumValues,
            List<String> context,
            String argName
    ) {
        try {
            return switch (argType) {
                case "boolean" -> new CoercionResult(
                        "true".equalsIgnoreCase(raw) || "1".equals(raw) || "yes".equalsIgnoreCase(raw),
                        null
                );
                case "integer" -> new CoercionResult(Long.parseLong(raw), null);
                case "float" -> new CoercionResult(Double.parseDouble(raw), null);
                case "enum" -> {
                    if (!enumValues.contains(raw)) {
                        yield new CoercionResult(null, new ParseError(
                                "invalid_enum_value",
                                "Invalid value '" + raw + "' for argument '" + argName + "'. Must be one of: "
                                        + String.join(", ", enumValues),
                                null,
                                context
                        ));
                    }
                    yield new CoercionResult(raw, null);
                }
                case "string" -> {
                    if (raw.isEmpty()) {
                        yield new CoercionResult(null, new ParseError(
                                "invalid_value",
                                "Argument '" + argName + "' must be a non-empty string",
                                null,
                                context
                        ));
                    }
                    yield new CoercionResult(raw, null);
                }
                case "path" -> new CoercionResult(raw, null);
                case "file" -> {
                    Path path = Path.of(raw);
                    if (!Files.isRegularFile(path)) {
                        yield new CoercionResult(null, new ParseError(
                                "invalid_value",
                                "Argument '" + argName + "': '" + raw + "' is not an existing file",
                                null,
                                context
                        ));
                    }
                    yield new CoercionResult(raw, null);
                }
                case "directory" -> {
                    Path path = Path.of(raw);
                    if (!Files.isDirectory(path)) {
                        yield new CoercionResult(null, new ParseError(
                                "invalid_value",
                                "Argument '" + argName + "': '" + raw + "' is not an existing directory",
                                null,
                                context
                        ));
                    }
                    yield new CoercionResult(raw, null);
                }
                default -> new CoercionResult(raw, null);
            };
        } catch (NumberFormatException error) {
            String kind = "float".equals(argType) ? "float" : "integer";
            return new CoercionResult(null, new ParseError(
                    "invalid_value",
                    "Invalid " + kind + " for argument '" + argName + "': '" + raw + "'",
                    null,
                    context
            ));
        } catch (RuntimeException error) {
            return new CoercionResult(null, new ParseError(
                    "invalid_value",
                    "Argument '" + argName + "': cannot access '" + raw + "'",
                    null,
                    context
            ));
        }
    }

    private void resolveFixed(
            List<String> tokens,
            Map<String, Object> parsedFlags,
            List<String> context,
            Map<String, Object> result,
            List<ParseError> errors
    ) {
        for (int index = 0; index < argumentDefs.size(); index += 1) {
            ArgumentDef argument = argumentDefs.get(index);
            if (index < tokens.size()) {
                CoercionResult coercion = coerceValue(tokens.get(index), argument.type(), argument.enumValues(), context, argument.displayName());
                if (coercion.error() != null) {
                    errors.add(coercion.error());
                } else {
                    result.put(argument.id(), coercion.value());
                }
            } else if (isRequired(argument, parsedFlags)) {
                errors.add(new ParseError(
                        "missing_required_argument",
                        "Missing required argument: <" + argument.displayName() + ">",
                        null,
                        context
                ));
            }
        }

        if (tokens.size() > argumentDefs.size()) {
            errors.add(new ParseError(
                    "too_many_arguments",
                    "Expected at most " + argumentDefs.size() + " positional argument(s), but got " + tokens.size(),
                    null,
                    context
            ));
        }
    }

    private void resolveVariadic(
            List<String> tokens,
            int variadicIndex,
            Map<String, Object> parsedFlags,
            List<String> context,
            Map<String, Object> result,
            List<ParseError> errors
    ) {
        List<ArgumentDef> leading = argumentDefs.subList(0, variadicIndex);
        ArgumentDef variadic = argumentDefs.get(variadicIndex);
        List<ArgumentDef> trailing = argumentDefs.subList(variadicIndex + 1, argumentDefs.size());

        for (int index = 0; index < leading.size(); index += 1) {
            ArgumentDef argument = leading.get(index);
            if (index < tokens.size()) {
                CoercionResult coercion = coerceValue(tokens.get(index), argument.type(), argument.enumValues(), context, argument.displayName());
                if (coercion.error() != null) {
                    errors.add(coercion.error());
                } else {
                    result.put(argument.id(), coercion.value());
                }
            } else if (isRequired(argument, parsedFlags)) {
                errors.add(new ParseError(
                        "missing_required_argument",
                        "Missing required argument: <" + argument.displayName() + ">",
                        null,
                        context
                ));
            }
        }

        int trailingStart = tokens.size() - trailing.size();
        for (int index = 0; index < trailing.size(); index += 1) {
            ArgumentDef argument = trailing.get(index);
            int tokenIndex = trailingStart + index;
            if (tokenIndex >= 0 && tokenIndex < tokens.size()) {
                CoercionResult coercion = coerceValue(tokens.get(tokenIndex), argument.type(), argument.enumValues(), context, argument.displayName());
                if (coercion.error() != null) {
                    errors.add(coercion.error());
                } else {
                    result.put(argument.id(), coercion.value());
                }
            } else if (isRequired(argument, parsedFlags)) {
                errors.add(new ParseError(
                        "missing_required_argument",
                        "Missing required argument: <" + argument.displayName() + ">",
                        null,
                        context
                ));
            }
        }

        int variadicStart = Math.min(leading.size(), tokens.size());
        int variadicEnd = Math.min(Math.max(leading.size(), trailingStart), tokens.size());
        List<String> variadicTokens = tokens.subList(variadicStart, variadicEnd);
        int count = variadicTokens.size();
        if (count < variadic.variadicMin()) {
            errors.add(new ParseError(
                    "too_few_arguments",
                    "Expected at least " + variadic.variadicMin() + " <" + variadic.displayName() + ">, got " + count,
                    null,
                    context
            ));
        } else if (variadic.variadicMax() != null && count > variadic.variadicMax()) {
            errors.add(new ParseError(
                    "too_many_arguments",
                    "Expected at most " + variadic.variadicMax() + " <" + variadic.displayName() + ">, got " + count,
                    null,
                    context
            ));
        }

        List<Object> coerced = new ArrayList<>();
        for (String token : variadicTokens) {
            CoercionResult coercion = coerceValue(token, variadic.type(), variadic.enumValues(), context, variadic.displayName());
            if (coercion.error() != null) {
                errors.add(coercion.error());
            } else {
                coerced.add(coercion.value());
            }
        }
        result.put(variadic.id(), List.copyOf(coerced));
    }

    private boolean isRequired(ArgumentDef argument, Map<String, Object> parsedFlags) {
        if (!argument.required()) {
            return false;
        }
        for (String flagId : argument.requiredUnlessFlag()) {
            Object value = parsedFlags.get(flagId);
            if (value != null && !Boolean.FALSE.equals(value)) {
                return false;
            }
        }
        return true;
    }

    record Resolution(Map<String, Object> arguments, List<ParseError> errors) {
    }

    record CoercionResult(Object value, ParseError error) {
    }
}
