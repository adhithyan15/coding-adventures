package com.codingadventures.lexer

import com.codingadventures.directedgraph.Graph
import com.codingadventures.grammartools.TokenDefinition
import com.codingadventures.grammartools.TokenGrammar

enum class TokenType {
    NAME, NUMBER, STRING, KEYWORD,
    PLUS, MINUS, STAR, SLASH,
    EQUALS, EQUALS_EQUALS,
    LPAREN, RPAREN, COMMA, COLON, SEMICOLON,
    LBRACE, RBRACE, LBRACKET, RBRACKET,
    DOT, BANG, NEWLINE, EOF,
    GRAMMAR
}

const val FLAG_PRECEDED_BY_NEWLINE = 1
const val FLAG_CONTEXT_KEYWORD = 2

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

class LexerError(message: String, val line: Int, val column: Int) :
    Exception("Lexer error at $line:$column: $message")

private data class CompiledPattern(val name: String, val regex: Regex, val alias: String?)
private enum class MatcherStage { SKIP, TOKEN, ERROR }
private data class MatcherNode(val stage: MatcherStage, val pattern: CompiledPattern)

class GrammarLexer(private val grammar: TokenGrammar) {
    private val matcherPipeline = buildMatcherPipeline(grammar)
    private val keywordSet = grammar.keywords.toSet()
    private val reservedSet = grammar.reservedKeywords.toSet()
    private val contextKeywordSet = grammar.contextKeywords.toSet()

    fun tokenize(source: String): List<Token> {
        val working = if (grammar.caseSensitive) source else source.lowercase()
        val tokens = mutableListOf<Token>()
        var pos = 0
        var line = 1
        var column = 1
        var precededByNewline = false

        while (pos < working.length) {
            var matched = false
            for (matcherNode in matcherPipeline) {
                val match = matcherNode.pattern.regex.find(working, pos)
                if (match == null || match.range.first != pos) continue

                val value = source.substring(pos, pos + match.value.length)
                when (matcherNode.stage) {
                    MatcherStage.SKIP -> {
                        for (ch in value) {
                            if (ch == '\n') {
                                line++
                                column = 1
                                precededByNewline = true
                            } else {
                                column++
                            }
                        }
                        pos += value.length
                    }

                    MatcherStage.TOKEN -> {
                        val typeName = matcherNode.pattern.alias ?: matcherNode.pattern.name
                        if (typeName == "NAME" && value in reservedSet) {
                            throw LexerError("Reserved keyword '$value'", line, column)
                        }

                        var flags = 0
                        if (precededByNewline) flags = flags or FLAG_PRECEDED_BY_NEWLINE
                        val checkValue = if (grammar.caseSensitive) value else value.lowercase()
                        if (typeName == "NAME" && checkValue in contextKeywordSet) {
                            flags = flags or FLAG_CONTEXT_KEYWORD
                        }

                        tokens += Token(TokenType.GRAMMAR, value, line, column, typeName, flags)
                        for (ch in value) {
                            if (ch == '\n') {
                                line++
                                column = 1
                            } else {
                                column++
                            }
                        }
                        pos += value.length
                        precededByNewline = false
                    }

                    MatcherStage.ERROR -> {
                        val typeName = matcherNode.pattern.alias ?: matcherNode.pattern.name
                        tokens += Token(TokenType.GRAMMAR, value, line, column, typeName)
                        for (ch in value) {
                            if (ch == '\n') {
                                line++
                                column = 1
                            } else {
                                column++
                            }
                        }
                        pos += value.length
                    }
                }

                matched = true
                break
            }

            if (matched) continue
            throw LexerError("Unexpected character '${source[pos]}'", line, column)
        }

        if (keywordSet.isNotEmpty()) {
            for (index in tokens.indices) {
                val token = tokens[index]
                if (token.typeName == "NAME") {
                    val checkValue = if (grammar.caseSensitive) token.value else token.value.lowercase()
                    if (checkValue in keywordSet) {
                        tokens[index] = Token(TokenType.KEYWORD, token.value, token.line, token.column, "KEYWORD", token.flags)
                    }
                }
            }
        }

        tokens += Token(TokenType.EOF, "", line, column, "EOF")
        return tokens
    }

    private fun compileDefinitions(definitions: List<TokenDefinition>): List<CompiledPattern> =
        definitions.map { definition ->
            val regexString = if (definition.isRegex) {
                "\\G(?:${definition.pattern})"
            } else {
                "\\G${Regex.escape(definition.pattern)}"
            }
            CompiledPattern(definition.name, Regex(regexString), definition.alias)
        }

    private fun buildMatcherPipeline(grammar: TokenGrammar): List<MatcherNode> {
        val pipelineGraph = Graph()
        val nodeMetadata = linkedMapOf<String, MatcherNode>()
        pipelineGraph.addNode("__start__")

        var previousNode = "__start__"
        previousNode = appendMatchers(previousNode, "skip", MatcherStage.SKIP, compileDefinitions(grammar.skipDefinitions), pipelineGraph, nodeMetadata)
        previousNode = appendMatchers(previousNode, "token", MatcherStage.TOKEN, compileDefinitions(grammar.definitions), pipelineGraph, nodeMetadata)
        appendMatchers(previousNode, "error", MatcherStage.ERROR, compileDefinitions(grammar.errorDefinitions), pipelineGraph, nodeMetadata)

        return pipelineGraph.topologicalSort().mapNotNull(nodeMetadata::get)
    }

    private fun appendMatchers(
        previousNode: String,
        prefix: String,
        stage: MatcherStage,
        patterns: List<CompiledPattern>,
        pipelineGraph: Graph,
        nodeMetadata: MutableMap<String, MatcherNode>,
    ): String {
        var currentPrevious = previousNode
        patterns.forEachIndexed { index, pattern ->
            val nodeId = "$prefix:$index"
            pipelineGraph.addNode(nodeId)
            pipelineGraph.addEdge(currentPrevious, nodeId)
            nodeMetadata[nodeId] = MatcherNode(stage, pattern)
            currentPrevious = nodeId
        }
        return currentPrevious
    }
}
