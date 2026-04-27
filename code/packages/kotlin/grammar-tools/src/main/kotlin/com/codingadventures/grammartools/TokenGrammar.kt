// ============================================================================
// TokenGrammar.kt — Token grammar types and parser for .tokens files
// ============================================================================
//
// A .tokens file is a declarative description of a language's lexical grammar.
// Each line defines a token pattern (regex or literal) that the lexer should
// recognize. Sections organize keywords, skip patterns, error recovery, and
// pattern groups for context-sensitive lexing.
//
// This file contains all the data types and the parser for .tokens files,
// following the Kotlin convention of keeping related types together.
//
// Layer: TE (text/language layer)
// ============================================================================

package com.codingadventures.grammartools

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/**
 * A single token rule from a .tokens file.
 *
 * @property name       Token name, e.g. "NUMBER" or "PLUS"
 * @property pattern    The pattern string (without delimiters)
 * @property isRegex    true if /regex/, false if "literal"
 * @property lineNumber 1-based line where this definition appeared
 * @property alias      Optional type alias (e.g. STRING_DQ -> STRING)
 */
data class TokenDefinition(
    val name: String,
    val pattern: String,
    val isRegex: Boolean,
    val lineNumber: Int,
    val alias: String? = null
)

/**
 * A named set of token definitions for context-sensitive lexing.
 */
data class PatternGroup(
    val name: String,
    val definitions: List<TokenDefinition>
)

/**
 * The complete contents of a parsed .tokens file.
 */
data class TokenGrammar(
    var version: Int = 0,
    var caseInsensitive: Boolean = false,
    var caseSensitive: Boolean = true,
    val definitions: MutableList<TokenDefinition> = mutableListOf(),
    val keywords: MutableList<String> = mutableListOf(),
    var mode: String? = null,
    var escapeMode: String? = null,
    val skipDefinitions: MutableList<TokenDefinition> = mutableListOf(),
    val errorDefinitions: MutableList<TokenDefinition> = mutableListOf(),
    val reservedKeywords: MutableList<String> = mutableListOf(),
    val contextKeywords: MutableList<String> = mutableListOf(),
    val groups: MutableMap<String, PatternGroup> = mutableMapOf()
) {
    /** All defined token names including aliases and group tokens. */
    fun tokenNames(): Set<String> {
        val allDefs = definitions + groups.values.flatMap { it.definitions }
        val names = mutableSetOf<String>()
        for (d in allDefs) {
            names.add(d.name)
            d.alias?.let { names.add(it) }
        }
        return names
    }

    /** Token names as the parser will see them (aliases replace original names). */
    fun effectiveTokenNames(): Set<String> =
        (definitions + groups.values.flatMap { it.definitions })
            .map { it.alias ?: it.name }
            .toSet()
}

// ---------------------------------------------------------------------------
// Exceptions
// ---------------------------------------------------------------------------

class TokenGrammarError(val msg: String, val line: Int) :
    Exception("Line $line: $msg")

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

private val MAGIC_COMMENT = Regex("""^#\s*@(\w+)\s*(.*)$""")
private val GROUP_NAME_RE = Regex("""^[a-z_][a-z0-9_]*$""")
private val TOKEN_NAME_RE = Regex("""^[a-zA-Z_][a-zA-Z0-9_]*$""")
private val RESERVED_GROUP_NAMES = setOf("default", "skip", "keywords", "reserved", "errors")

/**
 * Parse the full text of a .tokens file into a [TokenGrammar].
 */
