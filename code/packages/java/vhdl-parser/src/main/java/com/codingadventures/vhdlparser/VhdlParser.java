package com.codingadventures.vhdlparser;

import com.codingadventures.grammartools.ParserGrammar;
import com.codingadventures.grammartools.ParserGrammarError;
import com.codingadventures.grammartools.ParserGrammarParser;
import com.codingadventures.parser.ASTNode;
import com.codingadventures.parser.GrammarParseError;
import com.codingadventures.parser.GrammarParser;
import com.codingadventures.vhdllexer.VhdlLexer;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

public final class VhdlParser {
    public static final String DEFAULT_VERSION = VhdlLexer.DEFAULT_VERSION;
    public static final List<String> SUPPORTED_VERSIONS = VhdlLexer.SUPPORTED_VERSIONS;

    private static final Map<String, ParserGrammar> PARSER_GRAMMARS = new ConcurrentHashMap<>();

    private VhdlParser() {}

    public static GrammarParser createVhdlParser() {
        return createVhdlParser(DEFAULT_VERSION);
    }

    public static GrammarParser createVhdlParser(String version) {
        return new GrammarParser(loadParserGrammar(version));
    }

    public static ASTNode parseVhdl(String source) {
        return parseVhdl(source, DEFAULT_VERSION);
    }

    public static ASTNode parseVhdl(String source, String version) {
        try {
            return createVhdlParser(version).parse(VhdlLexer.tokenizeVhdl(source, version));
        } catch (GrammarParseError error) {
            throw new IllegalArgumentException("VHDL parse failed: " + error.getMessage(), error);
        }
    }

    private static ParserGrammar loadParserGrammar(String version) {
        String validated = validateVersion(version);
        return PARSER_GRAMMARS.computeIfAbsent(validated, VhdlParser::parseParserGrammarResource);
    }

    private static String validateVersion(String version) {
        if (version == null || version.isBlank()) {
            return DEFAULT_VERSION;
        }
        if (!SUPPORTED_VERSIONS.contains(version)) {
            throw new IllegalArgumentException(
                    "Unknown VHDL version '" + version + "'. Valid values: " + String.join(", ", SUPPORTED_VERSIONS)
            );
        }
        return version;
    }

    private static ParserGrammar parseParserGrammarResource(String version) {
        try {
            return ParserGrammarParser.parse(readResource("vhdl" + version + ".grammar"));
        } catch (ParserGrammarError error) {
            throw new IllegalStateException("Failed to parse bundled VHDL parser grammar for version " + version, error);
        }
    }

    private static String readResource(String resourceName) {
        try (InputStream stream = VhdlParser.class.getClassLoader().getResourceAsStream(resourceName)) {
            if (stream == null) {
                throw new IllegalStateException("Missing bundled resource: " + resourceName);
            }
            return new String(stream.readAllBytes(), StandardCharsets.UTF_8);
        } catch (IOException error) {
            throw new IllegalStateException("Failed to read bundled resource: " + resourceName, error);
        }
    }
}
