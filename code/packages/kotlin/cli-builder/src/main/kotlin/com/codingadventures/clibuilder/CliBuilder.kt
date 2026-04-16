package com.codingadventures.clibuilder

import java.nio.file.Path

open class CliBuilderError(message: String, cause: Throwable? = null) : RuntimeException(message, cause)

class SpecError(detail: String, cause: Throwable? = null) : CliBuilderError("CliBuilder spec error: $detail", cause)

data class ParseError(
    val errorType: String,
    val message: String,
    val suggestion: String? = null,
    val context: List<String> = emptyList(),
) {
    fun format(): String = buildString {
        append("error[").append(errorType).append("]: ").append(message)
        if (!suggestion.isNullOrBlank()) {
            append("\n  Did you mean: ").append(suggestion)
        }
        if (context.isNotEmpty()) {
            append("\n  Context: ").append(context.joinToString(" "))
        }
    }
}

class ParseErrors(val errors: List<ParseError>) : CliBuilderError("${errors.size} parse error(s) found") {
    override val message: String = errors.joinToString("\n\n") { it.format() }
}

sealed interface ParseOutcome

data class ParseResult(
    val program: String,
    val commandPath: List<String>,
    val flags: Map<String, Any?>,
    val arguments: Map<String, Any?>,
    val explicitFlags: List<String>,
) : ParseOutcome

data class HelpResult(val text: String, val commandPath: List<String>) : ParseOutcome

data class VersionResult(val version: String) : ParseOutcome

data class ValidationResult(val valid: Boolean, val errors: List<String> = emptyList())

object CliBuilder {
    fun validateSpec(specFilePath: String): ValidationResult = validateSpec(Path.of(specFilePath))

    fun validateSpec(specFilePath: Path): ValidationResult = try {
        SpecLoader().load(specFilePath)
        ValidationResult(true)
    } catch (error: SpecError) {
        ValidationResult(false, listOf(error.message ?: "Unknown spec error"))
    }

    fun validateSpecString(json: String): ValidationResult = try {
        SpecLoader().loadString(json)
        ValidationResult(true)
    } catch (error: SpecError) {
        ValidationResult(false, listOf(error.message ?: "Unknown spec error"))
    }
}
