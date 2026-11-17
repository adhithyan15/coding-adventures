package CodingAdventures::CanonicalCbor::Types;

use strict;
use warnings;
use v5.28;
use feature 'bitwise';
no warnings 'experimental::bitwise';
no warnings 'recursion';

our $VERSION = '0.1.0';

use B ();
use Scalar::Util ();
use Encode qw(encode decode FB_CROAK LEAVE_SRC);
use CodingAdventures::CanonicalCbor::Constants;
use CodingAdventures::CanonicalCbor::Messages;

# ============================================================================
# coding_adventures.canonical_cbor.types
# ============================================================================
# Inside-out unforgeable algebraic value models representing the 9 genuine CBR01
# kinds, plus Span, SpannedMapEntry, DecodeMapSpansResult, and CborError.
#
# Objects use lexical refaddr registries with DESTROY cleanup, and SV referents
# are locked via Internals::SvREADONLY to completely prevent re-bless polymorphic
# spoofing or scalar mutation.
# ============================================================================

# Lexical registries indexed by Scalar::Util::refaddr($self)
my %VALUE_REGISTRY;
my %MAP_ENTRY_REGISTRY;
my %SPAN_REGISTRY;
my %SPANNED_ENTRY_REGISTRY;
my %RESULT_REGISTRY;
my %ERROR_REGISTRY;

# ----------------------------------------------------------------------------
# CborError Model
# ----------------------------------------------------------------------------

package CodingAdventures::CanonicalCbor::Types::CborError {
    use strict;
    use warnings;
    use overload
        '""'     => sub { $ERROR_REGISTRY{Scalar::Util::refaddr($_[0])}->{message} },
        '=='     => \&_equals,
        'eq'     => \&_equals,
        fallback => 1;

    sub id      { $ERROR_REGISTRY{Scalar::Util::refaddr($_[0])}->{id} }
    sub message { $ERROR_REGISTRY{Scalar::Util::refaddr($_[0])}->{message} }

    sub _equals {
        my ($a, $b) = @_;
        return 0 if !ref($b) || ref($b) ne 'CodingAdventures::CanonicalCbor::Types::CborError'
            || !exists $ERROR_REGISTRY{Scalar::Util::refaddr($b)};
        return $ERROR_REGISTRY{Scalar::Util::refaddr($a)}->{id} eq
               $ERROR_REGISTRY{Scalar::Util::refaddr($b)}->{id} ? 1 : 0;
    }

    sub DESTROY {
        local $@;
        delete $ERROR_REGISTRY{Scalar::Util::refaddr($_[0])};
    }
}

sub CborError {
    my ($id) = @_;
    my $msg = CodingAdventures::CanonicalCbor::Messages::message_for($id);
    my $anon;
    my $self = bless \$anon, 'CodingAdventures::CanonicalCbor::Types::CborError';
    Internals::SvREADONLY($anon, 1);
    $ERROR_REGISTRY{Scalar::Util::refaddr($self)} = {
        id      => $id,
        message => $msg,
    };
    return $self;
}

sub raise_error {
    my ($id) = @_;
    die CborError($id);
}

sub is_cbor_error {
    my ($v) = @_;
    return (ref($v) && ref($v) eq 'CodingAdventures::CanonicalCbor::Types::CborError'
        && exists $ERROR_REGISTRY{Scalar::Util::refaddr($v)}) ? 1 : 0;
}

# ----------------------------------------------------------------------------
# Validation Helpers
# ----------------------------------------------------------------------------

sub _is_valid_u64 {
    return 0 if !defined($_[0]) || ref($_[0]) || tied($_[0]);
    my $sv = B::svref_2object(\$_[0]);
    my $flags = $sv->FLAGS;
    return 0 unless ($flags & B::SVf_IOK) && !($flags & B::SVf_NOK);
    my $val = $_[0];
    return 0 if $val < 0;
    if ($flags & B::SVf_POK) {
        return 0 unless $sv->PV eq sprintf('%u', $val);
    }
    return 1;
}

