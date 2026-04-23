// ============================================================================
// ParserGrammarParser.java — Parser for .grammar files (EBNF syntax)
// ============================================================================
//
// A .grammar file describes a language's syntactic structure using EBNF:
//
//   program = { statement } ;
//   statement = assignment | expression_stmt ;
//   assignment = NAME EQUALS expression SEMI ;
//   expression = term { (PLUS | MINUS) term } ;
//   term = factor { (STAR | SLASH) factor } ;
//   factor = NUMBER | NAME | LPAREN expression RPAREN ;
//
// The parser works in two phases:
//
//   1. Tokenization — the grammar text is tokenized into a flat list of
//      internal tokens (IDENT, EQUALS, SEMI, PIPE, LBRACE, RBRACE, etc.)
//
//   2. Recursive descent parsing — the token list is parsed into a list of
//      GrammarRule objects, each with a name and an EBNF body.
//
// EBNF constructs supported:
//   - name = body ;           — rule definition
//   - A B C                   — sequence
//   - A | B                   — alternation
//   - { A }                   — zero or more repetition
//   - { A }+                  — one or more repetition
//   - { A // B }              — separated repetition
//   - [ A ]                   — optional
//   - ( A )                   — grouping
//   - &A                      — positive lookahead
//   - !A                      — negative lookahead
//   - "literal"               — string literal
//   - UPPERCASE               — token reference
//   - lowercase               — rule reference
//
// Layer: TE (text/language layer)
// ============================================================================

package com.codingadventures.grammartools;

import java.util.ArrayList;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Parses .grammar file text into a {@link ParserGrammar}.
 */
public final class ParserGrammarParser {

    private static final Pattern MAGIC_COMMENT = Pattern.compile("^#\\s*@(\\w+)\\s*(.*)$");

    private ParserGrammarParser() {}

    /**
     * Parse a .grammar file into a ParserGrammar.
     *
     * @param source the complete text of a .grammar file
     * @return the parsed ParserGrammar
     * @throws ParserGrammarError if the grammar text is malformed
     */
    public static ParserGrammar parse(String source) throws ParserGrammarError {
        ParserGrammar grammar = new ParserGrammar();

        // Scan for magic comments before tokenizing
        for (String rawLine : source.split("\n", -1)) {
            String stripped = rawLine.strip();
            if (!stripped.startsWith("#")) continue;
            Matcher m = MAGIC_COMMENT.matcher(stripped);
            if (m.matches()) {
                String key = m.group(1);
                String value = m.group(2).strip();
                if ("version".equals(key)) {
                    try { grammar.setVersion(Integer.parseInt(value)); }
                    catch (NumberFormatException ignored) {}
                }
            }
        }

        List<InternalToken> tokens = tokenize(source);
        Parser parser = new Parser(tokens);
        List<GrammarRule> rules = parser.parseRules();
        grammar.getRules().addAll(rules);
        return grammar;
    }

    // =========================================================================
    // Internal Token — A simple token for grammar file tokenization
    // =========================================================================

    private record InternalToken(String kind, String value, int line) {}

    // =========================================================================
    // Tokenizer — Converts grammar text into a flat token list
    // =========================================================================

    private static List<InternalToken> tokenize(String source) throws ParserGrammarError {
        List<InternalToken> tokens = new ArrayList<>();
        String[] lines = source.split("\n", -1);

        for (int i = 0; i < lines.length; i++) {
            int lineNum = i + 1;
            String line = lines[i].stripTrailing();
            String stripped = line.strip();
            if (stripped.isEmpty() || stripped.startsWith("#")) continue;

            int j = 0;
            while (j < line.length()) {
                char ch = line.charAt(j);
                if (ch == ' ' || ch == '\t') { j++; continue; }
                if (ch == '#') break; // inline comment

                switch (ch) {
                    case '=' -> { tokens.add(new InternalToken("EQUALS", "=", lineNum)); j++; }
                    case ';' -> { tokens.add(new InternalToken("SEMI", ";", lineNum)); j++; }
                    case '|' -> { tokens.add(new InternalToken("PIPE", "|", lineNum)); j++; }
                    case '{' -> { tokens.add(new InternalToken("LBRACE", "{", lineNum)); j++; }
                    case '}' -> { tokens.add(new InternalToken("RBRACE", "}", lineNum)); j++; }
                    case '[' -> { tokens.add(new InternalToken("LBRACKET", "[", lineNum)); j++; }
                    case ']' -> { tokens.add(new InternalToken("RBRACKET", "]", lineNum)); j++; }
                    case '(' -> { tokens.add(new InternalToken("LPAREN", "(", lineNum)); j++; }
                    case ')' -> { tokens.add(new InternalToken("RPAREN", ")", lineNum)); j++; }
                    case '&' -> { tokens.add(new InternalToken("AMPERSAND", "&", lineNum)); j++; }
                    case '!' -> { tokens.add(new InternalToken("BANG", "!", lineNum)); j++; }
                    case '+' -> { tokens.add(new InternalToken("PLUS", "+", lineNum)); j++; }
                    case '/' -> {
                        if (j + 1 < line.length() && line.charAt(j + 1) == '/') {
                            tokens.add(new InternalToken("DOUBLE_SLASH", "//", lineNum));
                            j += 2;
                        } else {
                            throw new ParserGrammarError("Unexpected character '/'", lineNum);
                        }
                    }
                    case '"' -> {
                        int k = j + 1;
                        while (k < line.length() && line.charAt(k) != '"') {
                            if (line.charAt(k) == '\\') k++;
                            k++;
                        }
                        if (k >= line.length()) {
                            throw new ParserGrammarError("Unterminated string literal", lineNum);
                        }
                        tokens.add(new InternalToken("STRING", line.substring(j + 1, k), lineNum));
                        j = k + 1;
                    }
                    default -> {
                        if (Character.isLetter(ch) || ch == '_') {
                            int k = j;
                            while (k < line.length() && (Character.isLetterOrDigit(line.charAt(k)) || line.charAt(k) == '_')) {
                                k++;
                            }
                            tokens.add(new InternalToken("IDENT", line.substring(j, k), lineNum));
                            j = k;
                        } else {
                            throw new ParserGrammarError("Unexpected character '" + ch + "'", lineNum);
                        }
                    }
                }
            }
        }
        tokens.add(new InternalToken("EOF", "", lines.length));
        return tokens;
    }

