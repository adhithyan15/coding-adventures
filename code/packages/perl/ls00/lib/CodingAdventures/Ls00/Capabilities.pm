package CodingAdventures::Ls00::Capabilities;

# ============================================================================
# CodingAdventures::Ls00::Capabilities -- Build LSP capabilities + semantic tokens
# ============================================================================
#
# # What Are Capabilities?
#
# During the LSP initialize handshake, the server sends back a "capabilities"
# object telling the editor which LSP features it supports.  The editor uses
# this to decide which requests to send.  If a capability is absent, the
# editor won't even try -- so no "Go to Definition" button appears unless
# definitionProvider is true.
#
# Building capabilities dynamically (based on the bridge's can() responses)
# means the server is always honest about what it can do.
#
# # Semantic Token Legend
#
# Semantic tokens use a compact binary encoding.  Instead of sending
# {"type":"keyword"} per token, LSP sends an integer index into a legend.
# The legend must be declared in the capabilities.
#
# # Encoding
#
# LSP encodes semantic tokens as a flat array of integers in 5-tuples:
#   [deltaLine, deltaStartChar, length, tokenTypeIndex, modifierBitmask, ...]
#
# The "delta" encoding makes most values small (often 0 or 1), which
# compresses well.

use strict;
use warnings;

use JSON::PP ();

use Exporter 'import';
our @EXPORT_OK = qw(
    build_capabilities
    semantic_token_legend
    encode_semantic_tokens
    token_type_index
    token_modifier_mask
);
our %EXPORT_TAGS = ( all => \@EXPORT_OK );

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# build_capabilities($bridge) -> \%capabilities
#
# Inspect the bridge at runtime and return the LSP capabilities hashref
# to include in the initialize response.
#
# Uses Perl's $bridge->can('method') to check which optional provider
# methods the bridge implements.
# ---------------------------------------------------------------------------

sub build_capabilities {
    my ($bridge) = @_;

    # textDocumentSync=2 means "incremental": the editor sends only changed
    # ranges, not the full file, on every keystroke.
    my %caps = ( textDocumentSync => 2 );

    if ($bridge->can('hover')) {
        $caps{hoverProvider} = JSON::PP::true;
    }

    if ($bridge->can('definition')) {
        $caps{definitionProvider} = JSON::PP::true;
    }

    if ($bridge->can('references')) {
        $caps{referencesProvider} = JSON::PP::true;
    }

    if ($bridge->can('completion')) {
        $caps{completionProvider} = {
            triggerCharacters => [' ', '.'],
        };
    }

    if ($bridge->can('rename')) {
        $caps{renameProvider} = JSON::PP::true;
    }

    if ($bridge->can('document_symbols')) {
        $caps{documentSymbolProvider} = JSON::PP::true;
    }

    if ($bridge->can('folding_ranges')) {
        $caps{foldingRangeProvider} = JSON::PP::true;
    }

    if ($bridge->can('signature_help')) {
        $caps{signatureHelpProvider} = {
            triggerCharacters => ['(', ','],
        };
    }

    if ($bridge->can('format')) {
        $caps{documentFormattingProvider} = JSON::PP::true;
    }

    if ($bridge->can('semantic_tokens')) {
        $caps{semanticTokensProvider} = {
            legend => semantic_token_legend(),
            full   => JSON::PP::true,
        };
    }

    return \%caps;
}

# ---------------------------------------------------------------------------
# semantic_token_legend() -> \%legend
#
# Return the full legend for all supported semantic token types and
# modifiers.  The ordering matters: index 0 corresponds to "namespace",
# index 1 to "type", etc.
# ---------------------------------------------------------------------------

sub semantic_token_legend {
    return {
        tokenTypes => [
            'namespace',      # 0
            'type',           # 1
            'class',          # 2
            'enum',           # 3
            'interface',      # 4
            'struct',         # 5
            'typeParameter',  # 6
            'parameter',      # 7
            'variable',       # 8
            'property',       # 9
            'enumMember',     # 10
            'event',          # 11
            'function',       # 12
            'method',         # 13
            'macro',          # 14
            'keyword',        # 15
            'modifier',       # 16
            'comment',        # 17
            'string',         # 18
            'number',         # 19
            'regexp',         # 20
            'operator',       # 21
            'decorator',      # 22
        ],
        tokenModifiers => [
            'declaration',    # bit 0
            'definition',     # bit 1
            'readonly',       # bit 2
            'static',         # bit 3
            'deprecated',     # bit 4
            'abstract',       # bit 5
            'async',          # bit 6
            'modification',   # bit 7
            'documentation',  # bit 8
            'defaultLibrary', # bit 9
        ],
    };
}

