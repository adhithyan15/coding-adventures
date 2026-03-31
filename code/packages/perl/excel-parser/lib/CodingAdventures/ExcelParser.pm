package CodingAdventures::ExcelParser;

# ============================================================================
# CodingAdventures::ExcelParser — Hand-written recursive-descent Excel parser
# ============================================================================
#
# This module parses Excel formula strings into an Abstract Syntax Tree (AST).
# It uses a hand-written recursive-descent approach, matching the pattern
# established by CodingAdventures::TomlParser.
#
# # What is an Excel formula?
#
# Excel formulas are a mini-language for describing computations in cells.
# They optionally begin with "=" and can contain:
#
#   Literals:      numbers (42, 3.14, .5), strings ("hello"), booleans
#                  (TRUE/FALSE), error constants (#DIV/0!, #VALUE!, etc.)
#   References:    cell refs (A1, $B$2), ranges (A1:B10), cross-sheet
#                  refs (Sheet1!A1), named ranges (MyData)
#   Operators:     arithmetic (+, -, *, /, ^, &), comparison (=, <>, <,
#                  <=, >, >=), unary (-, +), postfix (%)
#   Functions:     NAME(arg, arg, ...) — e.g., SUM(A1:B10), IF(cond, t, f)
#   Arrays:        {1,2;3,4} — 2-D array constants
#   Grouping:      ( expression )
#
# # Grammar implemented
#
#   formula             = [ EQUALS ] expression ;
#   expression          = comparison_expr ;
#   comparison_expr     = concat_expr { comp_op concat_expr } ;
#   comp_op             = EQUALS | NOT_EQUALS | LESS_THAN | LESS_EQUALS
#                       | GREATER_THAN | GREATER_EQUALS ;
#   concat_expr         = additive_expr { AMP additive_expr } ;
#   additive_expr       = multiplicative_expr { (PLUS|MINUS) multiplicative_expr } ;
#   multiplicative_expr = power_expr { (STAR|SLASH) power_expr } ;
#   power_expr          = unary_expr { CARET unary_expr } ;
#   unary_expr          = { (PLUS|MINUS) } postfix_expr ;
#   postfix_expr        = primary { PERCENT } ;
#   primary             = LPAREN expression RPAREN
#                       | array_constant
#                       | function_call
#                       | ref_prefix_expr
#                       | cell_range | CELL
#                       | NAME | NUMBER | STRING | BOOL | ERROR_CONSTANT ;
#   cell_range          = CELL [ COLON CELL ] ;
#   ref_prefix_expr     = REF_PREFIX [ CELL | NAME ] [ COLON (REF_PREFIX CELL | CELL) ] ;
#   function_call       = NAME LPAREN [ arg_list ] RPAREN ;
#   arg_list            = arg { (COMMA|SEMICOLON) arg } ;
#   arg                 = [ expression ] ;
#   array_constant      = LBRACE array_row { SEMICOLON array_row } RBRACE ;
#   array_row           = array_item { COMMA array_item } ;
#   array_item          = [ (PLUS|MINUS) ] (NUMBER|STRING|BOOL|ERROR_CONSTANT) ;
#
# # Operator precedence (lowest to highest)
#
#   1. comparison  (=  <>  <  <=  >  >=)
#   2. concatenation (&)
#   3. additive (+ -)
#   4. multiplicative (* /)
#   5. power (^)
#   6. unary prefix (+ -)
#   7. postfix (%)
#   8. primary
#
# # Parse state
#
# Two package-level variables hold the current parse state:
#   $tokens_ref — arrayref from CodingAdventures::ExcelLexer->tokenize
#   $pos        — 0-based current index
#
# The parser is not re-entrant (package-level state), but formula parsing
# is always synchronous, so this is fine in practice.
#
# # Excel history note
#
# The Excel formula language traces back to VisiCalc (1979, Dan Bricklin)
# and Multiplan (1982, Microsoft).  The operator precedence is essentially
# the same as standard mathematics.  The "=" at the start of a formula
# is not part of the expression — it is a signal to the spreadsheet
# application that the cell contains a formula rather than plain text.
#
# The case-insensitivity is handled by the lexer (CodingAdventures::ExcelLexer),
# which lowercases the source before tokenizing.

