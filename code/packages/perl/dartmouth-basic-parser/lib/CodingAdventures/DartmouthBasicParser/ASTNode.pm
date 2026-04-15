package CodingAdventures::DartmouthBasicParser::ASTNode;

# ============================================================================
# CodingAdventures::DartmouthBasicParser::ASTNode — AST node for BASIC parser
# ============================================================================
#
# An ASTNode represents a single node in the Abstract Syntax Tree produced
# by the Dartmouth BASIC parser. Each node corresponds to either:
#
#   - An internal rule node: produced by a grammar rule (program, line,
#     statement, let_stmt, expr, etc.). Has a rule_name and zero or more
#     children.
#
#   - A leaf node: wraps a single lexer token (LINE_NUM, KEYWORD, NAME, etc.).
#     Has is_leaf == 1 and a token hashref.
#
# # AST structure for "10 LET X = 5\n"
#
#   ASTNode(rule_name="program")
#   └── ASTNode(rule_name="line")
#       ├── ASTNode(rule_name="token", is_leaf=1, token={type=LINE_NUM, value="10"})
#       ├── ASTNode(rule_name="statement")
#       │   └── ASTNode(rule_name="let_stmt")
#       │       ├── ASTNode(rule_name="token", is_leaf=1, token={type=KEYWORD, value="LET"})
#       │       ├── ASTNode(rule_name="variable")
#       │       │   └── ASTNode(rule_name="token", is_leaf=1, token={type=NAME, value="X"})
#       │       ├── ASTNode(rule_name="token", is_leaf=1, token={type=EQ, value="="})
#       │       └── ASTNode(rule_name="expr")   [→ term → power → unary → primary]
#       │           └── ...
#       └── ASTNode(rule_name="token", is_leaf=1, token={type=NEWLINE, value="\n"})
#
# # Why blessed hashrefs?
#
# We use Perl's `bless` mechanism to attach method dispatch to plain hashrefs.
# This is the idiomatic Perl OOP pattern for lightweight objects. The
# alternative — using Moose or Moo — would add a heavy dependency for a
# simple data structure.
#
# Construction:
#   CodingAdventures::DartmouthBasicParser::ASTNode->new(
#       rule_name => "program",
#       children  => [$child1, $child2],
#   )
#
#   CodingAdventures::DartmouthBasicParser::ASTNode->new(
#       rule_name => "token",
#       is_leaf   => 1,
#       token     => { type => "NUMBER", value => "42", line => 1, col => 8 },
#   )

use strict;
use warnings;

our $VERSION = '0.01';

# --- new(%args) ---------------------------------------------------------------
#
# Create a new ASTNode.
#
# Required key:
#   rule_name (string) — the grammar rule or "token" for leaves
#
# Optional keys:
#   children  (arrayref) — child ASTNodes; defaults to []
#   is_leaf   (boolean)  — 1 for token-wrapping leaf nodes; defaults to 0
#   token     (hashref)  — the wrapped token; only valid when is_leaf is 1

sub new {
    my ($class, %args) = @_;
    $args{children} //= [];
    $args{is_leaf}  //= 0;
    return bless \%args, $class;
}

# --- rule_name() --------------------------------------------------------------
#
# Return the grammar rule name for this node.
# Examples: "program", "line", "statement", "let_stmt", "expr", "token"

sub rule_name { $_[0]->{rule_name} }

# --- children() ---------------------------------------------------------------
#
# Return the arrayref of child ASTNodes.
# For leaf nodes this is always an empty arrayref.

sub children { $_[0]->{children} || [] }

# --- is_leaf() ----------------------------------------------------------------
#
# Return 1 if this node wraps a single token, 0 otherwise.

sub is_leaf { $_[0]->{is_leaf} || 0 }

# --- token() ------------------------------------------------------------------
#
# Return the wrapped token hashref (only meaningful when is_leaf() is true).
# Token fields: type, value, line, col.
#
# Token types for Dartmouth BASIC:
#   LINE_NUM   — line-number label at start of each numbered line
#   NUMBER     — numeric literal (in expressions)
#   STRING     — double-quoted string literal
#   KEYWORD    — reserved word (LET, PRINT, IF, etc.) with value field
#   BUILTIN_FN — built-in math function (SIN, COS, etc.)
#   USER_FN    — user-defined function name (FNA–FNZ)
#   NAME       — variable name (single letter or letter+digit)
#   EQ LT GT LE GE NE PLUS MINUS STAR SLASH CARET
#   LPAREN RPAREN COMMA SEMICOLON
#   NEWLINE    — statement terminator (significant in BASIC)
#   EOF        — end of input sentinel

sub token { $_[0]->{token} }

1;

__END__

=head1 NAME

CodingAdventures::DartmouthBasicParser::ASTNode - Abstract Syntax Tree node for the BASIC parser

=head1 SYNOPSIS

    use CodingAdventures::DartmouthBasicParser::ASTNode;

    # Internal rule node
    my $node = CodingAdventures::DartmouthBasicParser::ASTNode->new(
        rule_name => 'program',
        children  => [$child],
    );

    # Leaf (token-wrapping) node
    my $leaf = CodingAdventures::DartmouthBasicParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => { type => 'LINE_NUM', value => '10', line => 1, col => 1 },
    );

    print $node->rule_name;      # "program"
    print $leaf->is_leaf;        # 1
    print $leaf->token->{type};  # "LINE_NUM"

=head1 DESCRIPTION

A simple blessed-hashref AST node produced by
L<CodingAdventures::DartmouthBasicParser>.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
