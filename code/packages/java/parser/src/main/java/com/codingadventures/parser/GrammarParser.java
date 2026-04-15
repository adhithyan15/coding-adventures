// ============================================================================
// GrammarParser.java — Grammar-driven recursive descent parser
// ============================================================================
//
// This parser takes a token stream (from a GrammarLexer) and a ParserGrammar
// (from a .grammar file) and produces an AST. It implements a recursive
// descent parser with packrat memoization for each (rule, position) pair.
//
// How it works:
//
//   1. Build a rule lookup table from the ParserGrammar's rule list.
//
//   2. Start parsing from the first rule (the start symbol).
//
//   3. For each rule, recursively match the rule body against the token
//      stream. The body is a tree of GrammarElement nodes (Sequence,
//      Alternation, Repetition, etc.) that we match recursively.
//
//   4. Each successful match returns a list of children (ASTNode and Token
//      objects) and the new position in the token stream.
//
//   5. Each (rule, position) result is memoized so we never re-parse the
//      same rule at the same position twice (packrat optimization).
//
// EBNF construct matching:
//
//   RuleReference (lowercase) — recursively parse the named rule
//   RuleReference (UPPERCASE) — match a token with that type name
//   Literal                   — match a token with that exact value
//   Sequence                  — match all elements in order
//   Alternation               — try each choice, take first that succeeds
//   Repetition                — match zero or more times
//   OneOrMoreRepetition       — match one or more times
//   Optional                  — match zero or one time
//   Group                     — match inner element (parenthesized grouping)
//   PositiveLookahead         — succeed if element matches (no consumption)
//   NegativeLookahead         — succeed if element does NOT match
//   SeparatedRepetition       — match with separator between elements
//
// Layer: TE (text/language layer)
// ============================================================================

package com.codingadventures.parser;

import com.codingadventures.grammartools.*;
import com.codingadventures.lexer.Token;

import java.util.*;

/**
 * A grammar-driven recursive descent parser with packrat memoization.
 */
public final class GrammarParser {

    private final ParserGrammar grammar;
    private final Map<String, GrammarRule> ruleMap;

    public GrammarParser(ParserGrammar grammar) {
        this.grammar = grammar;
        this.ruleMap = new HashMap<>();
        for (GrammarRule rule : grammar.getRules()) {
            ruleMap.put(rule.name(), rule);
        }
    }

    /**
     * Parse a token list starting from the grammar's first rule.
     *
     * @param tokens the token list (must end with an EOF token)
     * @return the root ASTNode
     * @throws GrammarParseError if parsing fails
     */
    public ASTNode parse(List<Token> tokens) throws GrammarParseError {
        if (grammar.getRules().isEmpty()) {
            throw new GrammarParseError("No rules in grammar", tokens.get(tokens.size() - 1));
        }

        String startRule = grammar.getRules().get(0).name();
        Map<String, Map<Integer, MemoEntry>> memo = new HashMap<>();

        MatchResult result = matchRule(startRule, tokens, 0, memo);
        if (result == null) {
            throw new GrammarParseError("Failed to parse starting rule '" + startRule + "'",
                    tokens.get(0));
        }

        return buildNode(startRule, result.children, tokens);
    }

    // =========================================================================
    // Internal matching
    // =========================================================================

    private record MemoEntry(List<Object> children, int endPos, boolean ok) {}
    private record MatchResult(List<Object> children, int endPos) {}

    private MatchResult matchRule(String ruleName, List<Token> tokens, int pos,
                                  Map<String, Map<Integer, MemoEntry>> memo) {
        // Check memo
        Map<Integer, MemoEntry> ruleMemo = memo.computeIfAbsent(ruleName, k -> new HashMap<>());
        MemoEntry cached = ruleMemo.get(pos);
        if (cached != null) {
            return cached.ok ? new MatchResult(cached.children, cached.endPos) : null;
        }

        GrammarRule rule = ruleMap.get(ruleName);
        if (rule == null) {
            ruleMemo.put(pos, new MemoEntry(null, pos, false));
            return null;
        }

        MatchResult result = matchElement(rule.body(), tokens, pos, memo);
        if (result != null) {
            // Wrap in a rule node
            List<Object> wrapped = List.of(buildNode(ruleName, result.children, tokens));
            ruleMemo.put(pos, new MemoEntry(wrapped, result.endPos, true));
            return new MatchResult(wrapped, result.endPos);
        }

        ruleMemo.put(pos, new MemoEntry(null, pos, false));
        return null;
    }

