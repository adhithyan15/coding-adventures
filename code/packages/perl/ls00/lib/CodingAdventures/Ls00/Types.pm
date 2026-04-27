package CodingAdventures::Ls00::Types;

# ============================================================================
# CodingAdventures::Ls00::Types -- All LSP data types
# ============================================================================
#
# These types mirror the LSP specification's TypeScript type definitions,
# translated to idiomatic Perl.  Each type is a plain hashref (or blessed
# hashref for constructors).  The LSP spec lives at:
# https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
#
# # Coordinate System
#
# LSP uses a 0-based, line/character coordinate system.  Line 0, character 0
# is the very first character of the file.  This differs from most editors
# (which display 1-based line numbers) and from our lexer (which emits
# 1-based tokens).  The bridge is responsible for converting.
#
# # UTF-16 Code Units
#
# LSP's "character" offset is measured in UTF-16 CODE UNITS, not bytes or
# Unicode codepoints.  This is a historical artifact: VS Code is built on
# TypeScript, which uses UTF-16 strings internally.  See DocumentManager.pm
# for the conversion function and a detailed explanation.

use strict;
use warnings;

use Exporter 'import';

our @EXPORT_OK = qw(
    position
    lsp_range
    location
    diagnostic
    token
    text_edit
    workspace_edit
    hover_result
    completion_item
    semantic_token
    document_symbol
    folding_range
    parameter_information
    signature_information
    signature_help_result

    SEVERITY_ERROR
    SEVERITY_WARNING
    SEVERITY_INFORMATION
    SEVERITY_HINT

    COMPLETION_TEXT
    COMPLETION_METHOD
    COMPLETION_FUNCTION
    COMPLETION_CONSTRUCTOR
    COMPLETION_FIELD
    COMPLETION_VARIABLE
    COMPLETION_CLASS
    COMPLETION_INTERFACE
    COMPLETION_MODULE
    COMPLETION_PROPERTY
    COMPLETION_UNIT
    COMPLETION_VALUE
    COMPLETION_ENUM
    COMPLETION_KEYWORD
    COMPLETION_SNIPPET
    COMPLETION_COLOR
    COMPLETION_FILE
    COMPLETION_REFERENCE
    COMPLETION_FOLDER
    COMPLETION_ENUM_MEMBER
    COMPLETION_CONSTANT
    COMPLETION_STRUCT
    COMPLETION_EVENT
    COMPLETION_OPERATOR
    COMPLETION_TYPE_PARAMETER

    SYMBOL_FILE
    SYMBOL_MODULE
    SYMBOL_NAMESPACE
    SYMBOL_PACKAGE
    SYMBOL_CLASS
    SYMBOL_METHOD
    SYMBOL_PROPERTY
    SYMBOL_FIELD
    SYMBOL_CONSTRUCTOR
    SYMBOL_ENUM
    SYMBOL_INTERFACE
    SYMBOL_FUNCTION
    SYMBOL_VARIABLE
    SYMBOL_CONSTANT
    SYMBOL_STRING
    SYMBOL_NUMBER
    SYMBOL_BOOLEAN
    SYMBOL_ARRAY
    SYMBOL_OBJECT
    SYMBOL_KEY
    SYMBOL_NULL
    SYMBOL_ENUM_MEMBER
    SYMBOL_STRUCT
    SYMBOL_EVENT
    SYMBOL_OPERATOR
    SYMBOL_TYPE_PARAMETER
);

our %EXPORT_TAGS = ( all => \@EXPORT_OK );

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# Diagnostic Severity Constants
#
# These match the LSP integer codes:
#   1 = Error   -- a hard error; the code cannot run or compile
#   2 = Warning -- potentially problematic, but not blocking
#   3 = Information -- informational message
#   4 = Hint    -- a suggestion (e.g., "consider using const")
# ---------------------------------------------------------------------------

use constant SEVERITY_ERROR       => 1;
use constant SEVERITY_WARNING     => 2;
use constant SEVERITY_INFORMATION => 3;
use constant SEVERITY_HINT        => 4;

# ---------------------------------------------------------------------------
# Completion Item Kind Constants
#
# These classify completion items so the editor can show the right icon
# (function icon, variable icon, keyword icon, etc.).
# ---------------------------------------------------------------------------

