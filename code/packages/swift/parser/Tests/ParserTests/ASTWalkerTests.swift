// ============================================================================
// ASTWalkerTests.swift — Tests for AST traversal utilities.
// ============================================================================

import XCTest
import Lexer
import GrammarTools
@testable import Parser

final class ASTWalkerTests: XCTestCase {

    // -----------------------------------------------------------------------
    // MARK: - Test Helpers
    // -----------------------------------------------------------------------

    private func tok(_ type: String, _ value: String) -> Token {
        Token(type: type, value: value, line: 1, column: 1)
    }

    /// Build a simple test AST:
    ///
    ///     program
    ///       |-- expr
    ///       |     |-- NUMBER "1"
    ///       |     |-- PLUS "+"
    ///       |     |-- term
    ///       |           |-- NUMBER "2"
    ///       |-- SEMI ";"
    ///
    private func sampleAST() -> ASTNode {
        let termNode = ASTNode(
            ruleName: "term",
            children: [.token(tok("NUMBER", "2"))]
        )
        let exprNode = ASTNode(
            ruleName: "expr",
            children: [
                .token(tok("NUMBER", "1")),
                .token(tok("PLUS", "+")),
                .node(termNode),
            ]
        )
        return ASTNode(
            ruleName: "program",
            children: [
                .node(exprNode),
                .token(tok("SEMI", ";")),
            ]
        )
    }

    // -----------------------------------------------------------------------
    // MARK: - walkAST
    // -----------------------------------------------------------------------

    func testWalkASTVisitsAllNodes() {
        let ast = sampleAST()
        var visited: [String] = []

        walkAST(ast) { node in
            visited.append(node.ruleName)
        }

        XCTAssertEqual(visited, ["program", "expr", "term"])
    }

    func testWalkASTSingleNode() {
        let ast = ASTNode(
            ruleName: "leaf",
            children: [.token(tok("NUMBER", "42"))]
        )
        var count = 0

        walkAST(ast) { _ in count += 1 }

        XCTAssertEqual(count, 1)
    }

    // -----------------------------------------------------------------------
    // MARK: - findNodes
    // -----------------------------------------------------------------------

    func testFindNodesByPredicate() {
        let ast = sampleAST()
        let numberNodes = findNodes(in: ast) { node in
            node.children.contains { child in
                if case .token(let t) = child, t.type == "NUMBER" {
                    return true
                }
                return false
            }
        }

        // Both "expr" and "term" contain NUMBER tokens
        XCTAssertEqual(numberNodes.count, 2)
    }

    func testFindNodesByName() {
        let ast = sampleAST()
        let exprNodes = findNodes(in: ast, named: "expr")

        XCTAssertEqual(exprNodes.count, 1)
        XCTAssertEqual(exprNodes[0].ruleName, "expr")
    }

    func testFindNodesNoMatch() {
        let ast = sampleAST()
        let missing = findNodes(in: ast, named: "nonexistent")

        XCTAssertTrue(missing.isEmpty)
    }

    // -----------------------------------------------------------------------
    // MARK: - collectTokens
    // -----------------------------------------------------------------------

    func testCollectTokensSourceOrder() {
        let ast = sampleAST()
        let tokens = collectTokens(from: ast)

        XCTAssertEqual(tokens.count, 4)
        XCTAssertEqual(tokens[0].value, "1")
        XCTAssertEqual(tokens[1].value, "+")
        XCTAssertEqual(tokens[2].value, "2")
        XCTAssertEqual(tokens[3].value, ";")
    }

    func testCollectTokensLeafNode() {
        let ast = ASTNode(
            ruleName: "atom",
            children: [.token(tok("NUMBER", "42"))]
        )
        let tokens = collectTokens(from: ast)

        XCTAssertEqual(tokens.count, 1)
        XCTAssertEqual(tokens[0].value, "42")
    }

    func testCollectTokensEmpty() {
        let ast = ASTNode(ruleName: "empty", children: [])
        let tokens = collectTokens(from: ast)

        XCTAssertTrue(tokens.isEmpty)
    }

    // -----------------------------------------------------------------------
    // MARK: - ASTNode Helpers
    // -----------------------------------------------------------------------

    func testIsASTNode() {
        let child1: ASTChild = .node(ASTNode(ruleName: "test", children: []))
        let child2: ASTChild = .token(tok("NUM", "1"))

        XCTAssertTrue(isASTNode(child1))
        XCTAssertFalse(isASTNode(child2))
    }

    func testIsLeafToken() {
        let child1: ASTChild = .token(tok("NUM", "1"))
        let child2: ASTChild = .node(ASTNode(ruleName: "test", children: []))

        XCTAssertTrue(isLeafToken(child1))
        XCTAssertFalse(isLeafToken(child2))
    }

    func testIsLeafNode() {
        let leaf = ASTNode(
            ruleName: "factor",
            children: [.token(tok("NUMBER", "42"))]
        )
        let nonLeaf = ASTNode(
            ruleName: "expr",
            children: [
                .token(tok("NUMBER", "1")),
                .token(tok("PLUS", "+")),
            ]
        )

        XCTAssertTrue(isLeafNode(leaf))
        XCTAssertFalse(isLeafNode(nonLeaf))
    }

    func testGetLeafToken() {
        let leaf = ASTNode(
            ruleName: "factor",
            children: [.token(tok("NUMBER", "42"))]
        )
        let nonLeaf = ASTNode(
            ruleName: "expr",
            children: [
                .token(tok("NUMBER", "1")),
                .token(tok("PLUS", "+")),
            ]
        )

        XCTAssertEqual(getLeafToken(leaf)?.value, "42")
        XCTAssertNil(getLeafToken(nonLeaf))
    }

    // -----------------------------------------------------------------------
    // MARK: - Position Tracking
    // -----------------------------------------------------------------------

    func testPositionsComputedFromChildren() {
        let node = ASTNode(
            ruleName: "test",
            children: [
                .token(Token(type: "A", value: "a", line: 1, column: 1)),
                .token(Token(type: "B", value: "b", line: 3, column: 10)),
            ]
        )

        XCTAssertEqual(node.startLine, 1)
        XCTAssertEqual(node.startColumn, 1)
        XCTAssertEqual(node.endLine, 3)
        XCTAssertEqual(node.endColumn, 10)
    }

    func testPositionsEmptyChildren() {
        let node = ASTNode(ruleName: "empty", children: [])

        XCTAssertEqual(node.startLine, 0)
        XCTAssertEqual(node.startColumn, 0)
        XCTAssertEqual(node.endLine, 0)
        XCTAssertEqual(node.endColumn, 0)
    }

    func testPositionsFromSubNodes() {
        let inner = ASTNode(
            ruleName: "inner",
            children: [
                .token(Token(type: "X", value: "x", line: 5, column: 3)),
            ]
        )
        let outer = ASTNode(
            ruleName: "outer",
            children: [.node(inner)]
        )

        XCTAssertEqual(outer.startLine, 5)
        XCTAssertEqual(outer.startColumn, 3)
    }
}