fun parseTokenGrammar(source: String): TokenGrammar {
    val grammar = TokenGrammar()
    val lines = source.split("\n")
    var currentSection: String? = null

    for ((i, rawLine) in lines.withIndex()) {
        val lineNumber = i + 1
        val line = rawLine.trimEnd()
        val stripped = line.trim()

        if (stripped.isEmpty()) continue

        // Comments and magic comments
        if (stripped.startsWith("#")) {
            MAGIC_COMMENT.matchEntire(stripped)?.let { m ->
                val key = m.groupValues[1]
                val value = m.groupValues[2].trim()
                when (key) {
                    "version" -> value.toIntOrNull()?.let { grammar.version = it }
                    "case_insensitive" -> grammar.caseInsensitive = (value == "true")
                }
            }
            continue
        }

        // mode: directive
        if (stripped.startsWith("mode:")) {
            val v = stripped.removePrefix("mode:").trim()
            if (v.isEmpty()) throw TokenGrammarError("Missing value after 'mode:'", lineNumber)
            grammar.mode = v
            currentSection = null
            continue
        }

        // escapes: directive
        if (stripped.startsWith("escapes:")) {
            val v = stripped.removePrefix("escapes:").trim()
            if (v.isEmpty()) throw TokenGrammarError("Missing value after 'escapes:'", lineNumber)
            grammar.escapeMode = v
            currentSection = null
            continue
        }

        // case_sensitive: directive
        if (stripped.startsWith("case_sensitive:")) {
            val v = stripped.removePrefix("case_sensitive:").trim().lowercase()
            if (v != "true" && v != "false")
                throw TokenGrammarError("Invalid value for 'case_sensitive:': '$v'", lineNumber)
            grammar.caseSensitive = (v == "true")
            currentSection = null
            continue
        }

        // Group headers
        if (stripped.startsWith("group ") && stripped.endsWith(":")) {
            val groupName = stripped.removePrefix("group ").removeSuffix(":").trim()
            if (groupName.isEmpty()) throw TokenGrammarError("Missing group name after 'group'", lineNumber)
            if (!GROUP_NAME_RE.matches(groupName))
                throw TokenGrammarError("Invalid group name: '$groupName'", lineNumber)
            if (groupName in RESERVED_GROUP_NAMES)
                throw TokenGrammarError("Reserved group name: '$groupName'", lineNumber)
            if (groupName in grammar.groups)
                throw TokenGrammarError("Duplicate group name: '$groupName'", lineNumber)
            grammar.groups[groupName] = PatternGroup(groupName, mutableListOf())
            currentSection = "group:$groupName"
            continue
        }

        // Section headers
        when (stripped) {
            "keywords:", "keywords :" -> { currentSection = "keywords"; continue }
            "reserved:", "reserved :" -> { currentSection = "reserved"; continue }
            "skip:", "skip :" -> { currentSection = "skip"; continue }
            "errors:", "errors :" -> { currentSection = "errors"; continue }
            "context_keywords:", "context_keywords :" -> { currentSection = "context_keywords"; continue }
        }

        // Inside a section
        if (currentSection != null) {
            if (line.isNotEmpty() && (line[0] == ' ' || line[0] == '\t')) {
                if (stripped.isEmpty()) continue
                when {
                    currentSection == "keywords" -> grammar.keywords.add(stripped)
                    currentSection == "reserved" -> grammar.reservedKeywords.add(stripped)
                    currentSection == "context_keywords" -> grammar.contextKeywords.add(stripped)
                    currentSection == "skip" -> parseSectionDef(stripped, lineNumber, grammar.skipDefinitions, "skip pattern")
                    currentSection == "errors" -> parseSectionDef(stripped, lineNumber, grammar.errorDefinitions, "error pattern")
                    currentSection!!.startsWith("group:") -> {
                        val gName = currentSection!!.removePrefix("group:")
                        val eqIdx = stripped.indexOf('=')
                        if (eqIdx == -1) throw TokenGrammarError("Expected definition in group '$gName'", lineNumber)
                        val n = stripped.substring(0, eqIdx).trim()
                        val p = stripped.substring(eqIdx + 1).trim()
                        if (n.isEmpty() || p.isEmpty()) throw TokenGrammarError("Incomplete definition in group '$gName'", lineNumber)
                        val defn = parseDefinition(p, n, lineNumber)
                        val old = grammar.groups[gName]!!
                        grammar.groups[gName] = PatternGroup(gName, old.definitions + defn)
                    }
                }
                continue
            }
            currentSection = null
        }

        // Token definition
        val eqIndex = line.indexOf('=')
        if (eqIndex == -1) throw TokenGrammarError("Expected token definition, got: '$stripped'", lineNumber)
        val namePart = line.substring(0, eqIndex).trim()
        val patternPart = line.substring(eqIndex + 1).trim()
        if (namePart.isEmpty()) throw TokenGrammarError("Missing token name before '='", lineNumber)
        if (!TOKEN_NAME_RE.matches(namePart))
            throw TokenGrammarError("Invalid token name: '$namePart'", lineNumber)
        if (patternPart.isEmpty()) throw TokenGrammarError("Missing pattern after '='", lineNumber)
        grammar.definitions.add(parseDefinition(patternPart, namePart, lineNumber))
    }
    return grammar
}

private fun parseSectionDef(stripped: String, lineNumber: Int, target: MutableList<TokenDefinition>, label: String) {
    val eqIdx = stripped.indexOf('=')
    if (eqIdx == -1) throw TokenGrammarError("Expected $label definition", lineNumber)
    val n = stripped.substring(0, eqIdx).trim()
    val p = stripped.substring(eqIdx + 1).trim()
    if (n.isEmpty() || p.isEmpty()) throw TokenGrammarError("Incomplete $label definition", lineNumber)
    target.add(parseDefinition(p, n, lineNumber))
}

