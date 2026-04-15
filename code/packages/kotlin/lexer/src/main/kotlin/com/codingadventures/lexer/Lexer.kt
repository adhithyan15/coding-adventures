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
        return tokens
    }

    private fun compileDefinitions(defs: List<TokenDefinition>): List<CompiledPattern> =
        defs.map { defn ->
            val pattern = if (defn.isRegex) "\\G(?:${defn.pattern})" else "\\G${Regex.escape(defn.pattern)}"
            CompiledPattern(defn.name, Regex(pattern), defn.alias)
        }
}
