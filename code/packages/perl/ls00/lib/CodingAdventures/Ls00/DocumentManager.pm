package CodingAdventures::Ls00::DocumentManager;

# ============================================================================
# CodingAdventures::Ls00::DocumentManager -- Tracks open file contents
# ============================================================================
#
# # The Document Manager's Job
#
# When the user opens a file in VS Code, the editor sends a textDocument/didOpen
# notification with the full file content.  From that point on, the editor does
# NOT re-send the entire file on every keystroke.  Instead, it sends incremental
# changes: what changed, and where.  The DocumentManager applies these changes
# to maintain the current text of each open file.
#
#   Editor opens file:   didOpen   -> DocumentManager stores text at version 1
#   User types "X":      didChange -> DocumentManager applies delta -> version 2
#   User saves:          didSave   -> (optional: trigger format)
#   User closes:         didClose  -> DocumentManager removes entry
#
# # Why Version Numbers?
#
# The editor increments the version number with every change.  The ParseCache
# uses (uri, version) as its cache key -- if the version matches, the cached
# parse result is still valid.
#
# # UTF-16: The Tricky Part
#
# LSP specifies that character offsets are measured in UTF-16 CODE UNITS.
# This is a historical accident: VS Code uses TypeScript, which uses UTF-16
# strings internally.
#
# Perl strings are sequences of characters (codepoints).  A single Unicode
# codepoint can occupy:
#   - 1 byte in UTF-8 (ASCII, e.g. 'A')
#   - 2 bytes in UTF-8 (e.g. 'e-acute', U+00E9)
#   - 3 bytes in UTF-8 (e.g. 'middle-dot-CJK', U+4E2D)
#   - 4 bytes in UTF-8 (e.g. guitar emoji, U+1F3B8)
#
# In UTF-16:
#   - Codepoints in the Basic Multilingual Plane (U+0000-U+FFFF) -> 1 code unit
#   - Codepoints above U+FFFF (emojis, rare CJK) -> 2 code units (surrogate pair)
#
# The function convert_utf16_offset_to_byte_offset() below performs this
# conversion.

use strict;
use warnings;
use Encode ();

use Exporter 'import';
our @EXPORT_OK = qw(convert_utf16_offset_to_byte_offset);

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# new() -> DocumentManager
#
# Create an empty DocumentManager.
# ---------------------------------------------------------------------------

sub new {
    my ($class) = @_;
    return bless { docs => {} }, $class;
}

# ---------------------------------------------------------------------------
# open($uri, $text, $version)
#
# Record a newly opened file.  Called when the editor sends didOpen.
# Stores the initial text and version number.
# ---------------------------------------------------------------------------

sub open {
    my ($self, $uri, $text, $version) = @_;
    $self->{docs}{$uri} = {
        uri     => $uri,
        text    => $text,
        version => $version,
    };
}

# ---------------------------------------------------------------------------
# get($uri) -> ($doc_hashref, $found_bool)
#
# Return the document for a URI.  In list context, returns ($doc, 1) if
# found, or (undef, 0) if the document is not open.
# ---------------------------------------------------------------------------

sub get {
    my ($self, $uri) = @_;
    if (exists $self->{docs}{$uri}) {
        return ($self->{docs}{$uri}, 1);
    }
    return (undef, 0);
}

# ---------------------------------------------------------------------------
# close($uri)
#
# Remove a document from the manager.  Called when the editor sends didClose.
# ---------------------------------------------------------------------------

sub close {
    my ($self, $uri) = @_;
    delete $self->{docs}{$uri};
}

# ---------------------------------------------------------------------------
# apply_changes($uri, \@changes, $version) -> $error_or_undef
#
# Apply a list of incremental changes to an open document.
#
# Each change is a hashref:
#   { range => { start => {line=>N, character=>N}, end => ... }, new_text => "..." }
#   If range is undef, new_text replaces the entire document (full sync).
#
# Returns an error string if the document is not open or a range is invalid.
# Returns undef on success.
# ---------------------------------------------------------------------------

