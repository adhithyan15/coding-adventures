package com.codingadventures.algolparser

import com.codingadventures.grammartools.ParserGrammar
import com.codingadventures.grammartools.parseParserGrammar
import com.codingadventures.parser.ASTNode
import com.codingadventures.parser.GrammarParser
import com.codingadventures.algollexer.AlgolLexer
import java.nio.charset.StandardCharsets
import java.util.concurrent.ConcurrentHashMap

object AlgolParser {
    const val DEFAULT_VERSION = AlgolLexer.DEFAULT_VERSION
    val SUPPORTED_VERSIONS: List<String> = AlgolLexer.SUPPORTED_VERSIONS

    private val parserGrammars = ConcurrentHashMap<String, ParserGrammar>()

    fun createAlgolParser(version: String = DEFAULT_VERSION): GrammarParser =
        GrammarParser(loadParserGrammar(version))

    fun parseAlgol(source: String, version: String = DEFAULT_VERSION): ASTNode =
        createAlgolParser(version).parse(AlgolLexer.tokenizeAlgol(source, version))

    private fun loadParserGrammar(version: String): ParserGrammar {
        val validated = validateVersion(version)
        return parserGrammars.computeIfAbsent(validated) { parseParserGrammar(readResource("$it.grammar")) }
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