use strict;
use warnings;

use CodingAdventures::ExcelLexer;
use CodingAdventures::ExcelParser::ASTNode;

our $VERSION = '0.01';

# Package-level parse state
my ($tokens_ref, $pos);

# ============================================================================
# Operator sets (for checking token types quickly)
# ============================================================================

my %COMP_OPS = map { $_ => 1 } qw(
    EQUALS NOT_EQUALS LESS_THAN LESS_EQUALS GREATER_THAN GREATER_EQUALS
);

# ============================================================================
# Public API
# ============================================================================

# --- parse($class, $source) ---------------------------------------------------
#
# Parse an Excel formula source string and return the root ASTNode.
#
# The leading "=" (if present) is consumed and stored in the root node's
# `eq` field.  The remaining formula is parsed as an expression.
#
# @param  $source  string  The Excel formula text.
# @return ASTNode          Root with rule_name "formula".
# @die                     On any lexer or parser error.

sub parse {
    my ($class, $source) = @_;

    my $toks    = CodingAdventures::ExcelLexer->tokenize($source);
    $tokens_ref = $toks;
    $pos        = 0;

    _skip_spaces();

    # Optionally consume the formula prefix "="
    my $eq_tok = undef;
    if ( _peek()->{type} eq 'EQUALS' ) {
        $eq_tok = _advance();
    }

    _skip_spaces();

    # Parse the expression body
    my $body = _parse_expression();

    _skip_spaces();

    # Verify all tokens consumed
    my $remaining = _peek();
    if ( $remaining->{type} ne 'EOF' ) {
        die sprintf(
            "CodingAdventures::ExcelParser: trailing content at line %d col %d: "
          . "unexpected %s ('%s')",
            $remaining->{line}, $remaining->{col},
            $remaining->{type}, $remaining->{value}
        );
    }

    return _node('formula', eq => $eq_tok, body => $body);
}

# ============================================================================
# Internal helpers
# ============================================================================

