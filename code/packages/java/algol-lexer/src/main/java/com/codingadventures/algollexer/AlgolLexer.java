package com.codingadventures.algollexer;

import com.codingadventures.grammartools.TokenGrammar;
import com.codingadventures.grammartools.TokenGrammarError;
import com.codingadventures.grammartools.TokenGrammarParser;
import com.codingadventures.lexer.GrammarLexer;
import com.codingadventures.lexer.LexerError;
import com.codingadventures.lexer.Token;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

public final class AlgolLexer {
    public static final String DEFAULT_VERSION = "algol60";
    public static final List<String> SUPPORTED_VERSIONS = List.of("algol60");

    private static final Map<String, TokenGrammar> TOKEN_GRAMMARS = new ConcurrentHashMap<>();

    private AlgolLexer() {}

    public static GrammarLexer createAlgolLexer() {
        return createAlgolLexer(DEFAULT_VERSION);
    }

    public static GrammarLexer createAlgolLexer(String version) {
        return new GrammarLexer(loadTokenGrammar(version));
    }

    public static List<Token> tokenizeAlgol(String source) {
        return tokenizeAlgol(source, DEFAULT_VERSION);
    }

    public static List<Token> tokenizeAlgol(String source, String version) {
        try {
            return createAlgolLexer(version).tokenize(source);
        } catch (LexerError error) {
            throw new IllegalArgumentException("ALGOL tokenization failed: " + error.getMessage(), error);
        }
    }

    private static TokenGrammar loadTokenGrammar(String version) {
        String validated = validateVersion(version);
        return TOKEN_GRAMMARS.computeIfAbsent(validated, AlgolLexer::parseTokenGrammarResource);
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

    private static TokenGrammar parseTokenGrammarResource(String version) {
        try {
            return TokenGrammarParser.parse(readResource(version + ".tokens"));
        } catch (TokenGrammarError error) {
            throw new IllegalStateException("Failed to parse bundled ALGOL token grammar for version " + version, error);
        }
    }

    private static String readResource(String resourceName) {
        try (InputStream stream = AlgolLexer.class.getClassLoader().getResourceAsStream(resourceName)) {
            if (stream == null) {
                throw new IllegalStateException("Missing bundled resource: " + resourceName);
            }
            return new String(stream.readAllBytes(), StandardCharsets.UTF_8);
        } catch (IOException error) {
            throw new IllegalStateException("Failed to read bundled resource: " + resourceName, error);
        }
    }
}