use constant COMPLETION_TEXT           => 1;
use constant COMPLETION_METHOD         => 2;
use constant COMPLETION_FUNCTION       => 3;
use constant COMPLETION_CONSTRUCTOR    => 4;
use constant COMPLETION_FIELD          => 5;
use constant COMPLETION_VARIABLE       => 6;
use constant COMPLETION_CLASS          => 7;
use constant COMPLETION_INTERFACE      => 8;
use constant COMPLETION_MODULE         => 9;
use constant COMPLETION_PROPERTY       => 10;
use constant COMPLETION_UNIT           => 11;
use constant COMPLETION_VALUE          => 12;
use constant COMPLETION_ENUM           => 13;
use constant COMPLETION_KEYWORD        => 14;
use constant COMPLETION_SNIPPET        => 15;
use constant COMPLETION_COLOR          => 16;
use constant COMPLETION_FILE           => 17;
use constant COMPLETION_REFERENCE      => 18;
use constant COMPLETION_FOLDER         => 19;
use constant COMPLETION_ENUM_MEMBER    => 20;
use constant COMPLETION_CONSTANT       => 21;
use constant COMPLETION_STRUCT         => 22;
use constant COMPLETION_EVENT          => 23;
use constant COMPLETION_OPERATOR       => 24;
use constant COMPLETION_TYPE_PARAMETER => 25;

# ---------------------------------------------------------------------------
# Symbol Kind Constants
#
# These classify document symbols for the outline panel.
# ---------------------------------------------------------------------------

use constant SYMBOL_FILE           => 1;
use constant SYMBOL_MODULE         => 2;
use constant SYMBOL_NAMESPACE      => 3;
use constant SYMBOL_PACKAGE        => 4;
use constant SYMBOL_CLASS          => 5;
use constant SYMBOL_METHOD         => 6;
use constant SYMBOL_PROPERTY       => 7;
use constant SYMBOL_FIELD          => 8;
use constant SYMBOL_CONSTRUCTOR    => 9;
use constant SYMBOL_ENUM           => 10;
use constant SYMBOL_INTERFACE      => 11;
use constant SYMBOL_FUNCTION       => 12;
use constant SYMBOL_VARIABLE       => 13;
use constant SYMBOL_CONSTANT       => 14;
use constant SYMBOL_STRING         => 15;
use constant SYMBOL_NUMBER         => 16;
use constant SYMBOL_BOOLEAN        => 17;
use constant SYMBOL_ARRAY          => 18;
use constant SYMBOL_OBJECT         => 19;
use constant SYMBOL_KEY            => 20;
use constant SYMBOL_NULL           => 21;
use constant SYMBOL_ENUM_MEMBER    => 22;
use constant SYMBOL_STRUCT         => 23;
use constant SYMBOL_EVENT          => 24;
use constant SYMBOL_OPERATOR       => 25;
use constant SYMBOL_TYPE_PARAMETER => 26;

# ---------------------------------------------------------------------------
# Constructor Functions
#
# Each constructor returns a plain hashref.  This is idiomatic Perl -- no
# need for OO ceremony when the data is just a record with named fields.
#
# All constructors use named parameters (key => value) for clarity.
# ---------------------------------------------------------------------------

# position(line => N, character => N) -> hashref
#
# A cursor position in a document.  Both line and character are 0-based.
# Character is measured in UTF-16 code units (see DocumentManager for why).
#
# Example: in "hello world", the 'w' in 'world' is at position(line => 0, character => 6).

sub position {
    my (%args) = @_;
    return {
        line      => $args{line}      // 0,
        character => $args{character} // 0,
    };
}

# lsp_range(start => $pos, end => $pos) -> hashref
#
# A span of text from start (inclusive) to end (exclusive).
# Think of it like a text selection: start is where the cursor lands
# when you click, end is where you drag to.
#
# Named "lsp_range" to avoid collision with Perl's built-in range operator.

sub lsp_range {
    my (%args) = @_;
    return {
        start => $args{start} // position(),
        end   => $args{end}   // position(),
    };
}

# location(uri => $str, range => $range) -> hashref
#
# A position in a specific file.  URI uses the "file://" scheme.

sub location {
    my (%args) = @_;
    return {
        uri   => $args{uri}   // '',
        range => $args{range} // lsp_range(),
    };
}

# diagnostic(range => $range, severity => N, message => $str, code => $str) -> hashref
#
# An error, warning, or hint to display in the editor.  The editor renders
# diagnostics as underlined squiggles, with the message shown on hover.

sub diagnostic {
    my (%args) = @_;
    return {
        range    => $args{range}    // lsp_range(),
        severity => $args{severity} // SEVERITY_ERROR,
        message  => $args{message}  // '',
        (defined $args{code} ? (code => $args{code}) : ()),
    };
}