sub _peek    { $tokens_ref->[$pos] // $tokens_ref->[-1] }
sub _peek_at { $tokens_ref->[$pos + $_[0]] // $tokens_ref->[-1] }

sub _advance {
    my $t = _peek();
    $pos++;
    return $t;
}

sub _expect {
    my ($type) = @_;
    my $t = _peek();
    unless ( $t->{type} eq $type ) {
        die sprintf(
            "CodingAdventures::ExcelParser: Expected %s, got %s ('%s') "
          . "at line %d col %d",
            $type, $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _advance();
}

sub _check { _peek()->{type} eq $_[0] }

sub _skip_spaces {
    while ( _peek()->{type} eq 'SPACE' ) {
        _advance();
    }
}

# --- _node($rule_name, %fields) -----------------------------------------------
#
# Construct a new ASTNode with the given rule_name and extra fields.

sub _node {
    my ($rule, %fields) = @_;
    return CodingAdventures::ExcelParser::ASTNode->new(
        rule_name => $rule,
        %fields,
    );
}

# ============================================================================
# Expression parsing — recursive descent with explicit precedence levels
# ============================================================================

# --- _parse_expression() ------------------------------------------------------
#
# Top-level expression: dispatches to comparison level.

sub _parse_expression {
    return _parse_comparison();
}

# --- _parse_comparison() ------------------------------------------------------
#
# Grammar: comparison_expr = concat_expr { comp_op concat_expr }
#
# Comparison operators: = <> < <= > >=
# These are left-associative and have the lowest precedence.

sub _parse_comparison {
    _skip_spaces();
    my $left = _parse_concat();
    _skip_spaces();

    while ( $COMP_OPS{ _peek()->{type} } ) {
        my $op = _advance();
        _skip_spaces();
        my $right = _parse_concat();
        _skip_spaces();
        $left = _node('binop', op => $op, left => $left, right => $right);
    }

    return $left;
}

# --- _parse_concat() ----------------------------------------------------------
#
# Grammar: concat_expr = additive_expr { AMP additive_expr }
#
# The & operator concatenates strings: ="Hello "&"World"

sub _parse_concat {
    _skip_spaces();
    my $left = _parse_additive();
    _skip_spaces();

    while ( _check('AMP') ) {
        my $op = _advance();
        _skip_spaces();
        my $right = _parse_additive();
        _skip_spaces();
        $left = _node('binop', op => $op, left => $left, right => $right);
    }

    return $left;
}

# --- _parse_additive() --------------------------------------------------------
#
# Grammar: additive_expr = multiplicative_expr { (PLUS|MINUS) multiplicative_expr }

sub _parse_additive {
    _skip_spaces();
    my $left = _parse_multiplicative();
    _skip_spaces();

    while ( _check('PLUS') || _check('MINUS') ) {
        my $op = _advance();
        _skip_spaces();
        my $right = _parse_multiplicative();
        _skip_spaces();
        $left = _node('binop', op => $op, left => $left, right => $right);
    }

    return $left;
}

# --- _parse_multiplicative() --------------------------------------------------
#
# Grammar: multiplicative_expr = power_expr { (STAR|SLASH) power_expr }

sub _parse_multiplicative {
    _skip_spaces();
    my $left = _parse_power();
    _skip_spaces();

    while ( _check('STAR') || _check('SLASH') ) {
        my $op = _advance();
        _skip_spaces();
        my $right = _parse_power();
        _skip_spaces();
        $left = _node('binop', op => $op, left => $left, right => $right);
    }

    return $left;
}

# --- _parse_power() -----------------------------------------------------------
#
# Grammar: power_expr = unary_expr { CARET unary_expr }
#
# The ^ operator raises to a power: =A1^2.
# In Excel, exponentiation is left-associative (unlike many other languages
# where it is right-associative).  We implement left-associativity here.

sub _parse_power {
    _skip_spaces();
    my $left = _parse_unary();
    _skip_spaces();

    while ( _check('CARET') ) {
        my $op = _advance();
        _skip_spaces();
        my $right = _parse_unary();
        _skip_spaces();
        $left = _node('binop', op => $op, left => $left, right => $right);
    }

    return $left;
}

# --- _parse_unary() -----------------------------------------------------------
#
# Grammar: unary_expr = { (PLUS|MINUS) } postfix_expr
#
# Unary + and - can be stacked: =--A1 (double negation, common in VBA tricks)
# Each is right-associative (outermost unary wraps inner).

sub _parse_unary {
    _skip_spaces();

    if ( _check('PLUS') || _check('MINUS') ) {
        my $op      = _advance();
        _skip_spaces();
        my $operand = _parse_unary();    # recurse for right-assoc stacking
        return _node('unop', op => $op, operand => $operand);
    }

    return _parse_postfix();
}

# --- _parse_postfix() ---------------------------------------------------------
#
# Grammar: postfix_expr = primary { PERCENT }
#
# The % operator divides by 100: =50% = 0.5, =A1*100% scales A1 to percent.
# Multiple % can be stacked: =1%% = 0.0001 (unusual but syntactically valid).

sub _parse_postfix {
    _skip_spaces();
    my $primary = _parse_primary();
    _skip_spaces();

    while ( _check('PERCENT') ) {
        my $op = _advance();
        $primary = _node('postfix', op => $op, operand => $primary);
        _skip_spaces();
    }

    return $primary;
}

# --- _parse_primary() ---------------------------------------------------------
#
# Grammar: primary = LPAREN expr RPAREN | array | function_call | ref_prefix
#                  | cell_range | cell | name | number | string | bool | error
#
# Disambiguation at this level:
#
#   LPAREN          → parenthesized expression
#   LBRACE          → array constant
#   ERROR_CONSTANT  → error leaf
#   STRING          → string leaf
#   NUMBER          → number leaf
#   TRUE | FALSE    → bool leaf
#   REF_PREFIX      → cross-sheet reference
#   CELL            → cell ref or start of range (A1:B2)
#   NAME + LPAREN   → function call
#   NAME            → named range / bare identifier

sub _parse_primary {
    _skip_spaces();
    my $t = _peek();

    # ---- Parenthesized expression -------------------------------------------
    if ( $t->{type} eq 'LPAREN' ) {
        _advance();    # consume (
        _skip_spaces();
        my $inner = _parse_expression();
        _skip_spaces();
        _expect('RPAREN');
        return _node('group', expr => $inner);
    }

    # ---- Array constant: { row ; row } --------------------------------------
    if ( $t->{type} eq 'LBRACE' ) {
        return _parse_array_constant();
    }

    # ---- Error constant: #DIV/0! etc. ----------------------------------------
    if ( $t->{type} eq 'ERROR_CONSTANT' ) {
        return _node('error', token => _advance());
    }

    # ---- String literal -----------------------------------------------------
    if ( $t->{type} eq 'STRING' ) {
        return _node('string', token => _advance());
    }

    # ---- Number literal -----------------------------------------------------
    if ( $t->{type} eq 'NUMBER' ) {
        return _node('number', token => _advance());
    }

    # ---- Boolean keyword (TRUE / FALSE) ------------------------------------
    if ( $t->{type} eq 'TRUE' || $t->{type} eq 'FALSE' ) {
        return _node('bool', token => _advance());
    }

    # ---- REF_PREFIX: Sheet1! or 'My Sheet'! ---------------------------------
    #
    # A REF_PREFIX is followed by a CELL or NAME (or nothing for external refs).
    # It can also start a range: Sheet1!A1:B2

    if ( $t->{type} eq 'REF_PREFIX' ) {
        my $prefix = _advance();
        _skip_spaces();

        if ( _check('CELL') ) {
            my $cell = _advance();
            _skip_spaces();

            # Range with cross-sheet prefix: Sheet1!A1:B2
            if ( _check('COLON') ) {
                _advance();    # consume :
                _skip_spaces();
                my $end_ref;
                if ( _check('REF_PREFIX') ) {
                    my $pfx2  = _advance();
                    _skip_spaces();
                    my $cell2 = _expect('CELL');
                    $end_ref = _node('ref_prefix',
                        prefix => $pfx2,
                        ref    => _node('cell', token => $cell2),
                    );
                } else {
                    my $cell2 = _expect('CELL');
                    $end_ref = _node('cell', token => $cell2);
                }
                return _node('range',
                    start_ref => _node('ref_prefix',
                        prefix => $prefix,
                        ref    => _node('cell', token => $cell),
                    ),
                    end_ref => $end_ref,
                );
            }

            return _node('ref_prefix',
                prefix => $prefix,
                ref    => _node('cell', token => $cell),
            );
        }

        if ( _check('NAME') ) {
            my $nm = _advance();
            return _node('ref_prefix',
                prefix => $prefix,
                ref    => _node('name', token => $nm),
            );
        }

        # Bare prefix (external reference — no CELL or NAME follows)
        return _node('ref_prefix', prefix => $prefix, ref => undef);
    }

    # ---- CELL reference (possibly start of a range) -------------------------

    if ( $t->{type} eq 'CELL' ) {
        my $cell_tok = _advance();
        _skip_spaces();

        # Range: A1:B10
        if ( _check('COLON') ) {
            _advance();    # consume :
            _skip_spaces();

            my $end_ref;
            if ( _check('REF_PREFIX') ) {
                my $pfx2  = _advance();
                _skip_spaces();
                my $cell2 = _expect('CELL');
                $end_ref = _node('ref_prefix',
                    prefix => $pfx2,
                    ref    => _node('cell', token => $cell2),
                );
            } elsif ( _check('CELL') ) {
                $end_ref = _node('cell', token => _advance());
            } else {
                die sprintf(
                    "CodingAdventures::ExcelParser: Expected CELL after COLON, "
                  . "got %s at line %d col %d",
                    _peek()->{type}, _peek()->{line}, _peek()->{col}
                );
            }

            return _node('range',
                start_ref => _node('cell', token => $cell_tok),
                end_ref   => $end_ref,
            );
        }

        return _node('cell', token => $cell_tok);
    }

    # ---- NAME — function call, column range (B:C), or named range -----------

    if ( $t->{type} eq 'NAME' ) {
        my $name_tok = _advance();
        _skip_spaces();

        # Function call: NAME LPAREN args RPAREN
        if ( _check('LPAREN') ) {
            _advance();    # consume (
            _skip_spaces();
            my $args = _parse_arg_list();
            _skip_spaces();
            _expect('RPAREN');
            return _node('call', name => $name_tok, args => $args);
        }

        # Column range: B:C or B:$C (NAME COLON NAME/CELL)
        if ( _check('COLON') ) {
            _advance();    # consume :
            _skip_spaces();
            my $end_ref;
            if ( _check('NAME') ) {
                $end_ref = _node('name', token => _advance());
            } elsif ( _check('CELL') ) {
                $end_ref = _node('cell', token => _advance());
            } else {
                my $tok = _peek();
                die sprintf(
                    "CodingAdventures::ExcelParser: Expected NAME or CELL after COLON "
                  . "in range, got %s ('%s') at line %d col %d",
                    $tok->{type}, $tok->{value}, $tok->{line}, $tok->{col}
                );
            }
            return _node('range',
                start_ref => _node('name', token => $name_tok),
                end_ref   => $end_ref,
            );
        }

        return _node('name', token => $name_tok);
    }

    # ---- Fallthrough: unexpected token --------------------------------------
    die sprintf(
        "CodingAdventures::ExcelParser: unexpected token %s ('%s') "
      . "at line %d col %d",
        $t->{type}, $t->{value}, $t->{line}, $t->{col}
    );
}

# ============================================================================
# Argument list parsing
# ============================================================================
#
# Grammar: arg_list = arg { (COMMA|SEMICOLON) arg }
#          arg      = [ expression ]
#
# Excel allows empty arguments in function calls:
#   =IF(,TRUE,FALSE)   — first argument is empty (treated as 0/FALSE/"")
#   =INDEX(A:A,,1)     — second argument empty (uses default)
#
# Empty arguments are represented as undef entries in the returned arrayref.

sub _parse_arg_list {
    my @args;

    # Immediately closing ) → empty arg list
    return \@args if _check('RPAREN');

    # First argument
    _skip_spaces();
    if ( _check('COMMA') || _check('SEMICOLON') ) {
        push @args, undef;    # empty first argument
    } else {
        push @args, _parse_expression();
    }
    _skip_spaces();

    while ( _check('COMMA') || _check('SEMICOLON') ) {
        _advance();    # consume separator
        _skip_spaces();

        # Trailing comma before ) → stop
        last if _check('RPAREN');

        if ( _check('COMMA') || _check('SEMICOLON') ) {
            push @args, undef;    # empty middle argument
        } else {
            push @args, _parse_expression();
        }
        _skip_spaces();
    }

    return \@args;
}

# ============================================================================
# Array constant parsing
# ============================================================================
#
# Grammar: array_constant = LBRACE array_row { SEMICOLON array_row } RBRACE
#          array_row      = array_item { COMMA array_item }
#          array_item     = [ (PLUS|MINUS) ] (NUMBER|STRING|BOOL|ERROR_CONSTANT)
#
# Array constants can only contain scalars, not expressions or references:
#   {1, 2, 3}            — 1-D row array
#   {1, 2; 3, 4}         — 2-D 2×2 array
#   {"a", "b"; "c", "d"} — 2-D string array
#   {-1, 2, -3}          — signed numbers
#
# Excel uses semicolons to separate rows (not newlines as in some notations).

sub _parse_array_constant {
    _expect('LBRACE');
    _skip_spaces();

    my @rows;

    # Parse a single array row
    my $parse_item = sub {
        _skip_spaces();
        if ( _check('MINUS') || _check('PLUS') ) {
            my $sign = _advance();
            _skip_spaces();
            my $num = _expect('NUMBER');
            return _node('unop',
                op      => $sign,
                operand => _node('number', token => $num),
            );
        }
        if ( _check('NUMBER') ) {
            return _node('number', token => _advance());
        }
        if ( _check('STRING') ) {
            return _node('string', token => _advance());
        }
        if ( _check('TRUE') || _check('FALSE') ) {
            return _node('bool', token => _advance());
        }
        if ( _check('ERROR_CONSTANT') ) {
            return _node('error', token => _advance());
        }
        die sprintf(
            "CodingAdventures::ExcelParser: expected array item, got %s "
          . "at line %d col %d",
            _peek()->{type}, _peek()->{line}, _peek()->{col}
        );
    };

    my $parse_row = sub {
        my @items;
        push @items, $parse_item->();
        _skip_spaces();
        while ( _check('COMMA') ) {
            _advance();
            push @items, $parse_item->();
            _skip_spaces();
        }
        return \@items;
    };

    push @rows, $parse_row->();
    _skip_spaces();

    while ( _check('SEMICOLON') ) {
        _advance();
        _skip_spaces();
        last if _check('RBRACE');    # trailing semicolon
        push @rows, $parse_row->();
        _skip_spaces();
    }

    _expect('RBRACE');
    return _node('array', rows => \@rows);
}

1;

__END__

=head1 NAME

CodingAdventures::ExcelParser - Hand-written recursive-descent Excel formula parser

=head1 SYNOPSIS

    use CodingAdventures::ExcelParser;

    my $ast = CodingAdventures::ExcelParser->parse('=SUM(A1:B10)');
    print $ast->rule_name;        # formula
    print $ast->{body}->rule_name;  # call

    my $ast2 = CodingAdventures::ExcelParser->parse('=IF(A1>0,"pos","neg")');
    my $args  = $ast2->{body}{args};  # arrayref of 3 ASTNodes

=head1 DESCRIPTION

A hand-written recursive-descent parser for Excel formula strings.
Tokenizes source text using C<CodingAdventures::ExcelLexer> and constructs
an AST using C<CodingAdventures::ExcelParser::ASTNode>.

Implements the Excel formula grammar with full operator precedence:
comparison < concatenation(&) < additive < multiplicative < power < unary < postfix(%).

Returns a root node with C<rule_name == "formula">.  The C<eq> field holds
the EQUALS token (or undef if the formula had no leading =).  The C<body>
field holds the expression root.

=head1 METHODS

=head2 parse($source)

Parse an Excel formula string.  Returns the root C<ASTNode>.
Dies on lexer or parser errors with a descriptive message.

=head1 AST NODE KINDS

    formula     eq (token|undef), body (ASTNode)
    binop       op (token), left (ASTNode), right (ASTNode)
    unop        op (token), operand (ASTNode)
    postfix     op (token), operand (ASTNode)
    call        name (token), args (arrayref of ASTNode|undef)
    range       start_ref (ASTNode), end_ref (ASTNode)
    ref_prefix  prefix (token), ref (ASTNode|undef)
    cell        token
    number      token
    string      token
    bool        token
    error       token
    name        token
    array       rows (arrayref of arrayref of ASTNode)
    group       expr (ASTNode)

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
