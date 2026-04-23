// ============================================================================
// ParserGrammar.kt — Parser grammar types and parser for .grammar files
// ============================================================================
//
// A .grammar file uses EBNF to describe how tokens combine into valid programs.
// This file contains the grammar element sealed hierarchy, the ParserGrammar
// data class, the tokenizer, the recursive descent parser, and the validators.
//
// Layer: TE (text/language layer)
// ============================================================================

package com.codingadventures.grammartools

// ---------------------------------------------------------------------------
// Grammar element types (sealed hierarchy)
// ---------------------------------------------------------------------------

/** Base type for all grammar rule body elements. */
sealed interface GrammarElement

data class RuleReference(val name: String, val isToken: Boolean) : GrammarElement
data class Literal(val value: String) : GrammarElement
data class Sequence(val elements: List<GrammarElement>) : GrammarElement
data class Alternation(val choices: List<GrammarElement>) : GrammarElement
data class Repetition(val element: GrammarElement) : GrammarElement
data class Optional(val element: GrammarElement) : GrammarElement
data class Group(val element: GrammarElement) : GrammarElement
data class PositiveLookahead(val element: GrammarElement) : GrammarElement
data class NegativeLookahead(val element: GrammarElement) : GrammarElement
data class OneOrMoreRepetition(val element: GrammarElement) : GrammarElement
data class SeparatedRepetition(
    val element: GrammarElement,
    val separator: GrammarElement,
    val atLeastOne: Boolean
) : GrammarElement

data class GrammarRule(val name: String, val body: GrammarElement, val lineNumber: Int)

data class ParserGrammar(
    var version: Int = 0,
    val rules: MutableList<GrammarRule> = mutableListOf()
) {
    fun ruleNames(): Set<String> = rules.map { it.name }.toSet()

    fun ruleReferences(): Set<String> {
        val refs = mutableSetOf<String>()
        rules.forEach { collectRuleRefs(it.body, refs) }
        return refs
    }

    fun tokenReferences(): Set<String> {
        val refs = mutableSetOf<String>()
        rules.forEach { collectTokenRefs(it.body, refs) }
        return refs
    }
}

class ParserGrammarError(message: String, val line: Int) :
    Exception("Line $line: $message")

// ---------------------------------------------------------------------------
// Reference collectors
// ---------------------------------------------------------------------------

private fun collectRuleRefs(element: GrammarElement, refs: MutableSet<String>) {
    when (element) {
        is RuleReference -> if (!element.isToken) refs.add(element.name)
        is Sequence -> element.elements.forEach { collectRuleRefs(it, refs) }
        is Alternation -> element.choices.forEach { collectRuleRefs(it, refs) }
        is Repetition -> collectRuleRefs(element.element, refs)
        is Optional -> collectRuleRefs(element.element, refs)
        is Group -> collectRuleRefs(element.element, refs)
        is PositiveLookahead -> collectRuleRefs(element.element, refs)
        is NegativeLookahead -> collectRuleRefs(element.element, refs)
        is OneOrMoreRepetition -> collectRuleRefs(element.element, refs)
        is SeparatedRepetition -> { collectRuleRefs(element.element, refs); collectRuleRefs(element.separator, refs) }
        is Literal -> {}
    }
}