sub _validate_and_normalize_bytes {
    if (tied($_[0]) || !defined($_[0]) || ref($_[0])) {
        raise_error('unsupported-simple');
    }
    my $bytes = $_[0];
    my $copy = $bytes;
    if (!utf8::downgrade($copy, 1)) {
        raise_error('unsupported-simple');
    }
    return $copy;
}

sub _validate_and_normalize_text {
    if (tied($_[0]) || !defined($_[0]) || ref($_[0])) {
        raise_error('unsupported-simple');
    }
    my $text = $_[0];
    my ($decoded_char_str, $octets);
    if (utf8::is_utf8($text)) {
        $octets = eval { encode('UTF-8', $text, FB_CROAK | LEAVE_SRC) };
        if ($@) {
            raise_error('invalid-utf8');
        }
        $decoded_char_str = $text;
    } else {
        $decoded_char_str = eval { decode('UTF-8', $text, FB_CROAK | LEAVE_SRC) };
        if ($@) {
            raise_error('invalid-utf8');
        }
        $octets = $text;
    }
    return ($decoded_char_str, $octets);
}

# ----------------------------------------------------------------------------
# Span Model
# ----------------------------------------------------------------------------

package CodingAdventures::CanonicalCbor::Types::Span {
    use strict;
    use warnings;
    use overload
        '""'     => sub {
            my $d = $SPAN_REGISTRY{Scalar::Util::refaddr($_[0])};
            return "[$d->{start}, $d->{finish})";
        },
        '=='     => \&_equals,
        'eq'     => \&_equals,
        fallback => 1;

    sub start  { $SPAN_REGISTRY{Scalar::Util::refaddr($_[0])}->{start} }
    sub finish { $SPAN_REGISTRY{Scalar::Util::refaddr($_[0])}->{finish} }
    sub length {
        my $d = $SPAN_REGISTRY{Scalar::Util::refaddr($_[0])};
        return $d->{finish} - $d->{start};
    }

    sub _equals {
        my ($a, $b) = @_;
        return 0 if !ref($b) || ref($b) ne 'CodingAdventures::CanonicalCbor::Types::Span'
            || !exists $SPAN_REGISTRY{Scalar::Util::refaddr($b)};
        my $da = $SPAN_REGISTRY{Scalar::Util::refaddr($a)};
        my $db = $SPAN_REGISTRY{Scalar::Util::refaddr($b)};
        return ($da->{start} == $db->{start} && $da->{finish} == $db->{finish}) ? 1 : 0;
    }

    sub DESTROY {
        local $@;
        delete $SPAN_REGISTRY{Scalar::Util::refaddr($_[0])};
    }
}

sub Span {
    if (!_is_valid_u64($_[0]) || !_is_valid_u64($_[1]) || $_[0] > $_[1]) {
        raise_error('unsupported-simple');
    }
    my ($start, $finish) = @_;
    my $anon;
    my $self = bless \$anon, 'CodingAdventures::CanonicalCbor::Types::Span';
    Internals::SvREADONLY($anon, 1);
    $SPAN_REGISTRY{Scalar::Util::refaddr($self)} = {
        start  => $start,
        finish => $finish,
    };
    return $self;
}

sub is_span {
    my ($v) = @_;
    return (ref($v) && ref($v) eq 'CodingAdventures::CanonicalCbor::Types::Span'
        && exists $SPAN_REGISTRY{Scalar::Util::refaddr($v)}) ? 1 : 0;
}

# ----------------------------------------------------------------------------
# SpannedMapEntry Model
# ----------------------------------------------------------------------------

