package com.codingadventures.parser

import com.codingadventures.directedgraph.Graph
import com.codingadventures.grammartools.Alternation
import com.codingadventures.grammartools.GrammarElement
import com.codingadventures.grammartools.GrammarRule
import com.codingadventures.grammartools.Group
import com.codingadventures.grammartools.Literal
import com.codingadventures.grammartools.NegativeLookahead
import com.codingadventures.grammartools.OneOrMoreRepetition
import com.codingadventures.grammartools.Optional
import com.codingadventures.grammartools.ParserGrammar
import com.codingadventures.grammartools.PositiveLookahead
import com.codingadventures.grammartools.Repetition
import com.codingadventures.grammartools.RuleReference
import com.codingadventures.grammartools.SeparatedRepetition
import com.codingadventures.grammartools.Sequence
import com.codingadventures.lexer.Token

data class ASTNode(
    val ruleName: String,
    val children: List<Any>,
    val startLine: Int = 0,
    val startColumn: Int = 0,
    val endLine: Int = 0,
    val endColumn: Int = 0
) {
    val isLeaf: Boolean get() = children.size == 1 && children[0] is Token
    val token: Token? get() = if (isLeaf) children[0] as Token else null

    fun descendantCount(): Int = children.sumOf { child ->
        1 + if (child is ASTNode) child.descendantCount() else 0
    }
}

class GrammarParseError(message: String, val token: Token) :
    Exception("Parse error at ${token.line}:${token.column}: $message")

private data class MemoEntry(val children: List<Any>?, val endPos: Int, val ok: Boolean)
private data class MatchResult(val children: List<Any>, val endPos: Int)
private data class ParseContext(
    val memo: MutableMap<String, MutableMap<Int, MemoEntry>> = mutableMapOf(),
    val dependencyGraph: Graph = Graph(),
)

class GrammarParser(private val grammar: ParserGrammar) {
    private val ruleMap: Map<String, GrammarRule> = grammar.rules.associateBy { it.name }

    fun parse(tokens: List<Token>): ASTNode {
        if (grammar.rules.isEmpty()) {
            throw GrammarParseError("No rules in grammar", tokens.last())
        }

        val startRule = grammar.rules[0].name
        val context = ParseContext()
        val result = matchRule(startRule, tokens, 0, context, null)
            ?: throw GrammarParseError("Failed to parse starting rule '$startRule'", tokens[0])

        return buildNode(startRule, result.children)
    }

    private fun matchRule(
        ruleName: String,
        tokens: List<Token>,
        pos: Int,
        context: ParseContext,
        callerState: String?,
    ): MatchResult? {
        val ruleMemo = context.memo.getOrPut(ruleName) { mutableMapOf() }
        ruleMemo[pos]?.let { cached ->
            return if (cached.ok) MatchResult(cached.children!!, cached.endPos) else null
        }

        val stateNode = stateNode(ruleName, pos)
        context.dependencyGraph.addNode(stateNode)
        val addedDependency = registerDependency(context.dependencyGraph, callerState, stateNode)
        if (callerState != null && !addedDependency) {
            ruleMemo[pos] = MemoEntry(null, pos, false)
            return null
        }

        val rule = ruleMap[ruleName]
        if (rule == null) {
            ruleMemo[pos] = MemoEntry(null, pos, false)
            return null
        }

        return try {
            val result = matchElement(rule.body, tokens, pos, context, stateNode)
            if (result != null) {
                val wrapped = listOf(buildNode(ruleName, result.children))
                ruleMemo[pos] = MemoEntry(wrapped, result.endPos, true)
                MatchResult(wrapped, result.endPos)
            } else {
                ruleMemo[pos] = MemoEntry(null, pos, false)
                null
            }
        } finally {
            if (callerState != null && addedDependency && context.dependencyGraph.hasEdge(callerState, stateNode)) {
                context.dependencyGraph.removeEdge(callerState, stateNode)
            }
        }
    }