private fun collectTokenRefs(element: GrammarElement, refs: MutableSet<String>) {
    when (element) {
        is RuleReference -> if (element.isToken) refs.add(element.name)
        is Sequence -> element.elements.forEach { collectTokenRefs(it, refs) }
        is Alternation -> element.choices.forEach { collectTokenRefs(it, refs) }
        is Repetition -> collectTokenRefs(element.element, refs)
        is Optional -> collectTokenRefs(element.element, refs)
        is Group -> collectTokenRefs(element.element, refs)
        is PositiveLookahead -> collectTokenRefs(element.element, refs)
        is NegativeLookahead -> collectTokenRefs(element.element, refs)
        is OneOrMoreRepetition -> collectTokenRefs(element.element, refs)
        is SeparatedRepetition -> { collectTokenRefs(element.element, refs); collectTokenRefs(element.separator, refs) }
        is Literal -> {}
    }
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

private data class InternalToken(val kind: String, val value: String, val line: Int)

private fun tokenizeGrammar(source: String): List<InternalToken> {
    val tokens = mutableListOf<InternalToken>()
    val lines = source.split("\n")
    for ((i, rawLine) in lines.withIndex()) {
        val lineNum = i + 1
        val line = rawLine.trimEnd()
        val stripped = line.trim()
        if (stripped.isEmpty() || stripped.startsWith("#")) continue

        var j = 0
        while (j < line.length) {
            val ch = line[j]
            if (ch == ' ' || ch == '\t') { j++; continue }
            if (ch == '#') break
            when (ch) {
                '=' -> { tokens.add(InternalToken("EQUALS", "=", lineNum)); j++ }
                ';' -> { tokens.add(InternalToken("SEMI", ";", lineNum)); j++ }
                '|' -> { tokens.add(InternalToken("PIPE", "|", lineNum)); j++ }
                '{' -> { tokens.add(InternalToken("LBRACE", "{", lineNum)); j++ }
                '}' -> { tokens.add(InternalToken("RBRACE", "}", lineNum)); j++ }
                '[' -> { tokens.add(InternalToken("LBRACKET", "[", lineNum)); j++ }
                ']' -> { tokens.add(InternalToken("RBRACKET", "]", lineNum)); j++ }
                '(' -> { tokens.add(InternalToken("LPAREN", "(", lineNum)); j++ }
                ')' -> { tokens.add(InternalToken("RPAREN", ")", lineNum)); j++ }
                '&' -> { tokens.add(InternalToken("AMPERSAND", "&", lineNum)); j++ }
                '!' -> { tokens.add(InternalToken("BANG", "!", lineNum)); j++ }
                '+' -> { tokens.add(InternalToken("PLUS", "+", lineNum)); j++ }
                '/' -> {
                    if (j + 1 < line.length && line[j + 1] == '/') {
                        tokens.add(InternalToken("DOUBLE_SLASH", "//", lineNum)); j += 2
                    } else throw ParserGrammarError("Unexpected character '/'", lineNum)
                }
                '"' -> {
                    var k = j + 1
                    while (k < line.length && line[k] != '"') {
                        if (line[k] == '\\') k++
                        k++
                    }
                    if (k >= line.length) throw ParserGrammarError("Unterminated string literal", lineNum)
                    tokens.add(InternalToken("STRING", line.substring(j + 1, k), lineNum))
                    j = k + 1
                }
                else -> {
                    if (ch.isLetter() || ch == '_') {
                        var k = j
                        while (k < line.length && (line[k].isLetterOrDigit() || line[k] == '_')) k++
                        tokens.add(InternalToken("IDENT", line.substring(j, k), lineNum))
                        j = k
                    } else throw ParserGrammarError("Unexpected character '$ch'", lineNum)
                }
            }
        }
    }
    tokens.add(InternalToken("EOF", "", lines.size))
    return tokens
}

// ---------------------------------------------------------------------------
// Recursive descent parser
// ---------------------------------------------------------------------------

private class GrammarParserImpl(private val tokens: List<InternalToken>) {
    private var pos = 0
    fun peek() = tokens[pos]
    fun advance() = tokens[pos++]
    fun expect(kind: String): InternalToken {
        val tok = advance()
        if (tok.kind != kind) throw ParserGrammarError("Expected $kind, got ${tok.kind}", tok.line)
        return tok
    }

    fun parseRules(): List<GrammarRule> {
        val rules = mutableListOf<GrammarRule>()
        while (peek().kind != "EOF") rules.add(parseRule())
        return rules
    }

    private fun parseRule(): GrammarRule {
        val nameTok = expect("IDENT")
        expect("EQUALS")
        val body = parseBody()
        expect("SEMI")
        return GrammarRule(nameTok.value, body, nameTok.line)
    }

    fun parseBody(): GrammarElement {
        val first = parseSequence()
        val alts = mutableListOf(first)
        while (peek().kind == "PIPE") { advance(); alts.add(parseSequence()) }
        return if (alts.size == 1) alts[0] else Alternation(alts.toList())
    }

    private fun parseSequence(): GrammarElement {
        val elems = mutableListOf<GrammarElement>()
        while (peek().kind !in listOf("PIPE", "SEMI", "RBRACE", "RBRACKET", "RPAREN", "EOF", "DOUBLE_SLASH")) {
            elems.add(parseElement())
        }
        if (elems.isEmpty()) throw ParserGrammarError("Expected at least one element", peek().line)
        return if (elems.size == 1) elems[0] else Sequence(elems.toList())
    }

    private fun parseElement(): GrammarElement {
        val tok = peek()
        if (tok.kind == "AMPERSAND") { advance(); return PositiveLookahead(parseElement()) }
        if (tok.kind == "BANG") { advance(); return NegativeLookahead(parseElement()) }
        return when (tok.kind) {
            "IDENT" -> { advance(); RuleReference(tok.value, tok.value[0].isUpperCase()) }
            "STRING" -> { advance(); Literal(tok.value) }
            "LBRACE" -> {
                advance()
                val body = parseBody()
                if (peek().kind == "DOUBLE_SLASH") {
                    advance()
                    val sep = parseBody()
                    expect("RBRACE")
                    val atLeast = if (peek().kind == "PLUS") { advance(); true } else false
                    SeparatedRepetition(body, sep, atLeast)
                } else {
                    expect("RBRACE")
                    if (peek().kind == "PLUS") { advance(); OneOrMoreRepetition(body) }
                    else Repetition(body)
                }
            }
            "LBRACKET" -> { advance(); val body = parseBody(); expect("RBRACKET"); Optional(body) }
            "LPAREN" -> { advance(); val body = parseBody(); expect("RPAREN"); Group(body) }
            else -> throw ParserGrammarError("Unexpected token ${tok.kind}", tok.line)
        }
    }
}

fun parseParserGrammar(source: String): ParserGrammar {
    val grammar = ParserGrammar()
    val magicRe = Regex("""^#\s*@(\w+)\s*(.*)$""")
    for (line in source.split("\n")) {
        val stripped = line.trim()
        if (!stripped.startsWith("#")) continue
        magicRe.matchEntire(stripped)?.let { m ->
            if (m.groupValues[1] == "version") m.groupValues[2].trim().toIntOrNull()?.let { grammar.version = it }
        }
    }
    val tokens = tokenizeGrammar(source)
    grammar.rules.addAll(GrammarParserImpl(tokens).parseRules())
    return grammar
}

// ---------------------------------------------------------------------------
// Validators
// ---------------------------------------------------------------------------

private val SYNTHETIC_TOKENS = setOf("NEWLINE", "INDENT", "DEDENT", "EOF")

fun validateParserGrammar(grammar: ParserGrammar, tokenNames: Set<String>? = null): List<String> {
    val issues = mutableListOf<String>()
    val defined = grammar.ruleNames()
    val refRules = grammar.ruleReferences()
    val refTokens = grammar.tokenReferences()

    val seen = mutableMapOf<String, Int>()
    for (rule in grammar.rules) {
        seen[rule.name]?.let {
            issues.add("Line ${rule.lineNumber}: Duplicate rule name '${rule.name}' (first on line $it)")
        } ?: run { seen[rule.name] = rule.lineNumber }
        if (rule.name != rule.name.lowercase()) issues.add("Line ${rule.lineNumber}: Rule name '${rule.name}' should be lowercase")
    }
    for (ref in refRules.sorted()) {
        if (ref !in defined) issues.add("Undefined rule reference: '$ref'")
    }
    tokenNames?.let {
        for (ref in refTokens.sorted()) {
            if (ref !in it && ref !in SYNTHETIC_TOKENS) issues.add("Undefined token reference: '$ref'")
        }
    }
    if (grammar.rules.isNotEmpty()) {
        val start = grammar.rules[0].name
        for (rule in grammar.rules) {
            if (rule.name != start && rule.name !in refRules)
                issues.add("Line ${rule.lineNumber}: Rule '${rule.name}' is defined but never referenced (unreachable)")
        }
    }
    return issues
}

// ---------------------------------------------------------------------------
// Cross-validator
// ---------------------------------------------------------------------------

fun crossValidate(tokenGrammar: TokenGrammar, parserGrammar: ParserGrammar): List<String> {
    val issues = mutableListOf<String>()
    val definedTokens = tokenGrammar.tokenNames().toMutableSet()
    definedTokens.addAll(listOf("NEWLINE", "EOF"))
    if (tokenGrammar.mode == "indentation") definedTokens.addAll(listOf("INDENT", "DEDENT"))
    val refTokens = parserGrammar.tokenReferences()

    for (ref in refTokens.sorted()) {
        if (ref !in definedTokens) issues.add("Error: Grammar references token '$ref' which is not defined in the tokens file")
    }
    for (defn in tokenGrammar.definitions) {
        val isUsed = defn.name in refTokens || (defn.alias != null && defn.alias in refTokens)
        if (!isUsed) issues.add("Warning: Token '${defn.name}' (line ${defn.lineNumber}) is defined but never used in the grammar")
    }
    return issues
}
