package CodingAdventures::Ls00::Handlers;

# ============================================================================
# CodingAdventures::Ls00::Handlers -- LSP request and notification handlers
# ============================================================================
#
# This module contains all the handler functions that the Server registers
# with the JSON-RPC dispatch loop.  Each handler corresponds to one LSP
# method (e.g., "textDocument/hover", "textDocument/didOpen").
#
# # Handler Contract
#
# Request handlers receive ($server, $id, $params) and return:
#   - A result value (hashref, arrayref, undef) for success
#   - A hashref { code => N, message => "..." } for an error response
#
# Notification handlers receive ($server, $params) and return nothing.
#
# # Server Lifecycle
#
#   Client (editor)              Server (us)
#     |                               |
#     |--initialize------------->     |  return capabilities
#     |<-----------result--------     |
#     |--initialized (notif)----->    |  no-op
#     |--textDocument/didOpen---->    |  open doc, parse, push diagnostics
#     |--textDocument/hover------>    |  call bridge->hover, return result
#     |<-----------result--------     |
#     |--shutdown---------------->    |  set shutdown flag, return undef
#     |--exit (notif)------------>    |  exit process

use strict;
use warnings;

use CodingAdventures::Ls00::Capabilities qw(build_capabilities encode_semantic_tokens);
use CodingAdventures::Ls00::LspErrors qw(:all);
use CodingAdventures::JsonRpc::Errors qw(:all);

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# register_handlers($server)
#
# Wire all LSP method names to their handler functions on the server's
# JSON-RPC server.
# ---------------------------------------------------------------------------

sub register_handlers {
    my ($server) = @_;

    my $rpc = $server->{rpc_server};

    # -- Lifecycle --
    $rpc->on_request('initialize', sub { handle_initialize($server, @_) });
    $rpc->on_notification('initialized', sub { handle_initialized($server, @_) });
    $rpc->on_request('shutdown', sub { handle_shutdown($server, @_) });
    $rpc->on_notification('exit', sub { handle_exit($server, @_) });

    # -- Text document synchronization --
    $rpc->on_notification('textDocument/didOpen', sub { handle_did_open($server, @_) });
    $rpc->on_notification('textDocument/didChange', sub { handle_did_change($server, @_) });
    $rpc->on_notification('textDocument/didClose', sub { handle_did_close($server, @_) });
    $rpc->on_notification('textDocument/didSave', sub { handle_did_save($server, @_) });

    # -- Feature requests --
    $rpc->on_request('textDocument/hover', sub { handle_hover($server, @_) });
    $rpc->on_request('textDocument/definition', sub { handle_definition($server, @_) });
    $rpc->on_request('textDocument/references', sub { handle_references($server, @_) });
    $rpc->on_request('textDocument/completion', sub { handle_completion($server, @_) });
    $rpc->on_request('textDocument/rename', sub { handle_rename($server, @_) });
    $rpc->on_request('textDocument/documentSymbol', sub { handle_document_symbol($server, @_) });
    $rpc->on_request('textDocument/semanticTokens/full', sub { handle_semantic_tokens_full($server, @_) });
    $rpc->on_request('textDocument/foldingRange', sub { handle_folding_range($server, @_) });
    $rpc->on_request('textDocument/signatureHelp', sub { handle_signature_help($server, @_) });
    $rpc->on_request('textDocument/formatting', sub { handle_formatting($server, @_) });
}

# =========================================================================
# Lifecycle Handlers
# =========================================================================

# handle_initialize -- LSP initialize request
#
# The first message from the editor.  We return our capabilities.

sub handle_initialize {
    my ($server, $id, $params) = @_;

    $server->{initialized} = 1;

    my $caps = build_capabilities($server->{bridge});

    return {
        capabilities => $caps,
        serverInfo   => {
            name    => 'ls00-generic-lsp-server',
            version => '0.1.0',
        },
    };
}

# handle_initialized -- "initialized" notification
#
# Handshake complete.  No action needed.

sub handle_initialized {
    my ($server, $params) = @_;
    # No-op.
}

# handle_shutdown -- LSP shutdown request
#
# Set the shutdown flag and return undef (null).

sub handle_shutdown {
    my ($server, $id, $params) = @_;
    $server->{shutdown} = 1;
    return undef;
}

# handle_exit -- "exit" notification
#
# Exit with code 0 if shutdown was received, 1 otherwise.

