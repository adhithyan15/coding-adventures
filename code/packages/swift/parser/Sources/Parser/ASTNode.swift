// ============================================================================
// ASTNode.swift — Generic AST nodes produced by grammar-driven parsing.
// ============================================================================
//
// When the grammar-driven parser matches a grammar rule against the token
// stream, it produces an AST (Abstract Syntax Tree) node. Each node records:
//
// 1. Which grammar rule it matched (the `ruleName`).
// 2. What it matched (the `children` -- a mix of sub-nodes and tokens).
// 3. Where in the source it appeared (line/column positions).
//
// The AST is "generic" because it is not language-specific. The same node
// type is used whether you're parsing Python, Ruby, or any other language.
// The grammar rules determine the *shape* of the tree -- the AST just
// faithfully records what the parser matched.
//
// ============================================================================
// CHILDREN: THE ASTChild ENUM
// ============================================================================
//
// Each child of an AST node is either:
// - Another AST node (from matching a sub-rule), or
// - A token (from matching a token reference or literal).
//
// We represent this as an enum `ASTChild` with two cases:
//
//     .node(ASTNode)  -- a sub-tree from a rule match
//     .token(Token)   -- a leaf from a token match
//
// This is Swift's sum type -- the equivalent of TypeScript's `ASTNode | Token`
// discriminated union, but type-safe and exhaustive.
//
// ============================================================================
// POSITION TRACKING
// ============================================================================
//
// Each ASTNode carries start and end positions (line + column). These are
// computed from the first and last child's positions, giving every node in
// the tree a precise source range. This enables:
//
// - Error messages that point to the exact source location
// - Source maps for debugging
// - IDE features like "go to definition" and syntax highlighting
//
// ============================================================================

import Lexer

// ---------------------------------------------------------------------------
// MARK: - ASTChild
// ---------------------------------------------------------------------------

/// A child element of an AST node -- either a sub-node or a leaf token.
///
/// This enum is the Swift equivalent of TypeScript's `ASTNode | Token`
/// union type. Using an enum ensures exhaustive handling in switch
/// statements -- the compiler catches missing cases.
///
/// Example tree for `1 + 2`:
///
///     ASTNode("expression")
///       |-- ASTChild.token(Token(type: "NUMBER", value: "1"))
///       |-- ASTChild.token(Token(type: "PLUS", value: "+"))
///       |-- ASTChild.token(Token(type: "NUMBER", value: "2"))
///
/// Example tree for `1 + 2 * 3`:
///
///     ASTNode("expression")
///       |-- ASTChild.token(Token(type: "NUMBER", value: "1"))
///       |-- ASTChild.token(Token(type: "PLUS", value: "+"))
///       |-- ASTChild.node(ASTNode("term"))
///             |-- ASTChild.token(Token(type: "NUMBER", value: "2"))
///             |-- ASTChild.token(Token(type: "STAR", value: "*"))
///             |-- ASTChild.token(Token(type: "NUMBER", value: "3"))
///
public enum ASTChild: Sendable, Equatable {
    /// A sub-tree produced by matching a grammar rule.
    case node(ASTNode)

    /// A leaf token produced by matching a token reference or literal.
    case token(Token)
}

// ---------------------------------------------------------------------------
// MARK: - ASTNode
// ---------------------------------------------------------------------------

/// A generic AST node produced by grammar-driven parsing.
///
/// Each node represents a matched grammar rule. The `ruleName` tells you
/// which rule matched, and the `children` tell you what it matched.
///
/// Position fields record where in the source this node spans:
///   - `startLine` / `startColumn`: position of the first token
///   - `endLine` / `endColumn`: position of the last token
///
/// All positions are 1-based (matching text editor conventions).
///
public struct ASTNode: Sendable, Equatable {
    /// The name of the grammar rule that produced this node.
    ///
    /// For example, if the grammar has `expression = term { PLUS term } ;`
    /// then a node produced by matching this rule has `ruleName = "expression"`.
    ///
    public let ruleName: String

    /// The children of this node -- a mix of sub-nodes and tokens.
    ///
    /// Children are in source order: the first child corresponds to the
    /// leftmost element that was matched.
    ///
    public let children: [ASTChild]