internal fun parseDefinition(patternPart: String, namePart: String, lineNumber: Int): TokenDefinition {
    return when {
        patternPart.startsWith("/") -> {
            val lastSlash = findClosingSlash(patternPart)
            if (lastSlash == -1) throw TokenGrammarError("Unclosed regex pattern for token '$namePart'", lineNumber)
            val body = patternPart.substring(1, lastSlash)
            if (body.isEmpty()) throw TokenGrammarError("Empty regex pattern for token '$namePart'", lineNumber)
            val remainder = patternPart.substring(lastSlash + 1).trim()
            val alias = parseAlias(remainder, namePart, lineNumber)
            TokenDefinition(namePart, body, true, lineNumber, alias)
        }
        patternPart.startsWith("\"") -> {
            val closeQuote = patternPart.indexOf('"', 1)
            if (closeQuote == -1) throw TokenGrammarError("Unclosed literal pattern for token '$namePart'", lineNumber)
            val body = patternPart.substring(1, closeQuote)
            if (body.isEmpty()) throw TokenGrammarError("Empty literal pattern for token '$namePart'", lineNumber)
            val remainder = patternPart.substring(closeQuote + 1).trim()
            val alias = parseAlias(remainder, namePart, lineNumber)
            TokenDefinition(namePart, body, false, lineNumber, alias)
        }
        else -> throw TokenGrammarError("Pattern must be /regex/ or \"literal\"", lineNumber)
    }
}

private fun parseAlias(remainder: String, namePart: String, lineNumber: Int): String? {
    if (remainder.isEmpty()) return null
    if (remainder.startsWith("->")) {
        val alias = remainder.removePrefix("->").trim()
        if (alias.isEmpty()) throw TokenGrammarError("Missing alias after '->' for '$namePart'", lineNumber)
        return alias
    }
    throw TokenGrammarError("Unexpected text after pattern for '$namePart'", lineNumber)
}

internal fun findClosingSlash(s: String): Int {
    var inBracket = false
    var i = 1
    while (i < s.length) {
        val ch = s[i]
        if (ch == '\\') { i += 2; continue }
        if (ch == '[' && !inBracket) inBracket = true
        else if (ch == ']' && inBracket) inBracket = false
        else if (ch == '/' && !inBracket) return i
        i++
    }
    val last = s.lastIndexOf('/')
    return if (last > 0) last else -1
}

// ---------------------------------------------------------------------------
// Validator
// ---------------------------------------------------------------------------

fun validateTokenGrammar(grammar: TokenGrammar): List<String> {
    val issues = mutableListOf<String>()
    issues.addAll(validateDefs(grammar.definitions, "token"))
    issues.addAll(validateDefs(grammar.skipDefinitions, "skip pattern"))
    issues.addAll(validateDefs(grammar.errorDefinitions, "error pattern"))

    grammar.mode?.let { if (it != "indentation") issues.add("Unknown lexer mode '$it'") }
    grammar.escapeMode?.let { if (it != "none") issues.add("Unknown escape mode '$it'") }

    for ((name, group) in grammar.groups) {
        if (!GROUP_NAME_RE.matches(name)) issues.add("Invalid group name '$name'")
        if (group.definitions.isEmpty()) issues.add("Empty pattern group '$name'")
        issues.addAll(validateDefs(group.definitions, "group '$name' token"))
    }
    return issues
}

private fun validateDefs(definitions: List<TokenDefinition>, label: String): List<String> {
    val issues = mutableListOf<String>()
    val seen = mutableMapOf<String, Int>()
    for (d in definitions) {
        seen[d.name]?.let {
            issues.add("Line ${d.lineNumber}: Duplicate $label name '${d.name}' (first on line $it)")
        } ?: run { seen[d.name] = d.lineNumber }

        if (d.pattern.isEmpty()) issues.add("Line ${d.lineNumber}: Empty pattern for $label '${d.name}'")

        if (d.isRegex) {
            try { Regex(d.pattern) }
            catch (e: Exception) { issues.add("Line ${d.lineNumber}: Invalid regex for $label '${d.name}'") }
        }

        if (d.name != d.name.uppercase()) issues.add("Line ${d.lineNumber}: Token name '${d.name}' should be UPPER_CASE")
        d.alias?.let { if (it != it.uppercase()) issues.add("Line ${d.lineNumber}: Alias '$it' should be UPPER_CASE") }
    }
    return issues
}
