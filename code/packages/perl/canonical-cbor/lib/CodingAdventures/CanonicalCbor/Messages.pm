package CodingAdventures::CanonicalCbor::Messages;

use strict;
use warnings;
use v5.28;

our $VERSION = '0.1.0';

# ============================================================================
# coding_adventures.canonical_cbor.messages
# ============================================================================
# Single source of truth for all 14 closed payload-blind CBR01 error diagnostics
# and security redaction placeholders (Standing Rule #20).
#
# No user-supplied data, keys, values, offsets, or sizes are ever reflected in
# error diagnostics, guaranteeing defense-in-depth against secret leakage.
# ============================================================================

our $REDACTED_PLACEHOLDER = '<redacted>';

# Fixed payload-blind diagnostic when an invalid error ID is requested.
our $UNKNOWN_ERROR_ID_MESSAGE = 'canonical-cbor: unknown error id';

our %ERROR_MESSAGES = (
    'unexpected-eof'          => 'canonical-cbor: unexpected end of input',
    'trailing-bytes'          => 'canonical-cbor: trailing bytes after decoded item',
    'reserved'                => 'canonical-cbor: reserved additional-info value',
    'indefinite'              => 'canonical-cbor: indefinite item rejected',
    'non-minimal-integer'     => 'canonical-cbor: argument is not in smallest form',
    'invalid-utf8'            => 'canonical-cbor: text is not valid UTF-8',
    'non-canonical-map-order' => 'canonical-cbor: map key order is not canonical',
    'unsupported-simple'      => 'canonical-cbor: unsupported simple value',
    'float-not-supported'     => 'canonical-cbor: floats are not supported',
    'too-deep'                => 'canonical-cbor: decoded nesting is too deep',
    'length-too-large'        => 'canonical-cbor: declared length is too large',
    'duplicate-map-key'       => 'canonical-cbor: duplicate canonical map key',
    'encode-too-deep'         => 'canonical-cbor: encoded nesting is too deep',
    'encode-too-large'        => 'canonical-cbor: encoded item is too large',
);

# Lock globals to prevent runtime tampering
Internals::SvREADONLY($REDACTED_PLACEHOLDER, 1);
Internals::SvREADONLY($UNKNOWN_ERROR_ID_MESSAGE, 1);
for my $v (values %ERROR_MESSAGES) {
    Internals::SvREADONLY($v, 1);
}
Internals::SvREADONLY(%ERROR_MESSAGES, 1);

# Return the payload-blind diagnostic message for a given error ID.
# Never reflects the input argument if invalid.
sub message_for {
    my ($id) = @_;
    if (!defined $id || !exists $ERROR_MESSAGES{$id}) {
        die "$UNKNOWN_ERROR_ID_MESSAGE\n";
    }
    return $ERROR_MESSAGES{$id};
}

# Return whether the given ID is one of the 14 valid closed CBR01 error IDs.
sub is_valid_id {
    my ($id) = @_;
    return (defined $id && exists $ERROR_MESSAGES{$id}) ? 1 : 0;
}

1;