    private MatchResult matchElement(GrammarElement element, List<Token> tokens, int pos,
                                     Map<String, Map<Integer, MemoEntry>> memo) {
        return switch (element) {
            case GrammarElement.RuleReference ref -> {
                if (ref.isToken()) {
                    // Match a token by type name
                    if (pos < tokens.size()) {
                        Token tok = tokens.get(pos);
                        if (ref.name().equals(tok.effectiveTypeName())) {
                            yield new MatchResult(List.of(tok), pos + 1);
                        }
                    }
                    yield null;
                } else {
                    // Match a grammar rule
                    yield matchRule(ref.name(), tokens, pos, memo);
                }
            }

            case GrammarElement.Literal lit -> {
                if (pos < tokens.size() && lit.value().equals(tokens.get(pos).getValue())) {
                    yield new MatchResult(List.of(tokens.get(pos)), pos + 1);
                }
                yield null;
            }

            case GrammarElement.Sequence seq -> {
                List<Object> children = new ArrayList<>();
                int curPos = pos;
                for (GrammarElement sub : seq.elements()) {
                    MatchResult r = matchElement(sub, tokens, curPos, memo);
                    if (r == null) yield null;
                    children.addAll(r.children);
                    curPos = r.endPos;
                }
                yield new MatchResult(children, curPos);
            }

            case GrammarElement.Alternation alt -> {
                for (GrammarElement choice : alt.choices()) {
                    MatchResult r = matchElement(choice, tokens, pos, memo);
                    if (r != null) yield r;
                }
                yield null;
            }

            case GrammarElement.Repetition rep -> {
                List<Object> children = new ArrayList<>();
                int curPos = pos;
                while (true) {
                    MatchResult r = matchElement(rep.element(), tokens, curPos, memo);
                    if (r == null || r.endPos == curPos) break;
                    children.addAll(r.children);
                    curPos = r.endPos;
                }
                yield new MatchResult(children, curPos);
            }

            case GrammarElement.OneOrMoreRepetition rep -> {
                MatchResult first = matchElement(rep.element(), tokens, pos, memo);
                if (first == null) yield null;
                List<Object> children = new ArrayList<>(first.children);
                int curPos = first.endPos;
                while (true) {
                    MatchResult r = matchElement(rep.element(), tokens, curPos, memo);
                    if (r == null || r.endPos == curPos) break;
                    children.addAll(r.children);
                    curPos = r.endPos;
                }
                yield new MatchResult(children, curPos);
            }

            case GrammarElement.Optional opt -> {
                MatchResult r = matchElement(opt.element(), tokens, pos, memo);
                yield r != null ? r : new MatchResult(List.of(), pos);
            }

            case GrammarElement.Group grp ->
                matchElement(grp.element(), tokens, pos, memo);

            case GrammarElement.PositiveLookahead la -> {
                MatchResult r = matchElement(la.element(), tokens, pos, memo);
                yield r != null ? new MatchResult(List.of(), pos) : null;
            }

            case GrammarElement.NegativeLookahead la -> {
                MatchResult r = matchElement(la.element(), tokens, pos, memo);
                yield r == null ? new MatchResult(List.of(), pos) : null;
            }

            case GrammarElement.SeparatedRepetition sep -> {
                List<Object> children = new ArrayList<>();
                int curPos = pos;

                // First element
                MatchResult first = matchElement(sep.element(), tokens, curPos, memo);
                if (first == null) {
                    yield sep.atLeastOne() ? null : new MatchResult(List.of(), pos);
                }
                children.addAll(first.children);
                curPos = first.endPos;

                // Subsequent: separator then element
                while (true) {
                    MatchResult sepMatch = matchElement(sep.separator(), tokens, curPos, memo);
                    if (sepMatch == null) break;
                    MatchResult elemMatch = matchElement(sep.element(), tokens, sepMatch.endPos, memo);
                    if (elemMatch == null) break;
                    children.addAll(sepMatch.children);
                    children.addAll(elemMatch.children);
                    curPos = elemMatch.endPos;
                }
                yield new MatchResult(children, curPos);
            }
        };
    }

    private ASTNode buildNode(String ruleName, List<Object> children, List<Token> tokens) {
        int startLine = 0, startColumn = 0, endLine = 0, endColumn = 0;

        // Find position from first and last tokens in children
        Token firstToken = findFirstToken(children);
        Token lastToken = findLastToken(children);

        if (firstToken != null) {
            startLine = firstToken.getLine();
            startColumn = firstToken.getColumn();
        }
        if (lastToken != null) {
            endLine = lastToken.getLine();
            endColumn = lastToken.getColumn();
        }

        return new ASTNode(ruleName, children, startLine, startColumn, endLine, endColumn);
    }

    private Token findFirstToken(List<Object> children) {
        for (Object child : children) {
            if (child instanceof Token t) return t;
            if (child instanceof ASTNode node) {
                Token t = findFirstToken(node.getChildren());
                if (t != null) return t;
            }
        }
        return null;
    }

    private Token findLastToken(List<Object> children) {
        for (int i = children.size() - 1; i >= 0; i--) {
            Object child = children.get(i);
            if (child instanceof Token t) return t;
            if (child instanceof ASTNode node) {
                Token t = findLastToken(node.getChildren());
                if (t != null) return t;
            }
        }
        return null;
    }
}
