package com.codingadventures.vhdllexer;

import com.codingadventures.grammartools.TokenGrammar;
import com.codingadventures.grammartools.TokenGrammarError;
import com.codingadventures.grammartools.TokenGrammarParser;
import com.codingadventures.lexer.GrammarLexer;
import com.codingadventures.lexer.LexerError;
import com.codingadventures.lexer.Token;
import com.codingadventures.lexer.TokenType;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;

public final class VhdlLexer {
    public static final String DEFAULT_VERSION = "2008";
    public static final List<String> SUPPORTED_VERSIONS = List.of("1987", "1993", "2002", "2008", "2019");

    private static final Map<String, TokenGrammar> TOKEN_GRAMMARS = new ConcurrentHashMap<>();

    private VhdlLexer() {}

    public static GrammarLexer createVhdlLexer() {
        return createVhdlLexer(DEFAULT_VERSION);
    }

    public static GrammarLexer createVhdlLexer(String version) {
        return new GrammarLexer(loadTokenGrammar(version));
    }

    public static List<Token> tokenizeVhdl(String source) {
        return tokenizeVhdl(source, DEFAULT_VERSION);
    }

    public static List<Token> tokenizeVhdl(String source, String version) {
        TokenGrammar grammar = loadTokenGrammar(version);
        try {
            List<Token> tokens = new GrammarLexer(grammar).tokenize(source);
            return normalizeCase(tokens, new HashSet<>(grammar.getKeywords()));
        } catch (LexerError error) {
            throw new IllegalArgumentException("VHDL tokenization failed: " + error.getMessage(), error);
        }
    }

    private static List<Token> normalizeCase(List<Token> tokens, Set<String> keywords) {
        List<Token> normalized = new ArrayList<>(tokens.size());
        for (Token token : tokens) {
            boolean normalizeKeyword = token.getType() == TokenType.KEYWORD;
            boolean normalizeName = token.getType() == TokenType.GRAMMAR && "NAME".equals(token.getTypeName());

            if (!normalizeKeyword && !normalizeName) {
                normalized.add(token);
                continue;
            }

            String lowered = token.getValue().toLowerCase();
            TokenType normalizedType = normalizeKeyword ? TokenType.KEYWORD : token.getType();
            String normalizedTypeName = normalizeKeyword ? "KEYWORD" : token.getTypeName();

            if (normalizeName && keywords.contains(lowered)) {
                normalizedType = TokenType.KEYWORD;
                normalizedTypeName = "KEYWORD";
            }

            normalized.add(new Token(
                    normalizedType,
                    lowered,
                    token.getLine(),
                    token.getColumn(),
                    normalizedTypeName,
                    token.getFlags()
            ));
        }
        return normalized;
    }

    private static TokenGrammar loadTokenGrammar(String version) {
        String validated = validateVersion(version);
        return TOKEN_GRAMMARS.computeIfAbsent(validated, VhdlLexer::parseTokenGrammarResource);
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

    private static TokenGrammar parseTokenGrammarResource(String version) {
        try {
            return TokenGrammarParser.parse(readResource("vhdl" + version + ".tokens"));
        } catch (TokenGrammarError error) {
            throw new IllegalStateException("Failed to parse bundled VHDL token grammar for version " + version, error);
        }
    }

    private static String readResource(String resourceName) {
        try (InputStream stream = VhdlLexer.class.getClassLoader().getResourceAsStream(resourceName)) {
            if (stream == null) {
                throw new IllegalStateException("Missing bundled resource: " + resourceName);
            }
            return new String(stream.readAllBytes(), StandardCharsets.UTF_8);
        } catch (IOException error) {
            throw new IllegalStateException("Failed to read bundled resource: " + resourceName, error);
        }
    }
}