    private fun matchElement(
        element: GrammarElement,
        tokens: List<Token>,
        pos: Int,
        context: ParseContext,
        callerState: String,
    ): MatchResult? = when (element) {
        is RuleReference -> if (element.isToken) {
            if (pos < tokens.size && element.name == tokens[pos].effectiveTypeName()) {
                MatchResult(listOf(tokens[pos]), pos + 1)
            } else {
                null
            }
        } else {
            matchRule(element.name, tokens, pos, context, callerState)
        }

        is Literal -> if (pos < tokens.size && element.value == tokens[pos].value) {
            MatchResult(listOf(tokens[pos]), pos + 1)
        } else {
            null
        }

        is Sequence -> {
            val children = mutableListOf<Any>()
            var currentPos = pos
            var failed = false
            for (subElement in element.elements) {
                val result = matchElement(subElement, tokens, currentPos, context, callerState)
                if (result == null) {
                    failed = true
                    break
                }
                children.addAll(result.children)
                currentPos = result.endPos
            }
            if (failed) null else MatchResult(children, currentPos)
        }

        is Alternation -> {
            var result: MatchResult? = null
            for (choice in element.choices) {
                result = matchElement(choice, tokens, pos, context, callerState)
                if (result != null) break
            }
            result
        }

        is Repetition -> {
            val children = mutableListOf<Any>()
            var currentPos = pos
            while (true) {
                val result = matchElement(element.element, tokens, currentPos, context, callerState) ?: break
                if (result.endPos == currentPos) break
                children.addAll(result.children)
                currentPos = result.endPos
            }
            MatchResult(children, currentPos)
        }

        is OneOrMoreRepetition -> {
            val first = matchElement(element.element, tokens, pos, context, callerState) ?: return null
            val children = first.children.toMutableList()
            var currentPos = first.endPos
            while (true) {
                val result = matchElement(element.element, tokens, currentPos, context, callerState) ?: break
                if (result.endPos == currentPos) break
                children.addAll(result.children)
                currentPos = result.endPos
            }
            MatchResult(children, currentPos)
        }

        is Optional -> matchElement(element.element, tokens, pos, context, callerState)
            ?: MatchResult(emptyList(), pos)

        is Group -> matchElement(element.element, tokens, pos, context, callerState)

        is PositiveLookahead -> if (matchElement(element.element, tokens, pos, context, callerState) != null) {
            MatchResult(emptyList(), pos)
        } else {
            null
        }

        is NegativeLookahead -> if (matchElement(element.element, tokens, pos, context, callerState) == null) {
            MatchResult(emptyList(), pos)
        } else {
            null
        }

        is SeparatedRepetition -> {
            val children = mutableListOf<Any>()
            val first = matchElement(element.element, tokens, pos, context, callerState)
            if (first == null) {
                if (element.atLeastOne) {
                    null
                } else {
                    MatchResult(emptyList(), pos)
                }
            } else {
                children.addAll(first.children)
                var currentPos = first.endPos
                while (true) {
                    val separatorMatch = matchElement(element.separator, tokens, currentPos, context, callerState) ?: break
                    val elementMatch = matchElement(element.element, tokens, separatorMatch.endPos, context, callerState) ?: break
                    children.addAll(separatorMatch.children)
                    children.addAll(elementMatch.children)
                    currentPos = elementMatch.endPos
                }
                MatchResult(children, currentPos)
            }
        }
    }

    private fun registerDependency(dependencyGraph: Graph, callerState: String?, calleeState: String): Boolean {
        if (callerState == null) return true
        if (callerState == calleeState) return false
        if (callerState in dependencyGraph.transitiveClosure(calleeState)) return false
        if (!dependencyGraph.hasEdge(callerState, calleeState)) {
            dependencyGraph.addEdge(callerState, calleeState)
        }
        return true
    }

    private fun stateNode(ruleName: String, pos: Int): String = "$ruleName@$pos"

    private fun buildNode(ruleName: String, children: List<Any>): ASTNode {
        val first = findFirstToken(children)
        val last = findLastToken(children)
        return ASTNode(
            ruleName = ruleName,
            children = children,
            startLine = first?.line ?: 0,
            startColumn = first?.column ?: 0,
            endLine = last?.line ?: 0,
            endColumn = last?.column ?: 0,
        )
    }

    private fun findFirstToken(children: List<Any>): Token? {
        for (child in children) {
            if (child is Token) return child
            if (child is ASTNode) {
                val nested = findFirstToken(child.children)
                if (nested != null) return nested
            }
        }
        return null
    }

    private fun findLastToken(children: List<Any>): Token? {
        for (index in children.indices.reversed()) {
            val child = children[index]
            if (child is Token) return child
            if (child is ASTNode) {
                val nested = findLastToken(child.children)
                if (nested != null) return nested
            }
        }
        return null
    }
}