sub handle_exit {
    my ($server, $params) = @_;
    if ($server->{shutdown}) {
        exit(0);
    } else {
        exit(1);
    }
}

# =========================================================================
# Text Document Synchronization Handlers
# =========================================================================

# handle_did_open -- textDocument/didOpen notification
#
# Register the document with the manager, parse, and push diagnostics.

sub handle_did_open {
    my ($server, $params) = @_;
    return unless ref $params eq 'HASH';

    my $td = $params->{textDocument};
    return unless ref $td eq 'HASH';

    my $uri     = $td->{uri} // '';
    my $text    = $td->{text} // '';
    my $version = $td->{version} // 1;
    return unless $uri;

    $server->{doc_manager}->open($uri, $text, $version);

    my $result = $server->{parse_cache}->get_or_parse(
        $uri, $version, $text, $server->{bridge}
    );
    _publish_diagnostics($server, $uri, $version, $result->{diagnostics});
}

# handle_did_change -- textDocument/didChange notification
#
# Apply incremental changes, re-parse, and push diagnostics.

sub handle_did_change {
    my ($server, $params) = @_;
    return unless ref $params eq 'HASH';

    my $uri = _parse_uri($params);
    return unless $uri;

    my $version = 0;
    if (ref $params->{textDocument} eq 'HASH') {
        $version = $params->{textDocument}{version} // 0;
    }

    my $changes_raw = $params->{contentChanges} // [];
    my @changes;

    for my $change_raw (@$changes_raw) {
        next unless ref $change_raw eq 'HASH';
        my %change = ( new_text => $change_raw->{text} // '' );

        if (defined $change_raw->{range} && ref $change_raw->{range} eq 'HASH') {
            $change{range} = _parse_lsp_range($change_raw->{range});
        }
        # If range is absent, it stays undef -> full replacement.

        push @changes, \%change;
    }

    my $err = $server->{doc_manager}->apply_changes($uri, \@changes, $version);
    return if $err;  # Document wasn't open.

    my ($doc, $found) = $server->{doc_manager}->get($uri);
    return unless $found;

    my $result = $server->{parse_cache}->get_or_parse(
        $uri, $doc->{version}, $doc->{text}, $server->{bridge}
    );
    _publish_diagnostics($server, $uri, $version, $result->{diagnostics});
}

# handle_did_close -- textDocument/didClose notification
#
# Remove the document and clear diagnostics.

sub handle_did_close {
    my ($server, $params) = @_;
    return unless ref $params eq 'HASH';

    my $uri = _parse_uri($params);
    return unless $uri;

    $server->{doc_manager}->close($uri);
    $server->{parse_cache}->evict($uri);

    # Clear diagnostics by publishing an empty list.
    _publish_diagnostics($server, $uri, 0, []);
}

# handle_did_save -- textDocument/didSave notification

sub handle_did_save {
    my ($server, $params) = @_;
    return unless ref $params eq 'HASH';

    my $uri = _parse_uri($params);
    return unless $uri;

    # If the client sends full text in didSave, apply it.
    if (defined $params->{text} && $params->{text} ne '') {
        my ($doc, $found) = $server->{doc_manager}->get($uri);
        if ($found) {
            $server->{doc_manager}->close($uri);
            $server->{doc_manager}->open($uri, $params->{text}, $doc->{version});
            my $result = $server->{parse_cache}->get_or_parse(
                $uri, $doc->{version}, $params->{text}, $server->{bridge}
            );
            _publish_diagnostics($server, $uri, $doc->{version}, $result->{diagnostics});
        }
    }
}

# =========================================================================
# Feature Request Handlers
# =========================================================================

# handle_hover -- textDocument/hover

sub handle_hover {
    my ($server, $id, $params) = @_;
    return undef unless ref $params eq 'HASH';

    my $uri = _parse_uri($params);
    my $pos = _parse_position($params);

    unless ($server->{bridge}->can('hover')) {
        return undef;
    }

    my ($doc, $parse_result, $err) = _get_parse_result($server, $uri);
    return $err if $err;

    return undef unless $parse_result->{ast};

    my ($hover, $bridge_err) = $server->{bridge}->hover($parse_result->{ast}, $pos);
    return undef if $bridge_err;
    return undef unless $hover;

    my $result = {
        contents => {
            kind  => 'markdown',
            value => $hover->{contents},
        },
    };

    if (defined $hover->{range}) {
        $result->{range} = _range_to_lsp($hover->{range});
    }

    return $result;
}

# handle_definition -- textDocument/definition

sub handle_definition {
    my ($server, $id, $params) = @_;
    return undef unless ref $params eq 'HASH';

    my $uri = _parse_uri($params);
    my $pos = _parse_position($params);

    unless ($server->{bridge}->can('definition')) {
        return undef;
    }

    my ($doc, $parse_result, $err) = _get_parse_result($server, $uri);
    return $err if $err;
    return undef unless $parse_result->{ast};

    my ($location, $bridge_err) = $server->{bridge}->definition(
        $parse_result->{ast}, $pos, $uri
    );
    return undef if $bridge_err || !$location;

    return _location_to_lsp($location);
}

# handle_references -- textDocument/references

sub handle_references {
    my ($server, $id, $params) = @_;
    return [] unless ref $params eq 'HASH';

    my $uri = _parse_uri($params);
    my $pos = _parse_position($params);

    # Extract includeDeclaration from context.
    my $include_decl = 0;
    if (ref $params->{context} eq 'HASH') {
        $include_decl = $params->{context}{includeDeclaration} ? 1 : 0;
    }

    unless ($server->{bridge}->can('references')) {
        return [];
    }

    my ($doc, $parse_result, $err) = _get_parse_result($server, $uri);
    return $err if $err;
    return [] unless $parse_result->{ast};

    my ($locations, $bridge_err) = $server->{bridge}->references(
        $parse_result->{ast}, $pos, $uri, $include_decl
    );
    return [] if $bridge_err;

    return [ map { _location_to_lsp($_) } @$locations ];
}

# handle_completion -- textDocument/completion

sub handle_completion {
    my ($server, $id, $params) = @_;
    my $empty = { isIncomplete => JSON::PP::false, items => [] };
    return $empty unless ref $params eq 'HASH';

    my $uri = _parse_uri($params);
    my $pos = _parse_position($params);

    unless ($server->{bridge}->can('completion')) {
        return $empty;
    }

    my ($doc, $parse_result, $err) = _get_parse_result($server, $uri);
    return $err if $err;
    return $empty unless $parse_result->{ast};

    my ($items, $bridge_err) = $server->{bridge}->completion($parse_result->{ast}, $pos);
    return $empty if $bridge_err;

    my @lsp_items;
    for my $item (@$items) {
        my %ci = ( label => $item->{label} );
        $ci{kind}             = $item->{kind}              if defined $item->{kind};
        $ci{detail}           = $item->{detail}            if defined $item->{detail};
        $ci{documentation}    = $item->{documentation}     if defined $item->{documentation};
        $ci{insertText}       = $item->{insert_text}       if defined $item->{insert_text};
        $ci{insertTextFormat} = $item->{insert_text_format} if defined $item->{insert_text_format};
        push @lsp_items, \%ci;
    }

    return { isIncomplete => JSON::PP::false, items => \@lsp_items };
}

# handle_rename -- textDocument/rename

sub handle_rename {
    my ($server, $id, $params) = @_;
    return { code => INVALID_PARAMS, message => 'invalid params' }
        unless ref $params eq 'HASH';

    my $uri      = _parse_uri($params);
    my $pos      = _parse_position($params);
    my $new_name = $params->{newName} // '';

    unless ($new_name) {
        return { code => INVALID_PARAMS, message => 'newName is required' };
    }

    unless ($server->{bridge}->can('rename')) {
        return { code => REQUEST_FAILED, message => 'rename not supported' };
    }

    my ($doc, $parse_result, $err) = _get_parse_result($server, $uri);
    return $err if $err;

    unless ($parse_result->{ast}) {
        return { code => REQUEST_FAILED, message => 'no AST available' };
    }

    my ($edit, $bridge_err) = $server->{bridge}->rename(
        $parse_result->{ast}, $pos, $new_name
    );
    if ($bridge_err) {
        return { code => REQUEST_FAILED, message => "$bridge_err" };
    }
    unless ($edit) {
        return { code => REQUEST_FAILED, message => 'symbol not found at position' };
    }

    # Convert WorkspaceEdit to LSP format.
    my %lsp_changes;
    for my $edit_uri (keys %{$edit->{changes}}) {
        my @lsp_edits;
        for my $te (@{$edit->{changes}{$edit_uri}}) {
            push @lsp_edits, {
                range   => _range_to_lsp($te->{range}),
                newText => $te->{new_text},
            };
        }
        $lsp_changes{$edit_uri} = \@lsp_edits;
    }

    return { changes => \%lsp_changes };
}

# handle_document_symbol -- textDocument/documentSymbol

sub handle_document_symbol {
    my ($server, $id, $params) = @_;
    return [] unless ref $params eq 'HASH';

    my $uri = _parse_uri($params);

    unless ($server->{bridge}->can('document_symbols')) {
        return [];
    }

    my ($doc, $parse_result, $err) = _get_parse_result($server, $uri);
    return $err if $err;
    return [] unless $parse_result->{ast};

    my ($symbols, $bridge_err) = $server->{bridge}->document_symbols($parse_result->{ast});
    return [] if $bridge_err;

    return _convert_document_symbols($symbols);
}

# handle_semantic_tokens_full -- textDocument/semanticTokens/full

sub handle_semantic_tokens_full {
    my ($server, $id, $params) = @_;
    my $empty = { data => [] };
    return $empty unless ref $params eq 'HASH';

    my $uri = _parse_uri($params);

    unless ($server->{bridge}->can('semantic_tokens')) {
        return $empty;
    }

    my ($doc, $found) = $server->{doc_manager}->get($uri);
    return $empty unless $found;

    my ($tokens, $tok_err) = $server->{bridge}->tokenize($doc->{text});
    return $empty if $tok_err;

    my ($sem_tokens, $bridge_err) = $server->{bridge}->semantic_tokens(
        $doc->{text}, $tokens
    );
    return $empty if $bridge_err;

    my $data = encode_semantic_tokens($sem_tokens);
    return { data => $data };
}

# handle_folding_range -- textDocument/foldingRange

sub handle_folding_range {
    my ($server, $id, $params) = @_;
    return [] unless ref $params eq 'HASH';

    my $uri = _parse_uri($params);

    unless ($server->{bridge}->can('folding_ranges')) {
        return [];
    }

    my ($doc, $parse_result, $err) = _get_parse_result($server, $uri);
    return $err if $err;
    return [] unless $parse_result->{ast};

    my ($ranges, $bridge_err) = $server->{bridge}->folding_ranges($parse_result->{ast});
    return [] if $bridge_err;

    my @result;
    for my $fr (@$ranges) {
        my %m = (
            startLine => $fr->{start_line},
            endLine   => $fr->{end_line},
        );
        $m{kind} = $fr->{kind} if defined $fr->{kind};
        push @result, \%m;
    }
    return \@result;
}

# handle_signature_help -- textDocument/signatureHelp

sub handle_signature_help {
    my ($server, $id, $params) = @_;
    return undef unless ref $params eq 'HASH';

    my $uri = _parse_uri($params);
    my $pos = _parse_position($params);

    unless ($server->{bridge}->can('signature_help')) {
        return undef;
    }

    my ($doc, $parse_result, $err) = _get_parse_result($server, $uri);
    return $err if $err;
    return undef unless $parse_result->{ast};

    my ($sig_help, $bridge_err) = $server->{bridge}->signature_help(
        $parse_result->{ast}, $pos
    );
    return undef if $bridge_err || !$sig_help;

    # Convert to LSP format.
    my @lsp_sigs;
    for my $sig (@{$sig_help->{signatures}}) {
        my @lsp_params;
        for my $p (@{$sig->{parameters}}) {
            my %pp = ( label => $p->{label} );
            $pp{documentation} = $p->{documentation} if defined $p->{documentation};
            push @lsp_params, \%pp;
        }
        my %s = (
            label      => $sig->{label},
            parameters => \@lsp_params,
        );
        $s{documentation} = $sig->{documentation} if defined $sig->{documentation};
        push @lsp_sigs, \%s;
    }

    return {
        signatures      => \@lsp_sigs,
        activeSignature => $sig_help->{active_signature},
        activeParameter => $sig_help->{active_parameter},
    };
}

# handle_formatting -- textDocument/formatting

sub handle_formatting {
    my ($server, $id, $params) = @_;
    return [] unless ref $params eq 'HASH';

    my $uri = _parse_uri($params);

    unless ($server->{bridge}->can('format')) {
        return [];
    }

    my ($doc, $found) = $server->{doc_manager}->get($uri);
    return [] unless $found;

    my ($edits, $bridge_err) = $server->{bridge}->format($doc->{text});
    if ($bridge_err) {
        return { code => REQUEST_FAILED, message => "formatting failed: $bridge_err" };
    }

    my @lsp_edits;
    for my $edit (@$edits) {
        push @lsp_edits, {
            range   => _range_to_lsp($edit->{range}),
            newText => $edit->{new_text},
        };
    }
    return \@lsp_edits;
}

# =========================================================================
# Internal Helpers
# =========================================================================

# _get_parse_result($server, $uri) -> ($doc, $parse_result, $err)
#
# Get the current parse result for a document.  Returns an error hashref
# (suitable for returning from a request handler) if the document is not open.

sub _get_parse_result {
    my ($server, $uri) = @_;

    my ($doc, $found) = $server->{doc_manager}->get($uri);
    unless ($found) {
        return (undef, undef, {
            code    => REQUEST_FAILED,
            message => "document not open: $uri",
        });
    }

    my $result = $server->{parse_cache}->get_or_parse(
        $uri, $doc->{version}, $doc->{text}, $server->{bridge}
    );
    return ($doc, $result, undef);
}

# _publish_diagnostics($server, $uri, $version, \@diagnostics)
#
# Send the textDocument/publishDiagnostics notification to the editor.

sub _publish_diagnostics {
    my ($server, $uri, $version, $diagnostics) = @_;

    my @lsp_diags;
    for my $d (@$diagnostics) {
        my %diag = (
            range    => _range_to_lsp($d->{range}),
            severity => $d->{severity},
            message  => $d->{message},
        );
        $diag{code} = $d->{code} if defined $d->{code} && $d->{code} ne '';
        push @lsp_diags, \%diag;
    }

    my %notif_params = (
        uri         => $uri,
        diagnostics => \@lsp_diags,
    );
    $notif_params{version} = $version if $version > 0;

    $server->send_notification('textDocument/publishDiagnostics', \%notif_params);
}

# _parse_uri($params) -> $uri
sub _parse_uri {
    my ($params) = @_;
    my $td = $params->{textDocument};
    return '' unless ref $td eq 'HASH';
    return $td->{uri} // '';
}

# _parse_position($params) -> \%position
sub _parse_position {
    my ($params) = @_;
    my $pos = $params->{position} // {};
    return {
        line      => $pos->{line}      // 0,
        character => $pos->{character} // 0,
    };
}

# _parse_lsp_range($raw) -> \%range
sub _parse_lsp_range {
    my ($raw) = @_;
    return {
        start => {
            line      => $raw->{start}{line}      // 0,
            character => $raw->{start}{character} // 0,
        },
        end => {
            line      => $raw->{end}{line}      // 0,
            character => $raw->{end}{character} // 0,
        },
    };
}

# _position_to_lsp($pos) -> \%lsp_pos
sub _position_to_lsp {
    my ($p) = @_;
    return {
        line      => $p->{line},
        character => $p->{character},
    };
}

# _range_to_lsp($range) -> \%lsp_range
sub _range_to_lsp {
    my ($r) = @_;
    return {
        start => _position_to_lsp($r->{start}),
        end   => _position_to_lsp($r->{end}),
    };
}

# _location_to_lsp($loc) -> \%lsp_location
sub _location_to_lsp {
    my ($l) = @_;
    return {
        uri   => $l->{uri},
        range => _range_to_lsp($l->{range}),
    };
}

# _convert_document_symbols(\@symbols) -> \@lsp_symbols
#
# Recursively convert document symbols to LSP format.
sub _convert_document_symbols {
    my ($symbols) = @_;
    my @result;
    for my $sym (@$symbols) {
        my %m = (
            name           => $sym->{name},
            kind           => $sym->{kind},
            range          => _range_to_lsp($sym->{range}),
            selectionRange => _range_to_lsp($sym->{selection_range}),
        );
        if ($sym->{children} && @{$sym->{children}}) {
            $m{children} = _convert_document_symbols($sym->{children});
        }
        push @result, \%m;
    }
    return \@result;
}

1;

__END__

=head1 NAME

CodingAdventures::Ls00::Handlers -- LSP request and notification handlers

=head1 DESCRIPTION

Contains all handler functions for LSP methods.  These are registered by
the Server module with the JSON-RPC dispatch loop.

=cut