package CodingAdventures::CanonicalCbor::Types::SpannedMapEntry {
    use strict;
    use warnings;
    no warnings 'recursion';
    use overload
        '""'     => sub {
            my $d = $SPANNED_ENTRY_REGISTRY{Scalar::Util::refaddr($_[0])};
            my $ph = $CodingAdventures::CanonicalCbor::Messages::REDACTED_PLACEHOLDER;
            return "SpannedMapEntry(key: $ph, value: $ph, value_span: " . $d->{value_span} . ")";
        },
        '=='     => \&_equals,
        'eq'     => \&_equals,
        fallback => 1;

    sub key        { $SPANNED_ENTRY_REGISTRY{Scalar::Util::refaddr($_[0])}->{key} }
    sub value      { $SPANNED_ENTRY_REGISTRY{Scalar::Util::refaddr($_[0])}->{value} }
    sub value_span { $SPANNED_ENTRY_REGISTRY{Scalar::Util::refaddr($_[0])}->{value_span} }

    sub _equals {
        my ($a, $b) = @_;
        return 0 if !ref($b) || ref($b) ne 'CodingAdventures::CanonicalCbor::Types::SpannedMapEntry'
            || !exists $SPANNED_ENTRY_REGISTRY{Scalar::Util::refaddr($b)};
        my $da = $SPANNED_ENTRY_REGISTRY{Scalar::Util::refaddr($a)};
        my $db = $SPANNED_ENTRY_REGISTRY{Scalar::Util::refaddr($b)};
        return ($da->{key} == $db->{key} &&
                $da->{value} == $db->{value} &&
                $da->{value_span} == $db->{value_span}) ? 1 : 0;
    }

    sub DESTROY {
        local $@;
        delete $SPANNED_ENTRY_REGISTRY{Scalar::Util::refaddr($_[0])};
    }
}

sub SpannedMapEntry {
    my ($key, $value, $value_span) = @_;
    if (!is_cbor_value($key) || !is_cbor_value($value) || !is_span($value_span)) {
        raise_error('unsupported-simple');
    }
    my $anon;
    my $self = bless \$anon, 'CodingAdventures::CanonicalCbor::Types::SpannedMapEntry';
    Internals::SvREADONLY($anon, 1);
    $SPANNED_ENTRY_REGISTRY{Scalar::Util::refaddr($self)} = {
        key        => $key,
        value      => $value,
        value_span => $value_span,
    };
    return $self;
}

sub is_spanned_map_entry {
    my ($v) = @_;
    return (ref($v) && ref($v) eq 'CodingAdventures::CanonicalCbor::Types::SpannedMapEntry'
        && exists $SPANNED_ENTRY_REGISTRY{Scalar::Util::refaddr($v)}) ? 1 : 0;
}

# ----------------------------------------------------------------------------
# DecodeMapSpansResult Model
# ----------------------------------------------------------------------------

package CodingAdventures::CanonicalCbor::Types::DecodeMapSpansResult {
    use strict;
    use warnings;
    no warnings 'recursion';
    use overload
        '""'     => sub {
            my $d = $RESULT_REGISTRY{Scalar::Util::refaddr($_[0])};
            return $d->{is_map}
                ? "DecodeMapSpansResult.map(#" . scalar(@{$d->{entries}}) . ")"
                : "DecodeMapSpansResult.non_map";
        },
        '=='     => \&_equals,
        'eq'     => \&_equals,
        fallback => 1;

    sub is_map  { $RESULT_REGISTRY{Scalar::Util::refaddr($_[0])}->{is_map} }
    sub entries {
        my $d = $RESULT_REGISTRY{Scalar::Util::refaddr($_[0])};
        return undef unless $d->{is_map};
        return [ @{$d->{entries}} ];
    }

    sub _equals {
        my ($a, $b) = @_;
        return 0 if !ref($b) || ref($b) ne 'CodingAdventures::CanonicalCbor::Types::DecodeMapSpansResult'
            || !exists $RESULT_REGISTRY{Scalar::Util::refaddr($b)};
        my $da = $RESULT_REGISTRY{Scalar::Util::refaddr($a)};
        my $db = $RESULT_REGISTRY{Scalar::Util::refaddr($b)};
        return 0 if $da->{is_map} != $db->{is_map};
        return 1 if !$da->{is_map};
        my @ea = @{$da->{entries}};
        my @eb = @{$db->{entries}};
        return 0 if @ea != @eb;
        for my $i (0 .. $#ea) {
            return 0 if !($ea[$i] == $eb[$i]);
        }
        return 1;
    }

    sub DESTROY {
        local $@;
        delete $RESULT_REGISTRY{Scalar::Util::refaddr($_[0])};
    }
}

