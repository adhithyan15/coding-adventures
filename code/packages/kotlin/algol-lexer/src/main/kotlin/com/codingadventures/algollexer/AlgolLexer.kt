package com.codingadventures.algollexer

import com.codingadventures.grammartools.TokenGrammar
import com.codingadventures.grammartools.parseTokenGrammar
import com.codingadventures.lexer.GrammarLexer
import com.codingadventures.lexer.Token
import java.nio.charset.StandardCharsets
import java.util.concurrent.ConcurrentHashMap

object AlgolLexer {
    const val DEFAULT_VERSION = "algol60"
    val SUPPORTED_VERSIONS: List<String> = listOf("algol60")

    private val tokenGrammars = ConcurrentHashMap<String, TokenGrammar>()

    fun createAlgolLexer(version: String = DEFAULT_VERSION): GrammarLexer =
        GrammarLexer(loadTokenGrammar(version))

    fun tokenizeAlgol(source: String, version: String = DEFAULT_VERSION): List<Token> =
        createAlgolLexer(version).tokenize(source)

    private fun loadTokenGrammar(version: String): TokenGrammar {
        val validated = validateVersion(version)
        return tokenGrammars.computeIfAbsent(validated) { parseTokenGrammar(readResource("$it.tokens")) }
    }

    private fun validateVersion(version: String?): String {
        if (version.isNullOrBlank()) {
            return DEFAULT_VERSION
        }
        require(version in SUPPORTED_VERSIONS) {
            "Unknown ALGOL version '$version'. Valid values: ${SUPPORTED_VERSIONS.joinToString(", ")}"
        }
        return version
    }

    private fun readResource(resourceName: String): String {
        val stream = javaClass.classLoader.getResourceAsStream(resourceName)
            ?: error("Missing bundled resource: $resourceName")
        return stream.use { String(it.readAllBytes(), StandardCharsets.UTF_8) }
    }
}