    // =========================================================================
    // Recursive Descent Parser
    // =========================================================================

    private static class Parser {
        private final List<InternalToken> tokens;
        private int pos = 0;

        Parser(List<InternalToken> tokens) {
            this.tokens = tokens;
        }

        private InternalToken peek() { return tokens.get(pos); }

        private InternalToken advance() { return tokens.get(pos++); }

        private InternalToken expect(String kind) throws ParserGrammarError {
            InternalToken tok = advance();
            if (!tok.kind().equals(kind)) {
                throw new ParserGrammarError("Expected " + kind + ", got " + tok.kind(), tok.line());
            }
            return tok;
        }

        List<GrammarRule> parseRules() throws ParserGrammarError {
            List<GrammarRule> rules = new ArrayList<>();
            while (!"EOF".equals(peek().kind())) {
                rules.add(parseRule());
            }
            return rules;
        }

        private GrammarRule parseRule() throws ParserGrammarError {
            InternalToken nameTok = expect("IDENT");
            expect("EQUALS");
            GrammarElement body = parseBody();
            expect("SEMI");
            return new GrammarRule(nameTok.value(), body, nameTok.line());
        }

        private GrammarElement parseBody() throws ParserGrammarError {
            GrammarElement first = parseSequence();
            List<GrammarElement> alternatives = new ArrayList<>();
            alternatives.add(first);

            while ("PIPE".equals(peek().kind())) {
                advance();
                alternatives.add(parseSequence());
            }

            return alternatives.size() == 1
                    ? alternatives.get(0)
                    : new GrammarElement.Alternation(List.copyOf(alternatives));
        }

        private GrammarElement parseSequence() throws ParserGrammarError {
            List<GrammarElement> elements = new ArrayList<>();
            while (true) {
                String kind = peek().kind();
                if ("PIPE".equals(kind) || "SEMI".equals(kind) || "RBRACE".equals(kind)
                        || "RBRACKET".equals(kind) || "RPAREN".equals(kind)
                        || "EOF".equals(kind) || "DOUBLE_SLASH".equals(kind)) {
                    break;
                }
                elements.add(parseElement());
            }
            if (elements.isEmpty()) {
                throw new ParserGrammarError("Expected at least one element in sequence", peek().line());
            }
            return elements.size() == 1 ? elements.get(0) : new GrammarElement.Sequence(List.copyOf(elements));
        }

        private GrammarElement parseElement() throws ParserGrammarError {
            InternalToken tok = peek();

            // Lookahead predicates
            if ("AMPERSAND".equals(tok.kind())) {
                advance();
                return new GrammarElement.PositiveLookahead(parseElement());
            }
            if ("BANG".equals(tok.kind())) {
                advance();
                return new GrammarElement.NegativeLookahead(parseElement());
            }

            switch (tok.kind()) {
                case "IDENT" -> {
                    advance();
                    boolean isToken = Character.isUpperCase(tok.value().charAt(0));
                    return new GrammarElement.RuleReference(tok.value(), isToken);
                }
                case "STRING" -> {
                    advance();
                    return new GrammarElement.Literal(tok.value());
                }
                case "LBRACE" -> {
                    advance();
                    GrammarElement body = parseBody();

                    // Separated repetition: { element // separator }
                    if ("DOUBLE_SLASH".equals(peek().kind())) {
                        advance();
                        GrammarElement separator = parseBody();
                        expect("RBRACE");
                        boolean atLeastOne = "PLUS".equals(peek().kind());
                        if (atLeastOne) advance();
                        return new GrammarElement.SeparatedRepetition(body, separator, atLeastOne);
                    }

                    expect("RBRACE");
                    // One-or-more: { element }+
                    if ("PLUS".equals(peek().kind())) {
                        advance();
                        return new GrammarElement.OneOrMoreRepetition(body);
                    }
                    return new GrammarElement.Repetition(body);
                }
                case "LBRACKET" -> {
                    advance();
                    GrammarElement body = parseBody();
                    expect("RBRACKET");
                    return new GrammarElement.Optional(body);
                }
                case "LPAREN" -> {
                    advance();
                    GrammarElement body = parseBody();
                    expect("RPAREN");
                    return new GrammarElement.Group(body);
                }
                default -> throw new ParserGrammarError("Unexpected token " + tok.kind(), tok.line());
            }
        }
    }
}