# ---------------------------------------------------------------------------
# token_type_index($token_type) -> $index or -1
#
# Return the integer index for a semantic token type string.
# Returns -1 if the type is not in the legend (the caller should skip it).
# ---------------------------------------------------------------------------

sub token_type_index {
    my ($token_type) = @_;
    my $legend = semantic_token_legend();
    my $types = $legend->{tokenTypes};
    for my $i (0 .. $#$types) {
        return $i if $types->[$i] eq $token_type;
    }
    return -1;
}

# ---------------------------------------------------------------------------
# token_modifier_mask(\@modifiers) -> $bitmask
#
# Return the bitmask for a list of modifier strings.
#
# The LSP semantic tokens encoding represents modifiers as a bitmask:
#   "declaration" -> bit 0 -> value 1
#   "definition"  -> bit 1 -> value 2
#   both          -> value 3 (bitwise OR)
# ---------------------------------------------------------------------------

sub token_modifier_mask {
    my ($modifiers) = @_;
    return 0 unless $modifiers && @$modifiers;

    my $legend = semantic_token_legend();
    my $mods = $legend->{tokenModifiers};
    my $mask = 0;

    for my $mod (@$modifiers) {
        for my $i (0 .. $#$mods) {
            if ($mods->[$i] eq $mod) {
                $mask |= (1 << $i);
                last;
            }
        }
    }

    return $mask;
}

# ---------------------------------------------------------------------------
# encode_semantic_tokens(\@tokens) -> \@data
#
# Convert an arrayref of semantic token hashrefs to the LSP compact integer
# encoding.
#
# Each input token has: { line, character, length, token_type, modifiers }
# Output is a flat arrayref of integers in 5-tuples:
#   [deltaLine, deltaStartChar, length, tokenTypeIndex, modifierBitmask, ...]
#
# Note: when deltaLine > 0, deltaStartChar is absolute (relative to column 0
# of the new line).  When deltaLine == 0, deltaStartChar is relative to the
# previous token's start character.
# ---------------------------------------------------------------------------

sub encode_semantic_tokens {
    my ($tokens) = @_;

    return [] unless $tokens && @$tokens;

    # Sort by (line, character) ascending.  The delta encoding requires
    # tokens to be in document order.
    my @sorted = sort {
        $a->{line} <=> $b->{line}
        || $a->{character} <=> $b->{character}
    } @$tokens;

    my @data;
    my $prev_line = 0;
    my $prev_char = 0;

    for my $tok (@sorted) {
        my $type_idx = token_type_index($tok->{token_type});
        next if $type_idx == -1;  # Unknown token type -- skip.

        my $delta_line = $tok->{line} - $prev_line;
        my $delta_char;
        if ($delta_line == 0) {
            # Same line: character offset is relative to previous token.
            $delta_char = $tok->{character} - $prev_char;
        } else {
            # Different line: character offset is absolute.
            $delta_char = $tok->{character};
        }

        my $mod_mask = token_modifier_mask($tok->{modifiers});

        push @data, $delta_line, $delta_char, $tok->{length}, $type_idx, $mod_mask;

        $prev_line = $tok->{line};
        $prev_char = $tok->{character};
    }

    return \@data;
}

1;

__END__

=head1 NAME

CodingAdventures::Ls00::Capabilities -- Build LSP capabilities and encode semantic tokens

=head1 SYNOPSIS

  use CodingAdventures::Ls00::Capabilities qw(:all);

  my $caps   = build_capabilities($bridge);
  my $legend = semantic_token_legend();
  my $data   = encode_semantic_tokens(\@tokens);

=head1 DESCRIPTION

Builds the LSP capabilities object dynamically based on the bridge's
implemented methods, and provides the semantic token legend and encoding.

=cut
