// ============================================================================
// Lexer.kt — Token types and grammar-driven lexer
// ============================================================================
//
// A lexer (also called a tokenizer or scanner) breaks source code into a
// stream of tokens. This file contains all the types and the GrammarLexer.
//
// Layer: TE (text/language layer)
// ============================================================================

package com.codingadventures.lexer

import com.codingadventures.grammartools.TokenDefinition
import com.codingadventures.grammartools.TokenGrammar

// ---------------------------------------------------------------------------
// Token types
// ---------------------------------------------------------------------------

enum class TokenType {
    NAME, NUMBER, STRING, KEYWORD,
    PLUS, MINUS, STAR, SLASH,
    EQUALS, EQUALS_EQUALS,
    LPAREN, RPAREN, COMMA, COLON, SEMICOLON,
    LBRACE, RBRACE, LBRACKET, RBRACKET,
    DOT, BANG, NEWLINE, EOF,
    GRAMMAR  // Grammar-driven token — actual type is in typeName
}

/** Bitmask flag: a line break appeared before this token. */
const val FLAG_PRECEDED_BY_NEWLINE = 1
/** Bitmask flag: this is a context-sensitive keyword. */
const val FLAG_CONTEXT_KEYWORD = 2

/**
 * An immutable token from source code.
 */
data class Token(
    val type: TokenType,
    val value: String,
    val line: Int,
    val column: Int,
    val typeName: String? = null,
    val flags: Int = 0
) {
    fun effectiveTypeName(): String = typeName ?: type.name
    fun hasFlag(flag: Int): Boolean = (flags and flag) != 0
    override fun toString() = "Token(${effectiveTypeName()}, \"$value\", $line:$column)"
}

// ---------------------------------------------------------------------------
// Exceptions
// ---------------------------------------------------------------------------

class LexerError(message: String, val line: Int, val column: Int) :
    Exception("Lexer error at $line:$column: $message")

// ---------------------------------------------------------------------------
// Compiled pattern
// ---------------------------------------------------------------------------

private data class CompiledPattern(val name: String, val regex: Regex, val alias: String?)

// ---------------------------------------------------------------------------
// Grammar-driven lexer
// ---------------------------------------------------------------------------

/**
 * A lexer driven by a [TokenGrammar]. Compiles token definitions into
 * regexes and tries them in priority order at each source position.
 */
class GrammarLexer(private val grammar: TokenGrammar) {

    private val patterns = compileDefinitions(grammar.definitions)
    private val skipPatterns = compileDefinitions(grammar.skipDefinitions)
    private val errorPatterns = compileDefinitions(grammar.errorDefinitions)
    private val keywordSet = grammar.keywords.toSet()
    private val reservedSet = grammar.reservedKeywords.toSet()
    private val contextKeywordSet = grammar.contextKeywords.toSet()
    private val layoutKeywordSet = grammar.layoutKeywords.toSet()

