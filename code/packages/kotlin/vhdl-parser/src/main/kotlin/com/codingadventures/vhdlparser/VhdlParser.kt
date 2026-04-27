package com.codingadventures.vhdlparser

import com.codingadventures.grammartools.ParserGrammar
import com.codingadventures.grammartools.parseParserGrammar
import com.codingadventures.parser.ASTNode
import com.codingadventures.parser.GrammarParser
import com.codingadventures.vhdllexer.VhdlLexer
import java.nio.charset.StandardCharsets
import java.util.concurrent.ConcurrentHashMap

object VhdlParser {
    const val DEFAULT_VERSION = VhdlLexer.DEFAULT_VERSION
    val SUPPORTED_VERSIONS: List<String> = VhdlLexer.SUPPORTED_VERSIONS

    private val parserGrammars = ConcurrentHashMap<String, ParserGrammar>()

    fun createVhdlParser(version: String = DEFAULT_VERSION): GrammarParser =
        GrammarParser(loadParserGrammar(version))

    fun parseVhdl(source: String, version: String = DEFAULT_VERSION): ASTNode =
        createVhdlParser(version).parse(VhdlLexer.tokenizeVhdl(source, version))

    private fun loadParserGrammar(version: String): ParserGrammar {
        val validated = validateVersion(version)
        return parserGrammars.computeIfAbsent(validated) { parseParserGrammar(readResource("vhdl$it.grammar")) }
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
