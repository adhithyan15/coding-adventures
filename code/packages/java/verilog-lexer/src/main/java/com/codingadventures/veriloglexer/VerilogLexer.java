package com.codingadventures.veriloglexer;

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

public final class VerilogLexer {
    public static final String DEFAULT_VERSION = "2005";
    public static final List<String> SUPPORTED_VERSIONS = List.of("1995", "2001", "2005");

    private static final Map<String, TokenGrammar> TOKEN_GRAMMARS = new ConcurrentHashMap<>();

    private VerilogLexer() {}

    public static GrammarLexer createVerilogLexer() {
        return createVerilogLexer(DEFAULT_VERSION);
    }

    public static GrammarLexer createVerilogLexer(String version) {
        return new GrammarLexer(loadTokenGrammar(version));
    }

    public static List<Token> tokenizeVerilog(String source) {
        return tokenizeVerilog(source, DEFAULT_VERSION);
    }

    public static List<Token> tokenizeVerilog(String source, String version) {
        try {
            return createVerilogLexer(version).tokenize(source);
        } catch (LexerError error) {
            throw new IllegalArgumentException("Verilog tokenization failed: " + error.getMessage(), error);
        }
    }

    private static TokenGrammar loadTokenGrammar(String version) {
        String validated = validateVersion(version);
        return TOKEN_GRAMMARS.computeIfAbsent(validated, VerilogLexer::parseTokenGrammarResource);
    }

    private static String validateVersion(String version) {
        if (version == null || version.isBlank()) {
            return DEFAULT_VERSION;
        }
        if (!SUPPORTED_VERSIONS.contains(version)) {
            throw new IllegalArgumentException(
                    "Unknown Verilog version '" + version + "'. Valid values: " + String.join(", ", SUPPORTED_VERSIONS)
            );
        }
        return version;
    }

    private static TokenGrammar parseTokenGrammarResource(String version) {
        try {
            return TokenGrammarParser.parse(readResource("verilog" + version + ".tokens"));
        } catch (TokenGrammarError error) {
            throw new IllegalStateException("Failed to parse bundled Verilog token grammar for version " + version, error);
        }
    }

    private static String readResource(String resourceName) {
        try (InputStream stream = VerilogLexer.class.getClassLoader().getResourceAsStream(resourceName)) {
            if (stream == null) {
                throw new IllegalStateException("Missing bundled resource: " + resourceName);
            }
            return new String(stream.readAllBytes(), StandardCharsets.UTF_8);
        } catch (IOException error) {
            throw new IllegalStateException("Failed to read bundled resource: " + resourceName, error);
        }
    }
}
