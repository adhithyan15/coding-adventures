// ============================================================================
// GrammarLexer.java — Grammar-driven tokenizer
// ============================================================================
//
// Instead of hardcoding which characters map to which tokens, this lexer
// reads token definitions from a TokenGrammar (parsed from a .tokens file)
// and uses those definitions to drive tokenization at runtime.
//
// How it works:
//
//   1. Compile each TokenDefinition into a Java regex Pattern.
//      Literal patterns are escaped with Pattern.quote().
//      Regex patterns are anchored at the start with \A.
//
//   2. At each position in the source, try patterns in priority order
//      (first match wins). Skip patterns are tried first; if one matches,
//      the lexer consumes the matched text silently (no token produced).
//
//   3. When a definition has an alias (e.g. STRING_DQ -> STRING), the
//      emitted token uses the alias as its type name.
//
//   4. After tokenization, keywords in the grammar's keyword list are
//      promoted: a NAME token whose value matches a keyword gets its
//      type changed to KEYWORD.
//
//   5. If no pattern matches at a position, the lexer raises a LexerError.
//
// Layer: TE (text/language layer)
// ============================================================================

package com.codingadventures.lexer;

import com.codingadventures.grammartools.TokenDefinition;
import com.codingadventures.grammartools.TokenGrammar;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * A lexer that tokenizes source code using a {@link TokenGrammar}.
 *
 * <p>This is the runtime equivalent of tools like Lex/Flex: it takes a
 * declarative description of tokens and produces a token stream.
 */
public final class GrammarLexer {

    // A compiled pattern ready for matching
    private record CompiledPattern(String name, Pattern regex, String alias) {}

    private final TokenGrammar grammar;
    private final List<CompiledPattern> patterns;
    private final List<CompiledPattern> skipPatterns;
    private final List<CompiledPattern> errorPatterns;
    private final Set<String> keywordSet;
    private final Set<String> reservedSet;
    private final Set<String> contextKeywordSet;

    /**
     * Create a new GrammarLexer from a TokenGrammar.
     *
     * @param grammar the parsed .tokens file
     */
    public GrammarLexer(TokenGrammar grammar) {
        this.grammar = grammar;
        this.patterns = compileDefinitions(grammar.getDefinitions());
        this.skipPatterns = compileDefinitions(grammar.getSkipDefinitions());
        this.errorPatterns = compileDefinitions(grammar.getErrorDefinitions());
        this.keywordSet = new HashSet<>(grammar.getKeywords());
        this.reservedSet = new HashSet<>(grammar.getReservedKeywords());
        this.contextKeywordSet = new HashSet<>(grammar.getContextKeywords());
    }

