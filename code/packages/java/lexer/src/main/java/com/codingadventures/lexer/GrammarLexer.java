package com.codingadventures.lexer;

import com.codingadventures.directedgraph.Graph;
import com.codingadventures.grammartools.TokenDefinition;
import com.codingadventures.grammartools.TokenGrammar;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public final class GrammarLexer {
    private enum MatcherStage {
        SKIP,
        TOKEN,
        ERROR
    }

    private record CompiledPattern(String name, Pattern regex, String alias) {}
    private record MatcherNode(MatcherStage stage, CompiledPattern pattern) {}

    private final TokenGrammar grammar;
    private final List<MatcherNode> matcherPipeline;
    private final Set<String> keywordSet;
    private final Set<String> reservedSet;
    private final Set<String> contextKeywordSet;

    public GrammarLexer(TokenGrammar grammar) {
        this.grammar = grammar;
        this.matcherPipeline = buildMatcherPipeline(grammar);
        this.keywordSet = new HashSet<>(grammar.getKeywords());
        this.reservedSet = new HashSet<>(grammar.getReservedKeywords());
        this.contextKeywordSet = new HashSet<>(grammar.getContextKeywords());
    }

    public List<Token> tokenize(String source) throws LexerError {
        String workingSource = grammar.isCaseSensitive() ? source : source.toLowerCase();

        List<Token> tokens = new ArrayList<>();
        int pos = 0;
        int line = 1;
        int column = 1;
        boolean precededByNewline = false;

        while (pos < workingSource.length()) {
            boolean matched = false;
            for (MatcherNode matcherNode : matcherPipeline) {
                Matcher matcher = matcherNode.pattern.regex.matcher(workingSource);
                matcher.region(pos, workingSource.length());
                if (!matcher.lookingAt()) {
                    continue;
                }

                String matchedValue = source.substring(pos, pos + matcher.group().length());
                switch (matcherNode.stage) {
                    case SKIP -> {
                        for (char ch : matchedValue.toCharArray()) {
                            if (ch == '\n') {
                                line++;
                                column = 1;
                                precededByNewline = true;
                            } else {
                                column++;
                            }
                        }
                        pos += matchedValue.length();
                    }

                    case TOKEN -> {
                        String typeName = matcherNode.pattern.alias != null
                                ? matcherNode.pattern.alias
                                : matcherNode.pattern.name;

                        if ("NAME".equals(typeName) && reservedSet.contains(matchedValue)) {
                            throw new LexerError("Reserved keyword '" + matchedValue + "'", line, column);
                        }

                        int flags = 0;
                        if (precededByNewline) {
                            flags |= Token.FLAG_PRECEDED_BY_NEWLINE;
                        }
                        if ("NAME".equals(typeName) && contextKeywordSet.contains(
                                grammar.isCaseSensitive() ? matchedValue : matchedValue.toLowerCase())) {
                            flags |= Token.FLAG_CONTEXT_KEYWORD;
                        }

                        tokens.add(new Token(TokenType.GRAMMAR, matchedValue, line, column, typeName, flags));

                        for (char ch : matchedValue.toCharArray()) {
                            if (ch == '\n') {
                                line++;
                                column = 1;
                            } else {
                                column++;
                            }
                        }
                        pos += matchedValue.length();
                        precededByNewline = false;
                    }

                    case ERROR -> {
                        String typeName = matcherNode.pattern.alias != null
                                ? matcherNode.pattern.alias
                                : matcherNode.pattern.name;
                        tokens.add(new Token(TokenType.GRAMMAR, matchedValue, line, column, typeName, 0));

                        for (char ch : matchedValue.toCharArray()) {
                            if (ch == '\n') {
                                line++;
                                column = 1;
                            } else {
                                column++;
                            }
                        }
                        pos += matchedValue.length();
                    }
                }

                matched = true;
                break;
            }

            if (matched) {
                continue;
            }

            throw new LexerError("Unexpected character '" + source.charAt(pos) + "'", line, column);
        }

        promoteKeywords(tokens);
        tokens.add(new Token(TokenType.EOF, "", line, column, "EOF", 0));
        return tokens;
    }

    private void promoteKeywords(List<Token> tokens) {
        if (keywordSet.isEmpty()) {
            return;
        }
        for (int index = 0; index < tokens.size(); index++) {
            Token token = tokens.get(index);
            if ("NAME".equals(token.getTypeName())) {
                String checkValue = grammar.isCaseSensitive() ? token.getValue() : token.getValue().toLowerCase();
                if (keywordSet.contains(checkValue)) {
                    tokens.set(index, new Token(TokenType.KEYWORD, token.getValue(), token.getLine(), token.getColumn(),
                            "KEYWORD", token.getFlags()));
                }
            }
        }
    }

    private static List<CompiledPattern> compileDefinitions(List<TokenDefinition> definitions) {
        List<CompiledPattern> result = new ArrayList<>();
        for (TokenDefinition definition : definitions) {
            String regexString = definition.isRegex()
                    ? "\\G(?:" + definition.getPattern() + ")"
                    : "\\G" + Pattern.quote(definition.getPattern());
            result.add(new CompiledPattern(
                    definition.getName(),
                    Pattern.compile(regexString),
                    definition.getAlias()
            ));
        }
        return result;
    }

    private static List<MatcherNode> buildMatcherPipeline(TokenGrammar grammar) {
        Graph pipelineGraph = new Graph();
        Map<String, MatcherNode> nodeMetadata = new HashMap<>();
        pipelineGraph.addNode("__start__");

        String previousNode = "__start__";
        previousNode = appendMatchers(previousNode, "skip", MatcherStage.SKIP,
                compileDefinitions(grammar.getSkipDefinitions()), pipelineGraph, nodeMetadata);
        previousNode = appendMatchers(previousNode, "token", MatcherStage.TOKEN,
                compileDefinitions(grammar.getDefinitions()), pipelineGraph, nodeMetadata);
        appendMatchers(previousNode, "error", MatcherStage.ERROR,
                compileDefinitions(grammar.getErrorDefinitions()), pipelineGraph, nodeMetadata);

        List<MatcherNode> orderedMatchers = new ArrayList<>();
        for (String nodeId : pipelineGraph.topologicalSort()) {
            MatcherNode node = nodeMetadata.get(nodeId);
            if (node != null) {
                orderedMatchers.add(node);
            }
        }
        return orderedMatchers;
    }

    private static String appendMatchers(String previousNode, String prefix, MatcherStage stage,
                                         List<CompiledPattern> patterns, Graph pipelineGraph,
                                         Map<String, MatcherNode> nodeMetadata) {
        String currentPrevious = previousNode;
        for (int index = 0; index < patterns.size(); index++) {
            String nodeId = prefix + ":" + index;
            pipelineGraph.addNode(nodeId);
            pipelineGraph.addEdge(currentPrevious, nodeId);
            nodeMetadata.put(nodeId, new MatcherNode(stage, patterns.get(index)));
            currentPrevious = nodeId;
        }
        return currentPrevious;
    }
}