sub is_decode_map_spans_result {
    my ($v) = @_;
    return (ref($v) && ref($v) eq 'CodingAdventures::CanonicalCbor::Types::DecodeMapSpansResult'
        && exists $RESULT_REGISTRY{Scalar::Util::refaddr($v)}) ? 1 : 0;
}

sub _new_decode_map_spans_result {
    my ($is_map, $entries) = @_;
    my $anon;
    my $self = bless \$anon, 'CodingAdventures::CanonicalCbor::Types::DecodeMapSpansResult';
    Internals::SvREADONLY($anon, 1);
    $RESULT_REGISTRY{Scalar::Util::refaddr($self)} = {
        is_map  => $is_map,
        entries => $entries,
    };
    return $self;
}

package CodingAdventures::CanonicalCbor::Types::DecodeMapSpansResultFactory {
    use strict;
    use warnings;

    sub map {
        my ($class, $entries) = @_;
        if (!defined $entries || ref($entries) ne 'ARRAY' || tied(@$entries)) {
            CodingAdventures::CanonicalCbor::Types::raise_error('unsupported-simple');
        }
        my @copy;
        for my $e (@$entries) {
            if (!CodingAdventures::CanonicalCbor::Types::is_spanned_map_entry($e)) {
                CodingAdventures::CanonicalCbor::Types::raise_error('unsupported-simple');
            }
            push @copy, $e;
        }
        return CodingAdventures::CanonicalCbor::Types::_new_decode_map_spans_result(1, \@copy);
    }

    sub non_map {
        my ($class) = @_;
        return CodingAdventures::CanonicalCbor::Types::_new_decode_map_spans_result(0, undef);
    }
}

our $DecodeMapSpansResult = 'CodingAdventures::CanonicalCbor::Types::DecodeMapSpansResultFactory';

# ----------------------------------------------------------------------------
# CborValue Hierarchy (Strictly the 9 Genuine CBR01 Kinds)
# ----------------------------------------------------------------------------