# token(type => $str, value => $str, line => N, column => N) -> hashref
#
# A single lexical token from the language's lexer.  Line and Column are
# 1-based (matching most lexers).  The bridge must convert to 0-based when
# building semantic tokens.

sub token {
    my (%args) = @_;
    return {
        type   => $args{type}   // '',
        value  => $args{value}  // '',
        line   => $args{line}   // 1,
        column => $args{column} // 1,
    };
}

# text_edit(range => $range, new_text => $str) -> hashref
#
# A single text replacement in a document.  If new_text is empty, the range
# is deleted.

sub text_edit {
    my (%args) = @_;
    return {
        range    => $args{range}    // lsp_range(),
        new_text => $args{new_text} // '',
    };
}

# workspace_edit(changes => { uri => [text_edit, ...] }) -> hashref
#
# Groups text edits across potentially multiple files.

sub workspace_edit {
    my (%args) = @_;
    return {
        changes => $args{changes} // {},
    };
}

# hover_result(contents => $markdown, range => $range_or_undef) -> hashref
#
# Content to show in the hover popup.  Contents is Markdown.

sub hover_result {
    my (%args) = @_;
    return {
        contents => $args{contents} // '',
        (defined $args{range} ? (range => $args{range}) : ()),
    };
}

# completion_item(label => $str, kind => N, ...) -> hashref
#
# A single autocomplete suggestion.

sub completion_item {
    my (%args) = @_;
    my $item = { label => $args{label} // '' };
    $item->{kind}              = $args{kind}              if defined $args{kind};
    $item->{detail}            = $args{detail}            if defined $args{detail};
    $item->{documentation}     = $args{documentation}     if defined $args{documentation};
    $item->{insert_text}       = $args{insert_text}       if defined $args{insert_text};
    $item->{insert_text_format} = $args{insert_text_format} if defined $args{insert_text_format};
    return $item;
}

# semantic_token(line => N, character => N, length => N, token_type => $str, modifiers => [...]) -> hashref
#
# One token's contribution to the semantic highlighting pass.
# Line and character are 0-based.

sub semantic_token {
    my (%args) = @_;
    return {
        line       => $args{line}       // 0,
        character  => $args{character}  // 0,
        length     => $args{length}     // 0,
        token_type => $args{token_type} // '',
        modifiers  => $args{modifiers}  // [],
    };
}

# document_symbol(name => $str, kind => N, range => $range, selection_range => $range, children => [...]) -> hashref
#
# One entry in the document outline panel.

sub document_symbol {
    my (%args) = @_;
    return {
        name            => $args{name}            // '',
        kind            => $args{kind}            // SYMBOL_VARIABLE,
        range           => $args{range}           // lsp_range(),
        selection_range => $args{selection_range} // lsp_range(),
        children        => $args{children}        // [],
    };
}

# folding_range(start_line => N, end_line => N, kind => $str) -> hashref
#
# A collapsible region of the document.

sub folding_range {
    my (%args) = @_;
    my $fr = {
        start_line => $args{start_line} // 0,
        end_line   => $args{end_line}   // 0,
    };
    $fr->{kind} = $args{kind} if defined $args{kind};
    return $fr;
}

# parameter_information(label => $str, documentation => $str) -> hashref

sub parameter_information {
    my (%args) = @_;
    my $pi = { label => $args{label} // '' };
    $pi->{documentation} = $args{documentation} if defined $args{documentation};
    return $pi;
}

# signature_information(label => $str, documentation => $str, parameters => [...]) -> hashref

sub signature_information {
    my (%args) = @_;
    my $si = {
        label      => $args{label}      // '',
        parameters => $args{parameters} // [],
    };
    $si->{documentation} = $args{documentation} if defined $args{documentation};
    return $si;
}

# signature_help_result(signatures => [...], active_signature => N, active_parameter => N) -> hashref

sub signature_help_result {
    my (%args) = @_;
    return {
        signatures       => $args{signatures}       // [],
        active_signature => $args{active_signature} // 0,
        active_parameter => $args{active_parameter} // 0,
    };
}

1;

__END__

=head1 NAME

CodingAdventures::Ls00::Types -- LSP data types as constructor functions

=head1 SYNOPSIS

  use CodingAdventures::Ls00::Types qw(:all);

  my $pos  = position(line => 0, character => 5);
  my $diag = diagnostic(
      range    => lsp_range(start => $pos, end => position(line => 0, character => 10)),
      severity => SEVERITY_ERROR,
      message  => 'unexpected token',
  );

=head1 DESCRIPTION

Provides constructor functions for all LSP data types used by the ls00
framework.  Each function returns a plain hashref.

=cut
