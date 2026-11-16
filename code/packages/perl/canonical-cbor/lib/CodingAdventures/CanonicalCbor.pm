package CodingAdventures::CanonicalCbor;

use strict;
use warnings;
use v5.28;

our $VERSION = '0.1.0';

use parent 'Exporter';

use CodingAdventures::CanonicalCbor::Constants;
use CodingAdventures::CanonicalCbor::Messages;
use CodingAdventures::CanonicalCbor::Types;
use CodingAdventures::CanonicalCbor::Codec;

our @EXPORT_OK = qw(
    encode_checked
    encode_into_checked
    decode
    decode_map_spans
    CborUnsigned
    CborNegative
    CborBytes
    CborText
    CborArray
    CborMapEntry
    CborMap
    CborTag
    CborBool
    CborNull
    Span
    SpannedMapEntry
    DecodeMapSpansResult
    CborError
    raise_error
    is_cbor_value
    is_cbor_map_entry
    is_span
    is_spanned_map_entry
    is_decode_map_spans_result
    is_cbor_error
);

our %EXPORT_TAGS = (
    all => \@EXPORT_OK,
);

# Re-export codec functions
sub encode_checked      { CodingAdventures::CanonicalCbor::Codec::encode_checked(@_) }
sub encode_into_checked { CodingAdventures::CanonicalCbor::Codec::encode_into_checked(@_) }
sub decode              { CodingAdventures::CanonicalCbor::Codec::decode(@_) }
sub decode_map_spans    { CodingAdventures::CanonicalCbor::Codec::decode_map_spans(@_) }

# Re-export type constructors and type checkers
sub CborUnsigned               { CodingAdventures::CanonicalCbor::Types::CborUnsigned(@_) }
sub CborNegative               { CodingAdventures::CanonicalCbor::Types::CborNegative(@_) }
sub CborBytes                  { CodingAdventures::CanonicalCbor::Types::CborBytes(@_) }
sub CborText                   { CodingAdventures::CanonicalCbor::Types::CborText(@_) }
sub CborArray                  { CodingAdventures::CanonicalCbor::Types::CborArray(@_) }
sub CborMapEntry               { CodingAdventures::CanonicalCbor::Types::CborMapEntry(@_) }
sub CborMap                    { CodingAdventures::CanonicalCbor::Types::CborMap(@_) }
sub CborTag                    { CodingAdventures::CanonicalCbor::Types::CborTag(@_) }
sub CborBool                   { CodingAdventures::CanonicalCbor::Types::CborBool(@_) }
sub CborNull                   { CodingAdventures::CanonicalCbor::Types::CborNull(@_) }
sub Span                       { CodingAdventures::CanonicalCbor::Types::Span(@_) }
sub SpannedMapEntry            { CodingAdventures::CanonicalCbor::Types::SpannedMapEntry(@_) }
sub DecodeMapSpansResult       { $CodingAdventures::CanonicalCbor::Types::DecodeMapSpansResult }
sub CborError                  { CodingAdventures::CanonicalCbor::Types::CborError(@_) }
sub raise_error                { CodingAdventures::CanonicalCbor::Types::raise_error(@_) }
sub is_cbor_value              { CodingAdventures::CanonicalCbor::Types::is_cbor_value(@_) }
sub is_cbor_map_entry          { CodingAdventures::CanonicalCbor::Types::is_cbor_map_entry(@_) }
sub is_span                    { CodingAdventures::CanonicalCbor::Types::is_span(@_) }
sub is_spanned_map_entry       { CodingAdventures::CanonicalCbor::Types::is_spanned_map_entry(@_) }
sub is_decode_map_spans_result { CodingAdventures::CanonicalCbor::Types::is_decode_map_spans_result(@_) }
sub is_cbor_error              { CodingAdventures::CanonicalCbor::Types::is_cbor_error(@_) }

1;