package CodingAdventures::CanonicalCbor::Types::CborValue {
    use strict;
    use warnings;
    no warnings 'recursion';
    use overload
        '""'     => \&_to_string,
        '=='     => \&_equals,
        'eq'     => \&_equals,
        fallback => 1;

    sub kind    { $VALUE_REGISTRY{Scalar::Util::refaddr($_[0])}->{kind} }
    sub value   { $VALUE_REGISTRY{Scalar::Util::refaddr($_[0])}->{value} }
    sub bytes   { $VALUE_REGISTRY{Scalar::Util::refaddr($_[0])}->{bytes} }
    sub text    { $VALUE_REGISTRY{Scalar::Util::refaddr($_[0])}->{text} }
    sub octets  { $VALUE_REGISTRY{Scalar::Util::refaddr($_[0])}->{octets} }
    sub tag     { $VALUE_REGISTRY{Scalar::Util::refaddr($_[0])}->{tag} }

    sub items {
        my $d = $VALUE_REGISTRY{Scalar::Util::refaddr($_[0])};
        return undef unless $d->{kind} eq 'array';
        return [ @{$d->{items}} ];
    }

    sub entries {
        my $d = $VALUE_REGISTRY{Scalar::Util::refaddr($_[0])};
        return undef unless $d->{kind} eq 'map';
        return [ @{$d->{entries}} ];
    }

    sub _to_string {
        my ($self) = @_;
        my $d = $VALUE_REGISTRY{Scalar::Util::refaddr($self)};
        my $k = $d->{kind};
        if ($k eq 'unsigned') {
            return "CborUnsigned($d->{value})";
        }
        if ($k eq 'negative') {
            return "CborNegative($d->{value})";
        }
        if ($k eq 'bytes') {
            return "CborBytes(#" . length($d->{bytes}) . ")";
        }
        if ($k eq 'text') {
            return "CborText(\"$d->{text}\")";
        }
        if ($k eq 'array') {
            return "CborArray(#" . scalar(@{$d->{items}}) . ")";
        }
        if ($k eq 'map') {
            return "CborMap(#" . scalar(@{$d->{entries}}) . ")";
        }
        if ($k eq 'tag') {
            return "CborTag($d->{tag}, $d->{value})";
        }
        if ($k eq 'boolean') {
            return "CborBool(" . ($d->{value} ? 'true' : 'false') . ")";
        }
        if ($k eq 'null') {
            return "CborNull";
        }
        return "CborValue($k)";
    }

    sub _equals {
        my ($a, $b) = @_;
        return 0 if !ref($b) || ref($b) ne 'CodingAdventures::CanonicalCbor::Types::CborValue'
            || !exists $VALUE_REGISTRY{Scalar::Util::refaddr($b)};
        my $da = $VALUE_REGISTRY{Scalar::Util::refaddr($a)};
        my $db = $VALUE_REGISTRY{Scalar::Util::refaddr($b)};
        return 0 if $da->{kind} ne $db->{kind};
        my $k = $da->{kind};
        if ($k eq 'unsigned' || $k eq 'negative') {
            return $da->{value} == $db->{value} ? 1 : 0;
        }
        if ($k eq 'bytes') {
            return $da->{bytes} eq $db->{bytes} ? 1 : 0;
        }
        if ($k eq 'text') {
            return $da->{octets} eq $db->{octets} ? 1 : 0;
        }
        if ($k eq 'boolean') {
            return ($da->{value} ? 1 : 0) == ($db->{value} ? 1 : 0) ? 1 : 0;
        }
        if ($k eq 'null') {
            return 1;
        }
        if ($k eq 'tag') {
            return ($da->{tag} == $db->{tag} && $da->{value} == $db->{value}) ? 1 : 0;
        }
        if ($k eq 'array') {
            my @ia = @{$da->{items}};
            my @ib = @{$db->{items}};
            return 0 if @ia != @ib;
            for my $i (0 .. $#ia) {
                return 0 if !($ia[$i] == $ib[$i]);
            }
            return 1;
        }
        if ($k eq 'map') {
            my @ea = @{$da->{entries}};
            my @eb = @{$db->{entries}};
            return 0 if @ea != @eb;
            for my $i (0 .. $#ea) {
                return 0 if !($ea[$i] == $eb[$i]);
            }
            return 1;
        }
        return 0;
    }

    sub DESTROY {
        local $@;
        delete $VALUE_REGISTRY{Scalar::Util::refaddr($_[0])};
    }
}

sub is_cbor_value {
    my ($v) = @_;
    return (ref($v) && ref($v) eq 'CodingAdventures::CanonicalCbor::Types::CborValue'
        && exists $VALUE_REGISTRY{Scalar::Util::refaddr($v)}) ? 1 : 0;
}

sub _make_cbor_value {
    my ($kind, $data) = @_;
    my $anon;
    my $self = bless \$anon, 'CodingAdventures::CanonicalCbor::Types::CborValue';
    Internals::SvREADONLY($anon, 1);
    $data->{kind} = $kind;
    $VALUE_REGISTRY{Scalar::Util::refaddr($self)} = $data;
    return $self;
}

sub CborUnsigned {
    if (!_is_valid_u64($_[0])) {
        raise_error('unsupported-simple');
    }
    return _make_cbor_value('unsigned', { value => $_[0] });
}

sub CborNegative {
    if (!_is_valid_u64($_[0])) {
        raise_error('unsupported-simple');
    }
    return _make_cbor_value('negative', { value => $_[0] });
}

sub CborBytes {
    my $bytes = _validate_and_normalize_bytes($_[0]);
    return _make_cbor_value('bytes', { bytes => $bytes });
}

