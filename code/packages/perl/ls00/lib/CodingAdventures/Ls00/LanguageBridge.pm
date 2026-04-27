package CodingAdventures::Ls00::LanguageBridge;

# ============================================================================
# CodingAdventures::Ls00::LanguageBridge -- Bridge interface documentation
# ============================================================================
#
# # Design Philosophy: Duck Typing with can()
#
# In Go, capability detection uses type assertions:
#
#     if hp, ok := bridge.(HoverProvider); ok { ... }
#
# In Perl, we use the built-in `can()` method:
#
#     if ($bridge->can('hover')) { ... }
#
# This is Perl's equivalent of interface checking.  Any blessed object that
# responds to `tokenize()` and `parse()` is a valid bridge.  No base class
# or interface declaration is required -- just implement the methods you want.
#
# # Required Methods
#
# Every bridge MUST implement these two methods:
#
#   $bridge->tokenize($source) -> (\@tokens, $err)
#
#     Lex the source string and return an arrayref of token hashrefs.
#     Each token has: { type => 'KEYWORD', value => 'let', line => 1, column => 1 }
#     Returns (undef, $error_string) on fatal error.
#
#   $bridge->parse($source) -> ($ast, \@diagnostics, $err)
#
#     Parse the source string and return:
#       - $ast:          any Perl value representing the parsed tree
#       - \@diagnostics: arrayref of diagnostic hashrefs (may be empty)
#       - $err:          error string on fatal failure, undef otherwise
#     Even with syntax errors, parse() should return a partial AST.
#
# # Optional Methods
#
# Implement any of these to enable the corresponding LSP feature.  The
# server checks `$bridge->can('method_name')` at startup and only
# advertises capabilities for methods the bridge actually has.
#
#   $bridge->hover($ast, $pos) -> ($hover_result, $err)
#     Returns hover content (markdown) for the symbol at $pos.
#     Return (undef, undef) if no hover info at this position.
#
#   $bridge->definition($ast, $pos, $uri) -> ($location, $err)
#     Returns the location where the symbol at $pos was declared.
#
#   $bridge->references($ast, $pos, $uri, $include_decl) -> (\@locations, $err)
#     Returns all uses of the symbol at $pos.
#
#   $bridge->completion($ast, $pos) -> (\@items, $err)
#     Returns autocomplete suggestions valid at $pos.
#
#   $bridge->rename($ast, $pos, $new_name) -> ($workspace_edit, $err)
#     Returns text edits needed to rename the symbol at $pos.
#
#   $bridge->semantic_tokens($source, \@tokens) -> (\@semantic_tokens, $err)
#     Maps raw tokens to semantic token data for highlighting.
#
#   $bridge->document_symbols($ast) -> (\@symbols, $err)
#     Returns the outline tree (document symbols) for the AST.
#
#   $bridge->folding_ranges($ast) -> (\@ranges, $err)
#     Returns collapsible regions derived from the AST.
#
#   $bridge->signature_help($ast, $pos) -> ($result, $err)
#     Returns signature hint information for the call at $pos.
#
#   $bridge->format($source) -> (\@text_edits, $err)
#     Returns text edits that format the document.
#
# # Example Bridge (Minimal)
#
#   package MyLang::Bridge;
#   sub new { bless {}, shift }
#   sub tokenize {
#       my ($self, $source) = @_;
#       # ... lex source ...
#       return (\@tokens, undef);
#   }
#   sub parse {
#       my ($self, $source) = @_;
#       # ... parse source ...
#       return ($ast, \@diagnostics, undef);
#   }
#   # That's it!  No stubs needed for hover, definition, etc.
#   # The server will not advertise those capabilities.

use strict;
use warnings;

our $VERSION = '0.01';

# This module is purely documentation -- no code to execute.
# Bridges do NOT need to inherit from this class.  Just implement the
# methods described above.

1;

__END__

=head1 NAME

CodingAdventures::Ls00::LanguageBridge -- Bridge interface documentation

=head1 DESCRIPTION

Documents the interface that bridge objects must implement to work with
the ls00 LSP server framework.

A bridge is any blessed Perl object.  It must implement C<tokenize()> and
C<parse()>.  Optional methods like C<hover()>, C<definition()>, etc. are
detected at runtime via C<< $bridge->can('method_name') >>.

See the source of this module for complete method signatures and examples.

=cut
