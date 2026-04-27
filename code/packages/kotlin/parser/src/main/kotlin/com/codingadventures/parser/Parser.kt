// ============================================================================
// Parser.kt — Grammar-driven recursive descent parser with packrat memoization
// ============================================================================
//
// Takes a token stream and a ParserGrammar and produces an AST.
//
// Layer: TE (text/language layer)
// ============================================================================

package com.codingadventures.parser

import com.codingadventures.grammartools.*
import com.codingadventures.lexer.Token

// ---------------------------------------------------------------------------
// AST node
// ---------------------------------------------------------------------------

/**
 * A generic AST node produced by grammar-driven parsing.
 * Children can be either ASTNode or Token instances.
 */
data class ASTNode(
    val ruleName: String,
    val children: List<Any>,  // ASTNode or Token
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

// ---------------------------------------------------------------------------
// Memo entry
// ---------------------------------------------------------------------------

private data class MemoEntry(val children: List<Any>?, val endPos: Int, val ok: Boolean)
private data class MatchResult(val children: List<Any>, val endPos: Int)

// ---------------------------------------------------------------------------
// Grammar parser
// ---------------------------------------------------------------------------

class GrammarParser(private val grammar: ParserGrammar) {

    private val ruleMap: Map<String, GrammarRule> = grammar.rules.associateBy { it.name }

    fun parse(tokens: List<Token>): ASTNode {
        if (grammar.rules.isEmpty())
            throw GrammarParseError("No rules in grammar", tokens.last())

        val startRule = grammar.rules[0].name
        val memo = mutableMapOf<String, MutableMap<Int, MemoEntry>>()

        val result = matchRule(startRule, tokens, 0, memo)
            ?: throw GrammarParseError("Failed to parse starting rule '$startRule'", tokens[0])

        return buildNode(startRule, result.children, tokens)
    }

    private fun matchRule(
        ruleName: String, tokens: List<Token>, pos: Int,
        memo: MutableMap<String, MutableMap<Int, MemoEntry>>
    ): MatchResult? {
        val ruleMemo = memo.getOrPut(ruleName) { mutableMapOf() }
        ruleMemo[pos]?.let { cached ->
            return if (cached.ok) MatchResult(cached.children!!, cached.endPos) else null
        }

        val rule = ruleMap[ruleName]
        if (rule == null) {
            ruleMemo[pos] = MemoEntry(null, pos, false)
            return null
        }

        val result = matchElement(rule.body, tokens, pos, memo)
        return if (result != null) {
            val wrapped = listOf(buildNode(ruleName, result.children, tokens))
            ruleMemo[pos] = MemoEntry(wrapped, result.endPos, true)
            MatchResult(wrapped, result.endPos)
        } else {
            ruleMemo[pos] = MemoEntry(null, pos, false)
            null
        }
    }

    private fun matchElement(
        element: GrammarElement, tokens: List<Token>, pos: Int,
        memo: MutableMap<String, MutableMap<Int, MemoEntry>>
    ): MatchResult? = when (element) {
        is RuleReference -> if (element.isToken) {
            if (pos < tokens.size && element.name == tokens[pos].effectiveTypeName())
                MatchResult(listOf(tokens[pos]), pos + 1)
            else null
        } else matchRule(element.name, tokens, pos, memo)

        is Literal -> if (pos < tokens.size && element.value == tokens[pos].value)
            MatchResult(listOf(tokens[pos]), pos + 1) else null

        is Sequence -> {
            val children = mutableListOf<Any>()
            var curPos = pos
            var failed = false
            for (sub in element.elements) {
                val r = matchElement(sub, tokens, curPos, memo)
                if (r == null) { failed = true; break }
                children.addAll(r.children)
                curPos = r.endPos
            }
            if (failed) null else MatchResult(children, curPos)
        }

        is Alternation -> {
            var result: MatchResult? = null
            for (choice in element.choices) {
                result = matchElement(choice, tokens, pos, memo)
                if (result != null) break
            }
            result
        }

        is Repetition -> {
            val children = mutableListOf<Any>()
            var curPos = pos
            while (true) {
                val r = matchElement(element.element, tokens, curPos, memo) ?: break
                if (r.endPos == curPos) break
                children.addAll(r.children)
                curPos = r.endPos
            }
            MatchResult(children, curPos)
        }

        is OneOrMoreRepetition -> {
            val first = matchElement(element.element, tokens, pos, memo) ?: return null
            val children = first.children.toMutableList()
            var curPos = first.endPos
            while (true) {
                val r = matchElement(element.element, tokens, curPos, memo) ?: break
                if (r.endPos == curPos) break
                children.addAll(r.children)
                curPos = r.endPos
            }
            MatchResult(children, curPos)
        }

        is Optional -> matchElement(element.element, tokens, pos, memo)
            ?: MatchResult(emptyList(), pos)

        is Group -> matchElement(element.element, tokens, pos, memo)

        is PositiveLookahead -> if (matchElement(element.element, tokens, pos, memo) != null)
            MatchResult(emptyList(), pos) else null

        is NegativeLookahead -> if (matchElement(element.element, tokens, pos, memo) == null)
            MatchResult(emptyList(), pos) else null

        is SeparatedRepetition -> {
            val children = mutableListOf<Any>()
            val first = matchElement(element.element, tokens, pos, memo)
            if (first == null) {
                if (element.atLeastOne) null else MatchResult(emptyList(), pos)
            } else {
                children.addAll(first.children)
                var curPos = first.endPos
                while (true) {
                    val sepMatch = matchElement(element.separator, tokens, curPos, memo) ?: break
                    val elemMatch = matchElement(element.element, tokens, sepMatch.endPos, memo) ?: break
                    children.addAll(sepMatch.children)
                    children.addAll(elemMatch.children)
                    curPos = elemMatch.endPos
                }
                MatchResult(children, curPos)
            }
        }
    }

    private fun buildNode(ruleName: String, children: List<Any>, tokens: List<Token>): ASTNode {
        val first = findFirstToken(children)
        val last = findLastToken(children)
        return ASTNode(
            ruleName, children,
            first?.line ?: 0, first?.column ?: 0,
            last?.line ?: 0, last?.column ?: 0
        )
    }

    private fun findFirstToken(children: List<Any>): Token? {
        for (child in children) {
            if (child is Token) return child
            if (child is ASTNode) findFirstToken(child.children)?.let { return it }
        }
        return null
    }

    private fun findLastToken(children: List<Any>): Token? {
        for (i in children.indices.reversed()) {
            val child = children[i]
            if (child is Token) return child
            if (child is ASTNode) findLastToken(child.children)?.let { return it }
        }
        return null
    }
}