sub apply_changes {
    my ($self, $uri, $changes, $version) = @_;

    my $doc = $self->{docs}{$uri};
    unless ($doc) {
        return "document not open: $uri";
    }

    for my $change (@$changes) {
        if (!defined $change->{range}) {
            # Full document replacement -- simplest case.
            $doc->{text} = $change->{new_text};
        } else {
            # Incremental update: splice new text at the specified range.
            my ($new_text, $err) = _apply_range_change(
                $doc->{text}, $change->{range}, $change->{new_text}
            );
            if ($err) {
                return "applying change to $uri: $err";
            }
            $doc->{text} = $new_text;
        }
    }

    $doc->{version} = $version;
    return undef;
}

# ---------------------------------------------------------------------------
# _apply_range_change($text, $range, $new_text) -> ($result_text, $err)
#
# Splice new_text into text at the given LSP range.  Converts LSP's
# (line, UTF-16 character) coordinates to byte offsets in the UTF-8 string.
# ---------------------------------------------------------------------------

sub _apply_range_change {
    my ($text, $range, $new_text) = @_;

    my ($start_byte, $err1) = _convert_position_to_byte_offset($text, $range->{start});
    return (undef, "start position: $err1") if $err1;

    my ($end_byte, $err2) = _convert_position_to_byte_offset($text, $range->{end});
    return (undef, "end position: $err2") if $err2;

    if ($start_byte > $end_byte) {
        return (undef, "start offset $start_byte > end offset $end_byte");
    }

    my $text_bytes = Encode::encode('UTF-8', $text);
    my $byte_len = length($text_bytes);
    $end_byte = $byte_len if $end_byte > $byte_len;

    # Perform the splice in byte space, then decode back.
    my $before = substr($text_bytes, 0, $start_byte);
    my $after  = substr($text_bytes, $end_byte);
    my $new_text_bytes = Encode::encode('UTF-8', $new_text);
    my $result_bytes = $before . $new_text_bytes . $after;

    return (Encode::decode('UTF-8', $result_bytes), undef);
}

# ---------------------------------------------------------------------------
# _convert_position_to_byte_offset($text, $pos) -> ($byte_offset, $err)
#
# Convert an LSP Position (0-based line, UTF-16 character) to a byte offset
# in the UTF-8 encoded string.
#
# Algorithm:
#   1. Walk line-by-line to find the byte offset of the start of the target line.
#   2. From that offset, walk UTF-8 codepoints converting each to its UTF-16
#      length until we reach the target UTF-16 character offset.
# ---------------------------------------------------------------------------

sub _convert_position_to_byte_offset {
    my ($text, $pos) = @_;

    my $line = $pos->{line} // 0;
    my $char = $pos->{character} // 0;

    # Encode to bytes for byte-level manipulation.
    my $bytes = Encode::encode('UTF-8', $text);
    my $byte_len = length($bytes);

    # Phase 1: find the byte offset of the start of the target line.
    my $line_start = 0;
    my $current_line = 0;

    while ($current_line < $line) {
        my $idx = index($bytes, "\n", $line_start);
        if ($idx == -1) {
            # Line number exceeds number of lines -- clamp to end.
            return ($byte_len, undef);
        }
        $line_start = $idx + 1;
        $current_line++;
    }

    # Phase 2: from line_start, advance $char UTF-16 code units.
    my $byte_offset = $line_start;
    my $utf16_units = 0;

    while ($utf16_units < $char && $byte_offset < $byte_len) {
        # Check for newline -- don't advance past line end.
        last if substr($bytes, $byte_offset, 1) eq "\n";

        # Decode one UTF-8 codepoint.
        my ($codepoint, $size) = _decode_utf8_codepoint($bytes, $byte_offset);

        # How many UTF-16 code units does this codepoint occupy?
        my $utf16_len = _utf16_unit_length($codepoint);

        # Would this codepoint overshoot the target?
        last if $utf16_units + $utf16_len > $char;

        $byte_offset += $size;
        $utf16_units += $utf16_len;
    }

    return ($byte_offset, undef);
}

