package com.codingadventures.algolparser;

import com.codingadventures.grammartools.ParserGrammar;
import com.codingadventures.grammartools.ParserGrammarError;
import com.codingadventures.grammartools.ParserGrammarParser;
import com.codingadventures.parser.ASTNode;
import com.codingadventures.parser.GrammarParseError;
import com.codingadventures.parser.GrammarParser;
import com.codingadventures.algollexer.AlgolLexer;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

public final class AlgolParser {
    public static final String DEFAULT_VERSION = AlgolLexer.DEFAULT_VERSION;
    public static final List<String> SUPPORTED_VERSIONS = AlgolLexer.SUPPORTED_VERSIONS;

    private static final Map<String, ParserGrammar> PARSER_GRAMMARS = new ConcurrentHashMap<>();

    private AlgolParser() {}

    public static GrammarParser createAlgolParser() {
        return createAlgolParser(DEFAULT_VERSION);
    }

    public static GrammarParser createAlgolParser(String version) {
        return new GrammarParser(loadParserGrammar(version));
    }

    public static ASTNode parseAlgol(String source) {
        return parseAlgol(source, DEFAULT_VERSION);
    }

    public static ASTNode parseAlgol(String source, String version) {
        try {
            return createAlgolParser(version).parse(AlgolLexer.tokenizeAlgol(source, version));
        } catch (GrammarParseError error) {
            throw new IllegalArgumentException("ALGOL parse failed: " + error.getMessage(), error);
        }
    }

    private static ParserGrammar loadParserGrammar(String version) {
        String validated = validateVersion(version);
        return PARSER_GRAMMARS.computeIfAbsent(validated, AlgolParser::parseParserGrammarResource);
    }

    private static String validateVersion(String version) {
        if (version == null || version.isBlank()) {
            return DEFAULT_VERSION;
        }
        if (!SUPPORTED_VERSIONS.contains(version)) {
            throw new IllegalArgumentException(
                    "Unknown ALGOL version '" + version + "'. Valid values: " + String.join(", ", SUPPORTED_VERSIONS)
            );
        }
        return version;
    }

    private static ParserGrammar parseParserGrammarResource(String version) {
        try {
            return ParserGrammarParser.parse(readResource(version + ".grammar"));
        } catch (ParserGrammarError error) {
            throw new IllegalStateException("Failed to parse bundled ALGOL parser grammar for version " + version, error);
        }
    }

    private static String readResource(String resourceName) {
        try (InputStream stream = AlgolParser.class.getClassLoader().getResourceAsStream(resourceName)) {
            if (stream == null) {
                throw new IllegalStateException("Missing bundled resource: " + resourceName);
            }
            return new String(stream.readAllBytes(), StandardCharsets.UTF_8);
        } catch (IOException error) {
            throw new IllegalStateException("Failed to read bundled resource: " + resourceName, error);
        }
    }
}
