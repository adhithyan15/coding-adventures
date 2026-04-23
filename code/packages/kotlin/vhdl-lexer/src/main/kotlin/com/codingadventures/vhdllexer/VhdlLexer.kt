package com.codingadventures.vhdllexer

import com.codingadventures.grammartools.TokenGrammar
import com.codingadventures.grammartools.parseTokenGrammar
import com.codingadventures.lexer.GrammarLexer
import com.codingadventures.lexer.Token
import com.codingadventures.lexer.TokenType
import java.nio.charset.StandardCharsets
import java.util.concurrent.ConcurrentHashMap

object VhdlLexer {
    const val DEFAULT_VERSION = "2008"
    val SUPPORTED_VERSIONS: List<String> = listOf("1987", "1993", "2002", "2008", "2019")

    private val tokenGrammars = ConcurrentHashMap<String, TokenGrammar>()

    fun createVhdlLexer(version: String = DEFAULT_VERSION): GrammarLexer =
        GrammarLexer(loadTokenGrammar(version))

    fun tokenizeVhdl(source: String, version: String = DEFAULT_VERSION): List<Token> {
        val grammar = loadTokenGrammar(version)
        val keywordSet = grammar.keywords.toSet()
        return GrammarLexer(grammar)
            .tokenize(source)
            .map { token -> normalizeToken(token, keywordSet) }
    }

    private fun normalizeToken(token: Token, keywords: Set<String>): Token {
        val normalizeKeyword = token.type == TokenType.KEYWORD
        val normalizeName = token.type == TokenType.GRAMMAR && token.typeName == "NAME"
        if (!normalizeKeyword && !normalizeName) {
            return token
        }

        val lowered = token.value.lowercase()
        return when {
            normalizeKeyword -> token.copy(value = lowered, typeName = "KEYWORD")
            normalizeName && lowered in keywords -> token.copy(type = TokenType.KEYWORD, value = lowered, typeName = "KEYWORD")
            else -> token.copy(value = lowered)
        }
    }

    private fun loadTokenGrammar(version: String): TokenGrammar {
        val validated = validateVersion(version)
        return tokenGrammars.computeIfAbsent(validated) { parseTokenGrammar(readResource("vhdl$it.tokens")) }
    }

    private fun validateVersion(version: String?): String {
        if (version.isNullOrBlank()) {
            return DEFAULT_VERSION
        }
        require(version in SUPPORTED_VERSIONS) {
            "Unknown VHDL version '$version'. Valid values: ${SUPPORTED_VERSIONS.joinToString(", ")}"
        }
        return version
    }

    private fun readResource(resourceName: String): String {
        val stream = javaClass.classLoader.getResourceAsStream(resourceName)
            ?: error("Missing bundled resource: $resourceName")
        return stream.use { String(it.readAllBytes(), StandardCharsets.UTF_8) }
    }
}
