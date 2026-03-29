package CodingAdventures::LatticeTranspiler;

# ============================================================================
# CodingAdventures::LatticeTranspiler — End-to-end Lattice → CSS pipeline
# ============================================================================
#
# This module wires together two packages into a single `transpile` method:
#
#   1. CodingAdventures::LatticeParser    — Lattice source text → AST
#   2. CodingAdventures::LatticeAstToCss  — Lattice AST → CSS text
#
# ## Pipeline
#
#   Lattice Source
#        │
#        ▼
#   LatticeParser->parse($source)   ← tokenize + recursive-descent parse
#        │ AST (ASTNode tree)
#        ▼
#   LatticeAstToCss->compile($ast)  ← variable expansion, mixin expansion,
#        │                             control flow, nesting flattening
#        ▼
#       CSS text
#
# ## Error Handling
#
# Both transpile() and transpile_file() use eval{} to catch exceptions.
# On success they return ($css, undef).
# On failure they return (undef, $error_message).
#
# Errors can arise from:
#   - Lexer errors (unknown characters in the source)
#   - Parser errors (syntax errors)
#   - Compiler errors (e.g., compile method receiving invalid input)

use strict;
use warnings;

use CodingAdventures::LatticeParser;
use CodingAdventures::LatticeAstToCss;

our $VERSION = '0.1.0';

# ============================================================================
# Public API
# ============================================================================

=head1 NAME

CodingAdventures::LatticeTranspiler - End-to-end Lattice CSS superset transpiler

=head1 SYNOPSIS

    use CodingAdventures::LatticeTranspiler;

    my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile(<<'END');
        $primary: #4a90d9;

        @mixin button($bg, $fg: white) {
            background: $bg;
            color: $fg;
            padding: 8px 16px;
        }

        .btn {
            @include button($primary);
        }
    END

    if ($err) {
        die "Transpile error: $err\n";
    }
    print $css;

=head1 DESCRIPTION

Wires together L<CodingAdventures::LatticeParser> and
L<CodingAdventures::LatticeAstToCss> into a single convenience API.

=head1 METHODS

=head2 transpile($source) -> ($css, $error)

Transpile a Lattice source string to CSS text.

On success: returns C<($css_string, undef)>.
On failure: returns C<(undef, $error_message)>.

=cut

sub transpile {
    my ($class, $source) = @_;

    # Step 1: Parse
    # LatticeParser->parse() dies on lexer/parser failure.
    my $ast;
    eval { $ast = CodingAdventures::LatticeParser->parse($source) };
    if ($@) {
        my $err = $@;
        chomp $err;
        return (undef, $err);
    }

    # Step 2: Compile AST → CSS
    my $css;
    eval { $css = CodingAdventures::LatticeAstToCss->compile($ast) };
    if ($@) {
        my $err = $@;
        chomp $err;
        return (undef, $err);
    }

    return ($css, undef);
}

=head2 transpile_file($path) -> ($css, $error)

Read a file and transpile it.

On success: returns C<($css_string, undef)>.
On failure: returns C<(undef, $error_message)>.

=cut

sub transpile_file {
    my ($class, $path) = @_;

    # Reject path traversal attempts. A path containing ".." could escape
    # the intended directory and read arbitrary files.
    if ($path =~ m{\.\.}) {
        return (undef, "LatticeTranspiler: path traversal not allowed: $path");
    }

    # Open and read the file
    open(my $fh, '<', $path)
        or return (undef, "LatticeTranspiler: cannot open '$path': $!");
    my $source = do { local $/; <$fh> };
    close $fh;

    return $class->transpile($source);
}

1;