    /// 1-based line number where this node starts (first token's line).
    public let startLine: Int

    /// 1-based column number where this node starts (first token's column).
    public let startColumn: Int

    /// 1-based line number where this node ends (last token's line).
    public let endLine: Int

    /// 1-based column number where this node ends (last token's column).
    public let endColumn: Int

    /// Create an ASTNode, auto-computing positions from children.
    ///
    /// The start position comes from the first child, and the end position
    /// from the last child. If there are no children, all positions are 0.
    ///
    /// This is the primary initializer -- most callers should use this one.
    /// The explicit-position initializer (`init(ruleName:children:startLine:...`)
    /// is only needed when you want to override the computed positions.
    ///
    public init(ruleName: String, children: [ASTChild]) {
        self.ruleName = ruleName
        self.children = children

        // Compute positions from children
        var sLine = 0, sCol = 0, eLine = 0, eCol = 0

        if let first = children.first {
            switch first {
            case .token(let tok):
                sLine = tok.line
                sCol = tok.column
            case .node(let node):
                sLine = node.startLine
                sCol = node.startColumn
            }
        }

        if let last = children.last {
            switch last {
            case .token(let tok):
                eLine = tok.line
                eCol = tok.column
            case .node(let node):
                eLine = node.endLine
                eCol = node.endColumn
            }
        }

        self.startLine = sLine
        self.startColumn = sCol
        self.endLine = eLine
        self.endColumn = eCol
    }

    /// Create an ASTNode with explicit position information.
    ///
    /// Use this when you need to override the auto-computed positions,
    /// for example in post-parse hooks or AST transforms.
    ///
    /// - Parameters:
    ///   - ruleName: The grammar rule name.
    ///   - children: The matched children (sub-nodes and tokens).
    ///   - startLine: 1-based start line.
    ///   - startColumn: 1-based start column.
    ///   - endLine: 1-based end line.
    ///   - endColumn: 1-based end column.
    ///
    public init(
        ruleName: String,
        children: [ASTChild],
        startLine: Int,
        startColumn: Int,
        endLine: Int,
        endColumn: Int
    ) {
        self.ruleName = ruleName
        self.children = children
        self.startLine = startLine
        self.startColumn = startColumn
        self.endLine = endLine
        self.endColumn = endColumn
    }
}

// ---------------------------------------------------------------------------
// MARK: - Helper Functions
// ---------------------------------------------------------------------------

/// Check if an `ASTChild` is an AST node (not a token).
///
/// This is the Swift equivalent of TypeScript's `isASTNode()` type guard.
///
/// - Parameter child: The child to check.
/// - Returns: True if the child is an `.node` case.
///
public func isASTNode(_ child: ASTChild) -> Bool {
    if case .node = child { return true }
    return false
}

/// Check if an `ASTChild` is a leaf token (not a sub-node).
///
/// - Parameter child: The child to check.
/// - Returns: True if the child is a `.token` case.
///
public func isLeafToken(_ child: ASTChild) -> Bool {
    if case .token = child { return true }
    return false
}

/// Check if an AST node is a leaf node (wraps a single token).
///
/// A leaf node has exactly one child, and that child is a token.
/// This typically occurs when a grammar rule matches a single token:
///
///     factor = NUMBER ;
///
/// The resulting ASTNode("factor") has one child: .token(NUMBER).
///
/// - Parameter node: The node to check.
/// - Returns: True if the node has exactly one child that is a token.
///
public func isLeafNode(_ node: ASTNode) -> Bool {
    node.children.count == 1 && isLeafToken(node.children[0])
}

/// Get the token from a leaf node, or nil for non-leaf nodes.
///
/// Convenience function for extracting the token from a single-token
/// node without manual pattern matching.
///
/// - Parameter node: The node to extract from.
/// - Returns: The token if this is a leaf node, nil otherwise.
///
public func getLeafToken(_ node: ASTNode) -> Token? {
    guard isLeafNode(node), case .token(let tok) = node.children[0] else {
        return nil
    }
    return tok
}
