package com.codingadventures.clibuilder

import java.nio.file.Files
import java.nio.file.Path

internal class PositionalResolver(private val argumentDefs: List<ArgumentDef>) {
    fun resolve(tokens: List<String>, parsedFlags: Map<String, Any?>, context: List<String>): Resolution {
        val result = linkedMapOf<String, Any?>()
        val errors = mutableListOf<ParseError>()

        if (argumentDefs.isEmpty()) {
            if (tokens.isNotEmpty()) {
                errors += ParseError(
                    errorType = "too_many_arguments",
                    message = "Expected no positional arguments, but got ${tokens.size}: $tokens",
                    context = context,
                )
            }
            return Resolution(result, errors)
        }

        val variadicIndex = argumentDefs.indexOfFirst { it.variadic }
        if (variadicIndex < 0) {
            resolveFixed(tokens, parsedFlags, context, result, errors)
        } else {
            resolveVariadic(tokens, variadicIndex, parsedFlags, context, result, errors)
        }

        argumentDefs.forEach { argument ->
            if (!result.containsKey(argument.id)) {
                result[argument.id] = argument.defaultValue ?: if (argument.variadic) emptyList<Any?>() else null
            }
        }

        return Resolution(result, errors)
    }

    private fun resolveFixed(
        tokens: List<String>,
        parsedFlags: Map<String, Any?>,
        context: List<String>,
        result: MutableMap<String, Any?>,
        errors: MutableList<ParseError>,
    ) {
        argumentDefs.forEachIndexed { index, argument ->
            when {
                index < tokens.size -> {
                    val coercion = coerceValue(tokens[index], argument.type, argument.enumValues, context, argument.displayName)
                    if (coercion.error != null) errors += coercion.error else result[argument.id] = coercion.value
                }

                isRequired(argument, parsedFlags) -> errors += ParseError(
                    errorType = "missing_required_argument",
                    message = "Missing required argument: <${argument.displayName}>",
                    context = context,
                )
            }
        }

        if (tokens.size > argumentDefs.size) {
            errors += ParseError(
                errorType = "too_many_arguments",
                message = "Expected at most ${argumentDefs.size} positional argument(s), but got ${tokens.size}",
                context = context,
            )
        }
    }

    private fun resolveVariadic(
        tokens: List<String>,
        variadicIndex: Int,
        parsedFlags: Map<String, Any?>,
        context: List<String>,
        result: MutableMap<String, Any?>,
        errors: MutableList<ParseError>,
    ) {
        val leading = argumentDefs.take(variadicIndex)
        val variadic = argumentDefs[variadicIndex]
        val trailing = argumentDefs.drop(variadicIndex + 1)

        leading.forEachIndexed { index, argument ->
            when {
                index < tokens.size -> {
                    val coercion = coerceValue(tokens[index], argument.type, argument.enumValues, context, argument.displayName)
                    if (coercion.error != null) errors += coercion.error else result[argument.id] = coercion.value
                }

                isRequired(argument, parsedFlags) -> errors += ParseError(
                    errorType = "missing_required_argument",
                    message = "Missing required argument: <${argument.displayName}>",
                    context = context,
                )
            }
        }

        val trailingStart = tokens.size - trailing.size
        trailing.forEachIndexed { index, argument ->
            val tokenIndex = trailingStart + index
            when {
                tokenIndex in tokens.indices -> {
                    val coercion = coerceValue(tokens[tokenIndex], argument.type, argument.enumValues, context, argument.displayName)
                    if (coercion.error != null) errors += coercion.error else result[argument.id] = coercion.value
                }

                isRequired(argument, parsedFlags) -> errors += ParseError(
                    errorType = "missing_required_argument",
                    message = "Missing required argument: <${argument.displayName}>",
                    context = context,
                )
            }
        }

        val variadicStart = minOf(leading.size, tokens.size)
        val variadicEnd = minOf(maxOf(leading.size, trailingStart), tokens.size)
        val variadicTokens = tokens.subList(variadicStart, variadicEnd)
        val count = variadicTokens.size

        when {
            count < variadic.variadicMin -> errors += ParseError(
                errorType = "too_few_arguments",
                message = "Expected at least ${variadic.variadicMin} <${variadic.displayName}>, got $count",
                context = context,
            )

            variadic.variadicMax != null && count > variadic.variadicMax -> errors += ParseError(
                errorType = "too_many_arguments",
                message = "Expected at most ${variadic.variadicMax} <${variadic.displayName}>, got $count",
                context = context,
            )
        }

        val coerced = mutableListOf<Any?>()
        variadicTokens.forEach { token ->
            val coercion = coerceValue(token, variadic.type, variadic.enumValues, context, variadic.displayName)
            if (coercion.error != null) errors += coercion.error else coerced += coercion.value
        }
        result[variadic.id] = coerced
    }

    private fun isRequired(argument: ArgumentDef, parsedFlags: Map<String, Any?>): Boolean {
        if (!argument.required) return false
        argument.requiredUnlessFlag.forEach { flagId ->
            val value = parsedFlags[flagId]
            if (value != null && value != false) {
                return false
            }
        }
        return true
    }

    companion object {
        fun coerceValue(raw: String, argType: String, enumValues: List<String>, context: List<String>, argName: String): CoercionResult =
            try {
                when (argType) {
                    "boolean" -> CoercionResult(raw.equals("true", true) || raw == "1" || raw.equals("yes", true), null)
                    "integer" -> CoercionResult(raw.toLong(), null)
                    "float" -> CoercionResult(raw.toDouble(), null)
                    "enum" -> {
                        if (raw !in enumValues) {
                            CoercionResult(
                                null,
                                ParseError(
                                    errorType = "invalid_enum_value",
                                    message = "Invalid value '$raw' for argument '$argName'. Must be one of: ${enumValues.joinToString(", ")}",
                                    context = context,
                                ),
                            )
                        } else {
                            CoercionResult(raw, null)
                        }
                    }

                    "string" -> {
                        if (raw.isEmpty()) {
                            CoercionResult(null, ParseError("invalid_value", "Argument '$argName' must be a non-empty string", context = context))
                        } else {
                            CoercionResult(raw, null)
                        }
                    }

                    "path" -> CoercionResult(raw, null)
                    "file" -> {
                        val path = Path.of(raw)
                        if (!Files.isRegularFile(path)) {
                            CoercionResult(null, ParseError("invalid_value", "Argument '$argName': '$raw' is not an existing file", context = context))
                        } else {
                            CoercionResult(raw, null)
                        }
                    }

                    "directory" -> {
                        val path = Path.of(raw)
                        if (!Files.isDirectory(path)) {
                            CoercionResult(null, ParseError("invalid_value", "Argument '$argName': '$raw' is not an existing directory", context = context))
                        } else {
                            CoercionResult(raw, null)
                        }
                    }

                    else -> CoercionResult(raw, null)
                }
            } catch (_: NumberFormatException) {
                val kind = if (argType == "float") "float" else "integer"
                CoercionResult(null, ParseError("invalid_value", "Invalid $kind for argument '$argName': '$raw'", context = context))
            } catch (_: RuntimeException) {
                CoercionResult(null, ParseError("invalid_value", "Argument '$argName': cannot access '$raw'", context = context))
            }
    }

    internal data class Resolution(val arguments: Map<String, Any?>, val errors: List<ParseError>)
    internal data class CoercionResult(val value: Any?, val error: ParseError?)
}
