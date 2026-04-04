// ============================================================================
// ASTWalker.swift — Utilities for traversing and querying AST trees.
// ============================================================================
//
// After parsing produces an AST, you often need to find specific nodes,
// collect all tokens, or transform the tree. This module provides utility
// functions for common AST traversal patterns.
//
// ============================================================================
// TRAVERSAL PATTERNS
// ============================================================================
//
// There are three main patterns for working with ASTs:
//
// 1. **Walking** -- Visit every node in the tree, calling a function at each.
//    Useful for printing, counting, or collecting information.
//
// 2. **Finding** -- Search the tree for nodes matching a predicate. Returns
//    a flat list of matching nodes. Useful for finding all function calls,
//    variable declarations, etc.
//
// 3. **Collecting** -- Gather all leaf tokens from a subtree. Useful for
//    reconstructing source text or extracting identifiers.
//
// All traversals use depth-first pre-order: visit the node first, then
// recurse into children left-to-right. This matches the source order of
// the code, so collected tokens appear in the same order as the original.
//
// ============================================================================

import Lexer

// ---------------------------------------------------------------------------
// MARK: - Walk AST
// ---------------------------------------------------------------------------

/// Walk an AST tree depth-first, calling a visitor function at each node.
///
/// The visitor is called with each `ASTNode` in the tree, starting from
/// the root. Children are visited left-to-right (source order).
///
/// Example: print all rule names in an AST:
///
///     walkAST(ast) { node in
///         print("Visited rule: \(node.ruleName)")
///     }
///
/// Example: count nodes by rule name:
///
///     var counts: [String: Int] = [:]
///     walkAST(ast) { node in
///         counts[node.ruleName, default: 0] += 1
///     }
///
/// - Parameters:
///   - node: The root node to start walking from.
///   - visitor: A function called for each node in the tree.
///
public func walkAST(_ node: ASTNode, visitor: (ASTNode) -> Void) {
    visitor(node)
    for child in node.children {
        if case .node(let childNode) = child {
            walkAST(childNode, visitor: visitor)
        }
    }
}

// ---------------------------------------------------------------------------
// MARK: - Find Nodes
// ---------------------------------------------------------------------------

/// Find all nodes in an AST that match a predicate.
///
/// Traverses the tree depth-first and collects nodes for which the
/// predicate returns true. The returned list is in source order.
///
/// Example: find all "expression" nodes:
///
///     let expressions = findNodes(in: ast) { $0.ruleName == "expression" }
///
/// Example: find all leaf nodes (single-token rules):
///
///     let leaves = findNodes(in: ast) { isLeafNode($0) }
///
/// - Parameters:
///   - node: The root node to search from.
///   - predicate: A function that returns true for matching nodes.
/// - Returns: An array of matching nodes in source order.
///
public func findNodes(
    in node: ASTNode,
    where predicate: (ASTNode) -> Bool
) -> [ASTNode] {
    var results: [ASTNode] = []
    walkAST(node) { current in
        if predicate(current) {
            results.append(current)
        }
    }
    return results
}

/// Find all nodes with a specific rule name.
///
/// Convenience wrapper around `findNodes(in:where:)` for the common case
/// of searching by rule name.
///
/// Example:
///
///     let funcDefs = findNodes(in: ast, named: "function_definition")
///
/// - Parameters:
///   - node: The root node to search from.
///   - ruleName: The rule name to match.
/// - Returns: An array of matching nodes in source order.
///
public func findNodes(in node: ASTNode, named ruleName: String) -> [ASTNode] {
    findNodes(in: node) { $0.ruleName == ruleName }
}

// ---------------------------------------------------------------------------
// MARK: - Collect Tokens
// ---------------------------------------------------------------------------

/// Collect all leaf tokens from an AST subtree in source order.
///
/// Walks the tree depth-first and gathers every token (leaf child),
/// skipping sub-nodes. The returned array preserves the original source
/// order, so joining the token values reconstructs the original text
/// (modulo whitespace).
///
/// Example: extract all tokens from an expression:
///
///     let tokens = collectTokens(from: exprNode)
///     let text = tokens.map(\.value).joined(separator: " ")
///
/// Example: find all identifiers in a subtree:
///
///     let identifiers = collectTokens(from: node)
///         .filter { $0.type == "NAME" }
///         .map(\.value)
///
/// - Parameter node: The root node to collect from.
/// - Returns: An array of tokens in source order.
///
public func collectTokens(from node: ASTNode) -> [Token] {
    var tokens: [Token] = []
    collectTokensRecursive(node, into: &tokens)
    return tokens
}

/// Internal recursive helper for collectTokens.
private func collectTokensRecursive(_ node: ASTNode, into tokens: inout [Token]) {
    for child in node.children {
        switch child {
        case .token(let tok):
            tokens.append(tok)
        case .node(let childNode):
            collectTokensRecursive(childNode, into: &tokens)
        }
    }
}