sub CborText {
    my ($text, $octets) = _validate_and_normalize_text($_[0]);
    return _make_cbor_value('text', { text => $text, octets => $octets });
}

sub CborArray {
    my ($items) = @_;
    if (!defined $items || ref($items) ne 'ARRAY' || tied(@$items)) {
        raise_error('unsupported-simple');
    }
    my @copy;
    for my $item (@$items) {
        if (!is_cbor_value($item)) {
            raise_error('unsupported-simple');
        }
        push @copy, $item;
    }
    return _make_cbor_value('array', { items => \@copy });
}

# ----------------------------------------------------------------------------
# CborMapEntry (Pair Model — NOT a CborValue)
# ----------------------------------------------------------------------------

package CodingAdventures::CanonicalCbor::Types::CborMapEntry {
    use strict;
    use warnings;
    no warnings 'recursion';
    use overload
        '""'     => sub {
            my $d = $MAP_ENTRY_REGISTRY{Scalar::Util::refaddr($_[0])};
            return "CborMapEntry($d->{key} => $d->{value})";
        },
        '=='     => \&_equals,
        'eq'     => \&_equals,
        fallback => 1;

    sub key   { $MAP_ENTRY_REGISTRY{Scalar::Util::refaddr($_[0])}->{key} }
    sub value { $MAP_ENTRY_REGISTRY{Scalar::Util::refaddr($_[0])}->{value} }

    sub _equals {
        my ($a, $b) = @_;
        return 0 if !ref($b) || ref($b) ne 'CodingAdventures::CanonicalCbor::Types::CborMapEntry'
            || !exists $MAP_ENTRY_REGISTRY{Scalar::Util::refaddr($b)};
        my $da = $MAP_ENTRY_REGISTRY{Scalar::Util::refaddr($a)};
        my $db = $MAP_ENTRY_REGISTRY{Scalar::Util::refaddr($b)};
        return ($da->{key} == $db->{key} && $da->{value} == $db->{value}) ? 1 : 0;
    }

    sub DESTROY {
        local $@;
        delete $MAP_ENTRY_REGISTRY{Scalar::Util::refaddr($_[0])};
    }
}

sub CborMapEntry {
    my ($key, $value) = @_;
    if (!is_cbor_value($key) || !is_cbor_value($value)) {
        raise_error('unsupported-simple');
    }
    my $anon;
    my $self = bless \$anon, 'CodingAdventures::CanonicalCbor::Types::CborMapEntry';
    Internals::SvREADONLY($anon, 1);
    $MAP_ENTRY_REGISTRY{Scalar::Util::refaddr($self)} = {
        key   => $key,
        value => $value,
    };
    return $self;
}

sub is_cbor_map_entry {
    my ($v) = @_;
    return (ref($v) && ref($v) eq 'CodingAdventures::CanonicalCbor::Types::CborMapEntry'
        && exists $MAP_ENTRY_REGISTRY{Scalar::Util::refaddr($v)}) ? 1 : 0;
}

sub CborMap {
    my ($entries) = @_;
    if (!defined $entries || ref($entries) ne 'ARRAY' || tied(@$entries)) {
        raise_error('unsupported-simple');
    }
    my @copy;
    for my $entry (@$entries) {
        if (!is_cbor_map_entry($entry)) {
            raise_error('unsupported-simple');
        }
        push @copy, $entry;
    }
    return _make_cbor_value('map', { entries => \@copy });
}

sub CborTag {
    if (!_is_valid_u64($_[0]) || !is_cbor_value($_[1])) {
        raise_error('unsupported-simple');
    }
    my ($tag, $value) = @_;
    return _make_cbor_value('tag', { tag => $tag, value => $value });
}

sub CborBool {
    if (tied($_[0]) || !defined($_[0]) || ref($_[0])) {
        raise_error('unsupported-simple');
    }
    my $value = $_[0];
    my $bool = ($value && $value ne '0' && $value ne '') ? 1 : 0;
    return _make_cbor_value('boolean', { value => $bool });
}

sub CborNull {
    return _make_cbor_value('null', {});
}

1;
