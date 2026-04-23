package com.codingadventures.parser;

import com.codingadventures.directedgraph.Graph;
import com.codingadventures.grammartools.GrammarElement;
import com.codingadventures.grammartools.GrammarRule;
import com.codingadventures.grammartools.ParserGrammar;
import com.codingadventures.lexer.Token;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

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

    public ASTNode parse(List<Token> tokens) throws GrammarParseError {
        if (grammar.getRules().isEmpty()) {
            throw new GrammarParseError("No rules in grammar", tokens.get(tokens.size() - 1));
        }

        String startRule = grammar.getRules().get(0).name();
        ParseContext context = new ParseContext();
        MatchResult result = matchRule(startRule, tokens, 0, context, null);
        if (result == null) {
            throw new GrammarParseError("Failed to parse starting rule '" + startRule + "'", tokens.get(0));
        }

        return buildNode(startRule, result.children, tokens);
    }

    private record MemoEntry(List<Object> children, int endPos, boolean ok) {}
    private record MatchResult(List<Object> children, int endPos) {}

    private static final class ParseContext {
        private final Map<String, Map<Integer, MemoEntry>> memo = new HashMap<>();
        private final Graph dependencyGraph = new Graph();
    }

    private MatchResult matchRule(String ruleName, List<Token> tokens, int pos,
                                  ParseContext context, String callerState) {
        Map<Integer, MemoEntry> ruleMemo = context.memo.computeIfAbsent(ruleName, ignored -> new HashMap<>());
        MemoEntry cached = ruleMemo.get(pos);
        if (cached != null) {
            return cached.ok ? new MatchResult(cached.children, cached.endPos) : null;
        }

        String stateNode = stateNode(ruleName, pos);
        context.dependencyGraph.addNode(stateNode);
        boolean addedDependency = registerDependency(context.dependencyGraph, callerState, stateNode);
        if (callerState != null && !addedDependency) {
            ruleMemo.put(pos, new MemoEntry(null, pos, false));
            return null;
        }

        GrammarRule rule = ruleMap.get(ruleName);
        if (rule == null) {
            ruleMemo.put(pos, new MemoEntry(null, pos, false));
            return null;
        }

        try {
            MatchResult result = matchElement(rule.body(), tokens, pos, context, stateNode);
            if (result != null) {
                List<Object> wrapped = List.of(buildNode(ruleName, result.children, tokens));
                ruleMemo.put(pos, new MemoEntry(wrapped, result.endPos, true));
                return new MatchResult(wrapped, result.endPos);
            }

            ruleMemo.put(pos, new MemoEntry(null, pos, false));
            return null;
        } finally {
            if (callerState != null && addedDependency && context.dependencyGraph.hasEdge(callerState, stateNode)) {
                context.dependencyGraph.removeEdge(callerState, stateNode);
            }
        }
    }

    private MatchResult matchElement(GrammarElement element, List<Token> tokens, int pos,
                                     ParseContext context, String callerState) {
        return switch (element) {
            case GrammarElement.RuleReference ref -> {
                if (ref.isToken()) {
                    if (pos < tokens.size()) {
                        Token token = tokens.get(pos);
                        if (ref.name().equals(token.effectiveTypeName())) {
                            yield new MatchResult(List.of(token), pos + 1);
                        }
                    }
                    yield null;
                }
                yield matchRule(ref.name(), tokens, pos, context, callerState);
            }

            case GrammarElement.Literal literal -> {
                if (pos < tokens.size() && literal.value().equals(tokens.get(pos).getValue())) {
                    yield new MatchResult(List.of(tokens.get(pos)), pos + 1);
                }
                yield null;
            }

            case GrammarElement.Sequence sequence -> {
                List<Object> children = new ArrayList<>();
                int currentPos = pos;
                for (GrammarElement subElement : sequence.elements()) {
                    MatchResult result = matchElement(subElement, tokens, currentPos, context, callerState);
                    if (result == null) {
                        yield null;
                    }
                    children.addAll(result.children);
                    currentPos = result.endPos;
                }
                yield new MatchResult(children, currentPos);
            }

            case GrammarElement.Alternation alternation -> {
                for (GrammarElement choice : alternation.choices()) {
                    MatchResult result = matchElement(choice, tokens, pos, context, callerState);
                    if (result != null) {
                        yield result;
                    }
                }
                yield null;
            }

            case GrammarElement.Repetition repetition -> {
                List<Object> children = new ArrayList<>();
                int currentPos = pos;
                while (true) {
                    MatchResult result = matchElement(repetition.element(), tokens, currentPos, context, callerState);
                    if (result == null || result.endPos == currentPos) {
                        break;
                    }
                    children.addAll(result.children);
                    currentPos = result.endPos;
                }
                yield new MatchResult(children, currentPos);
            }

            case GrammarElement.OneOrMoreRepetition repetition -> {
                MatchResult first = matchElement(repetition.element(), tokens, pos, context, callerState);
                if (first == null) {
                    yield null;
                }
                List<Object> children = new ArrayList<>(first.children);
                int currentPos = first.endPos;
                while (true) {
                    MatchResult result = matchElement(repetition.element(), tokens, currentPos, context, callerState);
                    if (result == null || result.endPos == currentPos) {
                        break;
                    }
                    children.addAll(result.children);
                    currentPos = result.endPos;
                }
                yield new MatchResult(children, currentPos);
            }

            case GrammarElement.Optional optional -> {
                MatchResult result = matchElement(optional.element(), tokens, pos, context, callerState);
                yield result != null ? result : new MatchResult(List.of(), pos);
            }

            case GrammarElement.Group group ->
                    matchElement(group.element(), tokens, pos, context, callerState);

            case GrammarElement.PositiveLookahead lookahead -> {
                MatchResult result = matchElement(lookahead.element(), tokens, pos, context, callerState);
                yield result != null ? new MatchResult(List.of(), pos) : null;
            }

            case GrammarElement.NegativeLookahead lookahead -> {
                MatchResult result = matchElement(lookahead.element(), tokens, pos, context, callerState);
                yield result == null ? new MatchResult(List.of(), pos) : null;
            }

            case GrammarElement.SeparatedRepetition separatedRepetition -> {
                List<Object> children = new ArrayList<>();
                MatchResult first = matchElement(separatedRepetition.element(), tokens, pos, context, callerState);
                if (first == null) {
                    yield separatedRepetition.atLeastOne() ? null : new MatchResult(List.of(), pos);
                }

                children.addAll(first.children);
                int currentPos = first.endPos;
                while (true) {
                    MatchResult separatorMatch = matchElement(
                            separatedRepetition.separator(), tokens, currentPos, context, callerState);
                    if (separatorMatch == null) {
                        break;
                    }

                    MatchResult elementMatch = matchElement(
                            separatedRepetition.element(), tokens, separatorMatch.endPos, context, callerState);
                    if (elementMatch == null) {
                        break;
                    }

                    children.addAll(separatorMatch.children);
                    children.addAll(elementMatch.children);
                    currentPos = elementMatch.endPos;
                }
                yield new MatchResult(children, currentPos);
            }
        };
    }

    private static boolean registerDependency(Graph dependencyGraph, String callerState, String calleeState) {
        if (callerState == null) {
            return true;
        }
        if (callerState.equals(calleeState)) {
            return false;
        }
        if (dependencyGraph.transitiveClosure(calleeState).contains(callerState)) {
            return false;
        }
        if (!dependencyGraph.hasEdge(callerState, calleeState)) {
            dependencyGraph.addEdge(callerState, calleeState);
        }
        return true;
    }

    private static String stateNode(String ruleName, int pos) {
        return ruleName + "@" + pos;
    }

    private ASTNode buildNode(String ruleName, List<Object> children, List<Token> tokens) {
        Token firstToken = findFirstToken(children);
        Token lastToken = findLastToken(children);

        return new ASTNode(
                ruleName,
                children,
                firstToken != null ? firstToken.getLine() : 0,
                firstToken != null ? firstToken.getColumn() : 0,
                lastToken != null ? lastToken.getLine() : 0,
                lastToken != null ? lastToken.getColumn() : 0
        );
    }

    private Token findFirstToken(List<Object> children) {
        for (Object child : children) {
            if (child instanceof Token token) {
                return token;
            }
            if (child instanceof ASTNode node) {
                Token nested = findFirstToken(node.getChildren());
                if (nested != null) {
                    return nested;
                }
            }
        }
        return null;
    }

    private Token findLastToken(List<Object> children) {
        for (int index = children.size() - 1; index >= 0; index--) {
            Object child = children.get(index);
            if (child instanceof Token token) {
                return token;
            }
            if (child instanceof ASTNode node) {
                Token nested = findLastToken(node.getChildren());
                if (nested != null) {
                    return nested;
                }
            }
        }
        return null;
    }
}