    /**
     * Tokenize source code into a list of tokens.
     *
     * <p>The returned list always ends with an EOF token.
     *
     * @param source the source code text
     * @return list of tokens
     * @throws LexerError if source cannot be tokenized
     */
    public List<Token> tokenize(String source) throws LexerError {
        // Optionally lowercase source for case-insensitive languages
        String workingSource = grammar.isCaseSensitive() ? source : source.toLowerCase();

        List<Token> tokens = new ArrayList<>();
        int pos = 0;
        int line = 1;
        int column = 1;
        boolean precededByNewline = false;

        while (pos < workingSource.length()) {
            // --- Try skip patterns first ---
            boolean skipped = false;
            for (CompiledPattern sp : skipPatterns) {
                Matcher m = sp.regex.matcher(workingSource);
                m.region(pos, workingSource.length());
                if (m.lookingAt()) {
                    String matched = m.group();
                    // Track newlines in skipped content
                    for (char ch : matched.toCharArray()) {
                        if (ch == '\n') {
                            line++;
                            column = 1;
                            precededByNewline = true;
                        } else {
                            column++;
                        }
                    }
                    pos += matched.length();
                    skipped = true;
                    break;
                }
            }
            if (skipped) continue;

            // --- Try token patterns ---
            boolean matched = false;
            for (CompiledPattern cp : patterns) {
                Matcher m = cp.regex.matcher(workingSource);
                m.region(pos, workingSource.length());
                if (m.lookingAt()) {
                    // Use original source for the token value (preserve case)
                    String value = source.substring(pos, pos + m.group().length());
                    String typeName = cp.alias != null ? cp.alias : cp.name;

                    // Check for reserved keywords
                    if ("NAME".equals(typeName) && reservedSet.contains(value)) {
                        throw new LexerError("Reserved keyword '" + value + "'", line, column);
                    }

                    // Build flags
                    int flags = 0;
                    if (precededByNewline) flags |= Token.FLAG_PRECEDED_BY_NEWLINE;
                    if ("NAME".equals(typeName) && contextKeywordSet.contains(
                            grammar.isCaseSensitive() ? value : value.toLowerCase())) {
                        flags |= Token.FLAG_CONTEXT_KEYWORD;
                    }

                    Token token = new Token(TokenType.GRAMMAR, value, line, column, typeName, flags);
                    tokens.add(token);

                    // Advance position and track line/column
                    for (char ch : value.toCharArray()) {
                        if (ch == '\n') {
                            line++;
                            column = 1;
                        } else {
                            column++;
                        }
                    }
                    pos += value.length();
                    matched = true;
                    precededByNewline = false;
                    break;
                }
            }
            if (matched) continue;

            // --- Try error recovery patterns ---
            boolean errorMatched = false;
            for (CompiledPattern ep : errorPatterns) {
                Matcher m = ep.regex.matcher(workingSource);
                m.region(pos, workingSource.length());
                if (m.lookingAt()) {
                    String value = source.substring(pos, pos + m.group().length());
                    String typeName = ep.alias != null ? ep.alias : ep.name;
                    Token token = new Token(TokenType.GRAMMAR, value, line, column, typeName, 0);
                    tokens.add(token);

                    for (char ch : value.toCharArray()) {
                        if (ch == '\n') { line++; column = 1; }
                        else column++;
                    }
                    pos += value.length();
                    errorMatched = true;
                    break;
                }
            }
            if (errorMatched) continue;

            // No pattern matched
            throw new LexerError("Unexpected character '" + source.charAt(pos) + "'", line, column);
        }

        // Keyword promotion: NAME tokens whose values match keywords become KEYWORD
        promoteKeywords(tokens);

        // Add EOF token
        tokens.add(new Token(TokenType.EOF, "", line, column, "EOF", 0));
        return tokens;
    }

    /**
     * Promote NAME tokens whose values match keywords to KEYWORD type.
     */
    private void promoteKeywords(List<Token> tokens) {
        if (keywordSet.isEmpty()) return;
        for (int i = 0; i < tokens.size(); i++) {
            Token t = tokens.get(i);
            if ("NAME".equals(t.getTypeName())) {
                String checkValue = grammar.isCaseSensitive() ? t.getValue() : t.getValue().toLowerCase();
                if (keywordSet.contains(checkValue)) {
                    tokens.set(i, new Token(TokenType.KEYWORD, t.getValue(), t.getLine(), t.getColumn(),
                            "KEYWORD", t.getFlags()));
                }
            }
        }
    }

    /**
     * Compile a list of TokenDefinitions into regex patterns.
     */
    private static List<CompiledPattern> compileDefinitions(List<TokenDefinition> definitions) {
        List<CompiledPattern> result = new ArrayList<>();
        for (TokenDefinition defn : definitions) {
            String regexStr;
            if (defn.isRegex()) {
                // Anchor regex at current position with \G
                regexStr = "\\G(?:" + defn.getPattern() + ")";
            } else {
                // Escape literal and anchor
                regexStr = "\\G" + Pattern.quote(defn.getPattern());
            }
            Pattern regex = Pattern.compile(regexStr);
            result.add(new CompiledPattern(defn.getName(), regex, defn.getAlias()));
        }
        return result;
    }
}
