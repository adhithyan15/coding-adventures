package CodingAdventures::Ls00;

# ============================================================================
# CodingAdventures::Ls00 -- Generic LSP server framework (umbrella module)
# ============================================================================
#
# # What is the Language Server Protocol?
#
# When you open a source file in VS Code and see red squiggles under syntax
# errors, autocomplete suggestions, or "Go to Definition" -- none of that is
# built into the editor.  It comes from a *language server*: a separate
# process that communicates with the editor over the Language Server Protocol.
#
# LSP was invented by Microsoft to solve the M x N problem:
#
#     M editors x N languages = M x N integrations to write
#
# With LSP, each language writes one server, and every LSP-aware editor gets
# all features automatically.  This package is the *generic* half -- it
# handles all the protocol boilerplate.  A language author only writes a
# "bridge" object that connects their lexer/parser to this framework.
#
# # Architecture
#
#     Lexer -> Parser -> [Bridge] -> [LspServer] -> VS Code / Neovim / Emacs
#
# # JSON-RPC over stdio
#
# Like the Debug Adapter Protocol (DAP), LSP speaks JSON-RPC over stdio.
# Each message is Content-Length-framed (same format as HTTP headers).  The
# underlying transport is handled by CodingAdventures::JsonRpc.
#
# # How to use this package
#
#   1. Create a bridge object that implements `tokenize` and `parse` methods
#      (and optionally `hover`, `definition`, etc.)
#   2. Call CodingAdventures::Ls00::Server->new($bridge, \*STDIN, \*STDOUT)
#   3. Call $server->serve() -- it blocks until the editor closes the connection.
#
# # Capability detection
#
# Perl's built-in `can()` method performs the same role as Go's type
# assertions.  If `$bridge->can('hover')` returns true, the server
# advertises hoverProvider and routes hover requests to `$bridge->hover()`.
# No stubs required for unsupported features.

use strict;
use warnings;

our $VERSION = '0.01';

# Load all sub-modules eagerly so callers get everything with one `use`.
use CodingAdventures::Ls00::Types          ();
use CodingAdventures::Ls00::LanguageBridge ();
use CodingAdventures::Ls00::DocumentManager();
use CodingAdventures::Ls00::ParseCache     ();
use CodingAdventures::Ls00::Capabilities   ();
use CodingAdventures::Ls00::LspErrors      ();
use CodingAdventures::Ls00::Handlers       ();
use CodingAdventures::Ls00::Server         ();

1;

__END__

=head1 NAME

CodingAdventures::Ls00 -- Generic Language Server Protocol framework

=head1 SYNOPSIS

  use CodingAdventures::Ls00;

  # 1. Create a bridge object (see CodingAdventures::Ls00::LanguageBridge)
  my $bridge = MyLanguage::Bridge->new();

  # 2. Create an LSP server
  my $server = CodingAdventures::Ls00::Server->new($bridge, \*STDIN, \*STDOUT);

  # 3. Serve (blocks until stdin closes)
  $server->serve();

=head1 DESCRIPTION

A generic LSP server framework that language-specific "bridges" plug into.
The bridge is any Perl object that implements at minimum C<tokenize()> and
C<parse()>.  Optional methods (C<hover>, C<definition>, C<completion>, etc.)
are detected at runtime via Perl's C<can()> method introspection.

=head1 MODULES

=over 4

=item L<CodingAdventures::Ls00::Types> -- LSP data types as constructor functions

=item L<CodingAdventures::Ls00::LanguageBridge> -- Bridge interface documentation

=item L<CodingAdventures::Ls00::DocumentManager> -- Tracks open documents

=item L<CodingAdventures::Ls00::ParseCache> -- Caches parse results by version

=item L<CodingAdventures::Ls00::Capabilities> -- Builds capability response

=item L<CodingAdventures::Ls00::LspErrors> -- LSP error code constants

=item L<CodingAdventures::Ls00::Handlers> -- LSP request/notification handlers

=item L<CodingAdventures::Ls00::Server> -- Main server coordinator

=back

=cut
