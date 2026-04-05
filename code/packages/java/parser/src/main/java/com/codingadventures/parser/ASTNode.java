// ============================================================================
// ASTNode.java — Generic AST node for grammar-driven parsing
// ============================================================================
//
// When a grammar-driven parser processes a token stream, it produces a tree
// of ASTNode objects. Each ASTNode represents either:
//
//   1. A rule match — a node with a rule name and child nodes/tokens
//   2. A leaf — a node wrapping a single Token from the lexer
//
// The tree structure mirrors the grammar rules. For example, parsing
// "1 + 2" with the grammar:
//
//   expression = term { PLUS term } ;
//   term = NUMBER ;
//
// Produces:
//
//   ASTNode("expression")
//     ASTNode("term")
//       Token(NUMBER, "1")
//     Token(PLUS, "+")
//     ASTNode("term")
//       Token(NUMBER, "2")
//
// Layer: TE (text/language layer)
// ============================================================================

package com.codingadventures.parser;

import com.codingadventures.lexer.Token;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * A generic AST node produced by grammar-driven parsing.
 *
 * <p>Children can be either other ASTNode instances or Token instances
 * from the lexer. Use {@link #isLeaf()} and {@link #getToken()} to check
 * for leaf nodes.
 */
public final class ASTNode {

    private final String ruleName;
    private final List<Object> children; // ASTNode or Token
    private final int startLine;
    private final int startColumn;
    private final int endLine;
    private final int endColumn;

    public ASTNode(String ruleName, List<Object> children,
                   int startLine, int startColumn, int endLine, int endColumn) {
        this.ruleName = ruleName;
        this.children = Collections.unmodifiableList(new ArrayList<>(children));
        this.startLine = startLine;
        this.startColumn = startColumn;
        this.endLine = endLine;
        this.endColumn = endColumn;
    }

    public ASTNode(String ruleName, List<Object> children) {
        this(ruleName, children, 0, 0, 0, 0);
    }

    public ASTNode(String ruleName) {
        this(ruleName, List.of());
    }

    public String getRuleName() { return ruleName; }
    public List<Object> getChildren() { return children; }
    public int getStartLine() { return startLine; }
    public int getStartColumn() { return startColumn; }
    public int getEndLine() { return endLine; }
    public int getEndColumn() { return endColumn; }

    /** True if this node wraps exactly one Token. */
    public boolean isLeaf() {
        return children.size() == 1 && children.get(0) instanceof Token;
    }

    /** Returns the leaf Token if isLeaf(), null otherwise. */
    public Token getToken() {
        return isLeaf() ? (Token) children.get(0) : null;
    }

    /** Count the total number of descendant nodes (for testing/debugging). */
    public int descendantCount() {
        int count = 0;
        for (Object child : children) {
            count++;
            if (child instanceof ASTNode node) {
                count += node.descendantCount();
            }
        }
        return count;
    }

    @Override
    public String toString() {
        return "ASTNode(" + ruleName + ", " + children.size() + " children)";
    }
}