    /**
     * Tokenize source code into a list of tokens.
     * The returned list always ends with an EOF token.
     */
    fun tokenize(source: String): List<Token> {
        val working = if (grammar.caseSensitive) source else source.lowercase()
        val tokens = mutableListOf<Token>()
        var pos = 0
        var line = 1
        var column = 1
        var precededByNewline = false

        while (pos < working.length) {
            // Try skip patterns first
            var skipped = false
            for (sp in skipPatterns) {
                val m = sp.regex.find(working, pos)
                if (m != null && m.range.first == pos) {
                    for (ch in m.value) {
                        if (ch == '\n') { line++; column = 1; precededByNewline = true }
                        else column++
                    }
                    pos += m.value.length
                    skipped = true
                    break
                }
            }
            if (skipped) continue

            // Try token patterns
            var matched = false
            for (cp in patterns) {
                val m = cp.regex.find(working, pos)
                if (m != null && m.range.first == pos) {
                    val value = source.substring(pos, pos + m.value.length)
                    val typeName = cp.alias ?: cp.name

                    if (typeName == "NAME" && value in reservedSet)
                        throw LexerError("Reserved keyword '$value'", line, column)

                    var flags = 0
                    if (precededByNewline) flags = flags or FLAG_PRECEDED_BY_NEWLINE
                    val checkVal = if (grammar.caseSensitive) value else value.lowercase()
                    if (typeName == "NAME" && checkVal in contextKeywordSet)
                        flags = flags or FLAG_CONTEXT_KEYWORD

                    tokens.add(Token(TokenType.GRAMMAR, value, line, column, typeName, flags))
                    for (ch in value) {
                        if (ch == '\n') { line++; column = 1 } else column++
                    }
                    pos += value.length
                    matched = true
                    precededByNewline = false
                    break
                }
            }
            if (matched) continue

            // Try error recovery patterns
            var errorMatched = false
            for (ep in errorPatterns) {
                val m = ep.regex.find(working, pos)
                if (m != null && m.range.first == pos) {
                    val value = source.substring(pos, pos + m.value.length)
                    tokens.add(Token(TokenType.GRAMMAR, value, line, column, ep.alias ?: ep.name))
                    for (ch in value) {
                        if (ch == '\n') { line++; column = 1 } else column++
                    }
                    pos += value.length
                    errorMatched = true
                    break
                }
            }
            if (errorMatched) continue

            throw LexerError("Unexpected character '${source[pos]}'", line, column)
        }

        // Keyword promotion
        if (keywordSet.isNotEmpty()) {
            for (i in tokens.indices) {
                val t = tokens[i]
                if (t.typeName == "NAME") {
                    val checkVal = if (grammar.caseSensitive) t.value else t.value.lowercase()
                    if (checkVal in keywordSet) {
                        tokens[i] = Token(TokenType.KEYWORD, t.value, t.line, t.column, "KEYWORD", t.flags)
                    }
                }
            }
        }

        tokens.add(Token(TokenType.EOF, "", line, column, "EOF"))
        return if (grammar.mode == "layout") applyLayout(tokens) else tokens
    }

    private fun applyLayout(tokens: List<Token>): List<Token> {
        val result = mutableListOf<Token>()
        val layoutStack = mutableListOf<Int>()
        var pendingLayouts = 0
        var suppressDepth = 0

        for ((index, token) in tokens.withIndex()) {
            val typeName = token.effectiveTypeName()

            if (typeName == "NEWLINE") {
                result += token
                val nextToken = tokens.drop(index + 1).firstOrNull { it.effectiveTypeName() != "NEWLINE" }
                if (suppressDepth == 0 && nextToken != null) {
                    while (layoutStack.isNotEmpty() && nextToken.column < layoutStack.last()) {
                        result += Token(TokenType.GRAMMAR, "}", nextToken.line, nextToken.column, "VIRTUAL_RBRACE")
                        layoutStack.removeAt(layoutStack.lastIndex)
                    }
                    if (layoutStack.isNotEmpty() &&
                        nextToken.effectiveTypeName() != "EOF" &&
                        nextToken.value != "}" &&
                        nextToken.column == layoutStack.last()
                    ) {
                        result += Token(TokenType.GRAMMAR, ";", nextToken.line, nextToken.column, "VIRTUAL_SEMICOLON")
                    }
                }
                continue
            }

            if (typeName == "EOF") {
                while (layoutStack.isNotEmpty()) {
                    result += Token(TokenType.GRAMMAR, "}", token.line, token.column, "VIRTUAL_RBRACE")
                    layoutStack.removeAt(layoutStack.lastIndex)
                }
                result += token
                continue
            }

            if (pendingLayouts > 0) {
                if (token.value == "{") {
                    pendingLayouts -= 1
                } else {
                    repeat(pendingLayouts) {
                        layoutStack += token.column
                        result += Token(TokenType.GRAMMAR, "{", token.line, token.column, "VIRTUAL_LBRACE")
                    }
                    pendingLayouts = 0
                }
            }

            result += token

            if (!typeName.startsWith("VIRTUAL_")) {
                when (token.value) {
                    "(", "[", "{" -> suppressDepth += 1
                    ")", "]", "}" -> if (suppressDepth > 0) suppressDepth -= 1
                }
            }

            if (layoutKeywordSet.contains(token.value) || layoutKeywordSet.contains(token.value.lowercase())) {
                pendingLayouts += 1
            }
        }

        return result
    }

    private fun compileDefinitions(defs: List<TokenDefinition>): List<CompiledPattern> =
        defs.map { defn ->
            val pattern = if (defn.isRegex) "\\G(?:${defn.pattern})" else "\\G${Regex.escape(defn.pattern)}"
            CompiledPattern(defn.name, Regex(pattern), defn.alias)
        }
}