# ---------------------------------------------------------------------------
# convert_utf16_offset_to_byte_offset($text, $line, $char) -> $byte_offset
#
# Exported version of the UTF-16 to byte offset conversion, for use in
# tests and external packages.
#
# # Why UTF-16?
#
# LSP character offsets are UTF-16 code units because VS Code's internal
# string representation is UTF-16 (as is JavaScript's String type).
#
# # Example
#
#   my $text = "hello \x{1F3B8} world";
#   # Guitar emoji (U+1F3B8) is 4 UTF-8 bytes but 2 UTF-16 code units.
#   # After the emoji, LSP says character=8 (6 for "hello ", 2 for emoji).
#   my $byte_off = convert_utf16_offset_to_byte_offset($text, 0, 8);
#   # $byte_off = 11
# ---------------------------------------------------------------------------

sub convert_utf16_offset_to_byte_offset {
    my ($text, $line, $char) = @_;
    my ($offset, $err) = _convert_position_to_byte_offset(
        $text, { line => $line, character => $char }
    );
    return $offset;
}

# ---------------------------------------------------------------------------
# _utf16_unit_length($codepoint) -> 1 or 2
#
# Returns the number of UTF-16 code units required to encode a Unicode
# codepoint.
#
# BMP codepoints (U+0000-U+FFFF): 1 code unit
# Non-BMP codepoints (U+10000+):  2 code units (surrogate pair)
# ---------------------------------------------------------------------------

sub _utf16_unit_length {
    my ($cp) = @_;
    return ($cp > 0xFFFF) ? 2 : 1;
}

# ---------------------------------------------------------------------------
# _decode_utf8_codepoint($bytes, $offset) -> ($codepoint, $byte_count)
#
# Decode one UTF-8 codepoint starting at $offset in the byte string $bytes.
# Returns the Unicode codepoint value and the number of bytes consumed.
#
# UTF-8 encoding rules:
#   0xxxxxxx                            -> 1 byte  (ASCII, U+0000-U+007F)
#   110xxxxx 10xxxxxx                   -> 2 bytes (U+0080-U+07FF)
#   1110xxxx 10xxxxxx 10xxxxxx          -> 3 bytes (U+0800-U+FFFF)
#   11110xxx 10xxxxxx 10xxxxxx 10xxxxxx -> 4 bytes (U+10000-U+10FFFF)
# ---------------------------------------------------------------------------

sub _decode_utf8_codepoint {
    my ($bytes, $offset) = @_;

    my $byte = ord(substr($bytes, $offset, 1));

    if ($byte < 0x80) {
        # Single-byte ASCII character.
        return ($byte, 1);
    } elsif ($byte < 0xC0) {
        # Continuation byte found at start -- invalid, treat as 1 byte.
        return ($byte, 1);
    } elsif ($byte < 0xE0) {
        # 2-byte sequence.
        my $b2 = ord(substr($bytes, $offset + 1, 1));
        my $cp = (($byte & 0x1F) << 6) | ($b2 & 0x3F);
        return ($cp, 2);
    } elsif ($byte < 0xF0) {
        # 3-byte sequence.
        my $b2 = ord(substr($bytes, $offset + 1, 1));
        my $b3 = ord(substr($bytes, $offset + 2, 1));
        my $cp = (($byte & 0x0F) << 12) | (($b2 & 0x3F) << 6) | ($b3 & 0x3F);
        return ($cp, 3);
    } else {
        # 4-byte sequence.
        my $b2 = ord(substr($bytes, $offset + 1, 1));
        my $b3 = ord(substr($bytes, $offset + 2, 1));
        my $b4 = ord(substr($bytes, $offset + 3, 1));
        my $cp = (($byte & 0x07) << 18) | (($b2 & 0x3F) << 12)
               | (($b3 & 0x3F) << 6)    | ($b4 & 0x3F);
        return ($cp, 4);
    }
}

1;

__END__

=head1 NAME

CodingAdventures::Ls00::DocumentManager -- Tracks open documents

=head1 SYNOPSIS

  use CodingAdventures::Ls00::DocumentManager;

  my $dm = CodingAdventures::Ls00::DocumentManager->new();
  $dm->open("file:///test.txt", "hello world", 1);

  my ($doc, $found) = $dm->get("file:///test.txt");
  print $doc->{text};  # "hello world"

  $dm->close("file:///test.txt");

=head1 DESCRIPTION

Maintains the current text content of all files open in the editor.  Handles
both full-document replacement and incremental (range-based) changes.

=cut
