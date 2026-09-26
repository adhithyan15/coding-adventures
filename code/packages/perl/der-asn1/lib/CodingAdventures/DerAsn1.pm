package CodingAdventures::DerAsn1;

use strict;
use warnings;
use bytes;
use Exporter qw(import);
use Hash::Util::FieldHash qw(fieldhash);
use Math::BigInt;
use Scalar::Util qw(blessed refaddr);
use CodingAdventures::DerTlv ();

our $VERSION = '0.1.0';
our @EXPORT_OK = qw(default_limits decode_boolean decode_integer decode_bit_string
    decode_octet_string decode_implicit_octet_string decode_ia5_string
    decode_implicit_ia5_string decode_null decode_object_identifier
    decode_implicit_object_identifier);

my $HOST_MAX = 9_223_372_036_854_775_807;
my $HOST_MAX_BIG = Math::BigInt->new('9223372036854775807');
my $U64_MAX = Math::BigInt->new('18446744073709551615');
my %DEFAULT_LIMITS = (
    der => CodingAdventures::DerTlv::default_limits(), max_depth => 32,
    max_total_elements => 16_384, max_oid_arcs => 128,
);

fieldhash my %DECODER_STATE;
fieldhash my %ELEMENT_STATE;
fieldhash my %CURSOR_STATE;
fieldhash my %INTEGER_STATE;
fieldhash my %BIT_STRING_STATE;
fieldhash my %OID_STATE;
fieldhash my %ERROR_STATE;

my $make_object = sub {
    my ($class) = @_;
    my $value = 0;
    return bless \$value, $class;
};
my $copy_der_limits = sub { return {%{ $_[0] }}; };
my $copy_limits = sub {
    my ($limits) = @_;
    return {der => $copy_der_limits->($limits->{der}), max_depth => $limits->{max_depth},
        max_total_elements => $limits->{max_total_elements}, max_oid_arcs => $limits->{max_oid_arcs}};
};
sub default_limits { return $copy_limits->(\%DEFAULT_LIMITS); }

my $non_negative_integer = sub {
    my ($name, $value) = @_;
    die "$name must be a non-negative integer"
        if !defined($value) || ref($value) || $value !~ /\A\d+\z/;
    my $exact = Math::BigInt->new("$value");
    die "$name exceeds supported host maximum" if $exact->bcmp($HOST_MAX_BIG) > 0;
    return 0 + $exact->bstr;
};

my $normalize_limits = sub {
    my ($provided) = @_;
    $provided //= {};
    die "limits must be a hash reference" if ref($provided) ne 'HASH';
    my %allowed = map { $_ => 1 } keys %DEFAULT_LIMITS;
    die "unknown limit $_" for grep { !$allowed{$_} } keys %$provided;
    my $der = exists($provided->{der}) ? $provided->{der} : $DEFAULT_LIMITS{der};
    die "der limits must be a hash reference" if ref($der) ne 'HASH';
    my %der_defaults = %{ $DEFAULT_LIMITS{der} };
    die "unknown DER limit $_" for grep { !exists $der_defaults{$_} } keys %$der;
    my %normalized_der = %der_defaults;
    $normalized_der{$_} = $non_negative_integer->("der.$_", $der->{$_}) for keys %$der;
    die "der.max_tag_number must fit u32" if $normalized_der{max_tag_number} > 4_294_967_295;
    my %limits = (der => \%normalized_der, max_depth => $DEFAULT_LIMITS{max_depth},
        max_total_elements => $DEFAULT_LIMITS{max_total_elements}, max_oid_arcs => $DEFAULT_LIMITS{max_oid_arcs});
    for my $name (qw(max_depth max_total_elements max_oid_arcs)) {
        $limits{$name} = $non_negative_integer->($name, $provided->{$name}) if exists $provided->{$name};
    }
    return \%limits;
};

my $error = sub {
    my ($kind, $offset, $framing_kind) = @_;
    my $error = $make_object->('CodingAdventures::DerAsn1::Error');
    $ERROR_STATE{$error} = {kind => $kind, offset => $offset, framing_kind => $framing_kind};
    return $error;
};
my $fail = sub { die $error->(@_); };
my $translate_framing = sub {
    my ($error) = @_;
    $fail->('framing', $error->offset, $error->kind)
        if blessed($error) && $error->isa('CodingAdventures::DerTlv::Error');
    die $error;
};
my $with_framing = sub {
    my ($code) = @_;
    my (@result, $ok, $error);
    { local $@; $ok = eval { @result = $code->(); 1 }; $error = $@; }
    $translate_framing->($error) if !$ok;
    return wantarray ? @result : $result[0];
};

my $decoder_state_for = sub {
    my ($decoder) = @_;
    die "decoder must be a validated Decoder"
        if !blessed($decoder) || !$decoder->isa('CodingAdventures::DerAsn1::Decoder') || !exists $DECODER_STATE{$decoder};
    return $DECODER_STATE{$decoder};
};
my $element_state_for = sub {
    my ($element) = @_;
    die "element must be a validated Element"
        if !blessed($element) || !$element->isa('CodingAdventures::DerAsn1::Element') || !exists $ELEMENT_STATE{$element};
    return $ELEMENT_STATE{$element};
};
my $cursor_state_for = sub {
    my ($cursor) = @_;
    die "cursor must be a validated Cursor"
        if !blessed($cursor) || !$cursor->isa('CodingAdventures::DerAsn1::Cursor') || !exists $CURSOR_STATE{$cursor};
    return $CURSOR_STATE{$cursor};
};
my $integer_state_for = sub {
    die "invalid DerInteger" if !exists $INTEGER_STATE{$_[0]};
    return $INTEGER_STATE{$_[0]};
};
my $bit_string_state_for = sub {
    die "invalid DerBitString" if !exists $BIT_STRING_STATE{$_[0]};
    return $BIT_STRING_STATE{$_[0]};
};
my $oid_state_for = sub {
    die "invalid ObjectIdentifier" if !exists $OID_STATE{$_[0]};
    return $OID_STATE{$_[0]};
};
my $error_state_for = sub {
    die "invalid Error" if !exists $ERROR_STATE{$_[0]};
    return $ERROR_STATE{$_[0]};
};
my $require_owner = sub {
    my ($decoder_state, $element_state) = @_;
    $fail->('decoder-limit-mismatch', 0)
        if refaddr($decoder_state->{owner}) != refaddr($element_state->{owner});
};
my $wrap_element = sub {
    my ($framed, $depth, $owner) = @_;
    my $element = $make_object->('CodingAdventures::DerAsn1::Element');
    my $tag = $framed->tag;
    $ELEMENT_STATE{$element} = {tag => {class => $tag->{class}, constructed => $tag->{constructed} ? 1 : 0,
        number => 0 + $tag->{number}}, header => '' . $framed->header, value => '' . $framed->value,
        encoded => '' . $framed->encoded, depth => $depth, owner => $owner};
    return $element;
};
my $normalize_tag_number = sub {
    my ($number) = @_;
    die "tag_number must be a u32"
        if !defined($number) || ref($number) || $number !~ /\A\d+\z/;
    my $exact = Math::BigInt->new("$number");
    die "tag_number must be a u32" if $exact->bcmp(4_294_967_295) > 0;
    return 0 + $exact->bstr;
};
my $expect_tag = sub {
    my ($state, $class, $constructed, $number) = @_;
    $number = $normalize_tag_number->($number);
    my $tag = $state->{tag};
    return if $tag->{class} eq $class && $tag->{constructed} == ($constructed ? 1 : 0) && $tag->{number} == $number;
    $fail->('unexpected-tag', 0);
};
my $check_child_depth = sub { $fail->('depth-limit-exceeded', 0) if $_[0] + 1 >= $_[1]->{max_depth}; };
my $primitive = sub {
    my ($element, $class, $number) = @_;
    my $state = $element_state_for->($element);
    $expect_tag->($state, $class, 0, $number);
    return ($state->{value}, length($state->{header}));
};
my $octet = sub { return ord(substr($_[0], $_[1], 1)); };

sub decode_boolean {
    my ($value, $offset) = $primitive->($_[0], 'universal', 1);
    $fail->('invalid-boolean-length', $offset) if length($value) != 1;
    my $octet = $octet->($value, 0);
    return 0 if $octet == 0;
    return 1 if $octet == 255;
    $fail->('invalid-boolean-value', $offset);
}
my $validate_integer = sub {
    my ($value, $offset) = @_;
    $fail->('empty-integer', $offset) if length($value) == 0;
    return if length($value) == 1;
    my ($first, $second) = ($octet->($value, 0), $octet->($value, 1));
    $fail->('non-minimal-integer', $offset)
        if ($first == 0 && ($second & 0x80) == 0) || ($first == 255 && ($second & 0x80) != 0);
};
sub decode_integer {
    my ($value, $offset) = $primitive->($_[0], 'universal', 2);
    $validate_integer->($value, $offset);
    my $integer = $make_object->('CodingAdventures::DerAsn1::DerInteger');
    $INTEGER_STATE{$integer} = {bytes => '' . $value, offset => $offset};
    return $integer;
}
sub decode_bit_string {
    my ($value, $offset) = $primitive->($_[0], 'universal', 3);
    $fail->('missing-unused-bit-count', $offset) if length($value) == 0;
    my $unused = $octet->($value, 0);
    my $payload = substr($value, 1);
    $fail->('invalid-unused-bit-count', $offset) if $unused > 7 || (length($payload) == 0 && $unused != 0);
    $fail->('non-zero-bit-padding', $offset + length($value) - 1)
        if length($payload) && $unused && ($octet->($payload, length($payload) - 1) & ((1 << $unused) - 1));
    $fail->('bit-length-overflow', $offset) if length($payload) > int(($HOST_MAX + $unused) / 8);
    my $bits = $make_object->('CodingAdventures::DerAsn1::DerBitString');
    $BIT_STRING_STATE{$bits} = {bytes => '' . $payload, unused_bits => $unused,
        bit_length => (length($payload) * 8) - $unused};
    return $bits;
}
sub decode_octet_string { return scalar(($primitive->($_[0], 'universal', 4))[0]); }
sub decode_implicit_octet_string { return scalar(($primitive->($_[0], 'context-specific', $_[1]))[0]); }
my $ia5 = sub {
    my ($value, $offset) = @_;
    for my $index (0 .. length($value) - 1) {
        $fail->('non-ascii-ia5-string', $offset + $index) if $octet->($value, $index) > 127;
    }
    return '' . $value;
};
sub decode_ia5_string { my ($v, $o) = $primitive->($_[0], 'universal', 22); return $ia5->($v, $o); }
sub decode_implicit_ia5_string { my ($v, $o) = $primitive->($_[0], 'context-specific', $_[1]); return $ia5->($v, $o); }
sub decode_null {
    my ($value, $offset) = $primitive->($_[0], 'universal', 5);
    $fail->('non-empty-null', $offset) if length($value);
    return;
}

my $oid_subidentifier = sub {
    my ($value, $start, $offset) = @_;
    $fail->('non-minimal-object-identifier', $offset + $start) if $octet->($value, $start) == 0x80;
    my $number = Math::BigInt->new(0);
    my $index = $start;
    while (1) {
        $fail->('unterminated-object-identifier', $offset + $index) if $index >= length($value);
        my $octet = $octet->($value, $index);
        my $payload = $octet & 0x7f;
        my $limit = $U64_MAX->copy->bsub($payload)->bdiv(128);
        $fail->('object-identifier-overflow', $offset + $index) if $number->bcmp($limit) > 0;
        $number->bmul(128)->badd($payload);
        ++$index;
        return ($number, $index) if ($octet & 0x80) == 0;
    }
};
my $decode_oid_value = sub {
    my ($value, $offset, $max_arcs) = @_;
    $fail->('empty-object-identifier', $offset) if length($value) == 0;
    my ($first, $index) = $oid_subidentifier->($value, 0, $offset);
    my @arcs = $first->bcmp(40) < 0 ? (Math::BigInt->new(0), $first->copy)
        : $first->bcmp(80) < 0 ? (Math::BigInt->new(1), $first->copy->bsub(40))
        : (Math::BigInt->new(2), $first->copy->bsub(80));
    $fail->('oid-arc-limit-exceeded', $offset) if @arcs > $max_arcs;
    while ($index < length($value)) {
        my $start = $index;
        my ($arc, $next) = $oid_subidentifier->($value, $index, $offset);
        $fail->('oid-arc-limit-exceeded', $offset + $start) if @arcs >= $max_arcs;
        push @arcs, $arc;
        $index = $next;
    }
    my $oid = $make_object->('CodingAdventures::DerAsn1::ObjectIdentifier');
    $OID_STATE{$oid} = {encoded => '' . $value, arcs => [map { $_->bstr } @arcs]};
    return $oid;
};
sub decode_object_identifier {
    my ($element, $limits) = @_;
    my ($value, $offset) = $primitive->($element, 'universal', 6);
    return $decode_oid_value->($value, $offset, $normalize_limits->($limits)->{max_oid_arcs});
}
sub decode_implicit_object_identifier {
    my ($element, $tag_number, $limits) = @_;
    my ($value, $offset) = $primitive->($element, 'context-specific', $tag_number);
    return $decode_oid_value->($value, $offset, $normalize_limits->($limits)->{max_oid_arcs});
}

my $container = sub {
    my ($self, $element, $number) = @_;
    my $decoder_state = $decoder_state_for->($self);
    my $element_state = $element_state_for->($element);
    $require_owner->($decoder_state, $element_state);
    $expect_tag->($element_state, 'universal', 1, $number);
    $check_child_depth->($element_state->{depth}, $decoder_state->{limits});
    my $cursor = $make_object->('CodingAdventures::DerAsn1::Cursor');
    my $lower = $with_framing->(sub {
        return CodingAdventures::DerTlv::Cursor->new($element_state->{value}, $decoder_state->{limits}->{der});
    });
    $CURSOR_STATE{$cursor} = {lower => $lower, depth => $element_state->{depth} + 1,
        limits => $decoder_state->{limits}, owner => $decoder_state->{owner}};
    return $cursor;
};

package CodingAdventures::DerAsn1::Decoder;
sub new {
    my ($class, $limits) = @_;
    my $decoder = $make_object->($class);
    my $owner = $make_object->('CodingAdventures::DerAsn1::_Owner');
    $DECODER_STATE{$decoder} = {limits => $normalize_limits->($limits), elements_read => 0, owner => $owner};
    return $decoder;
}
sub limits { return $copy_limits->($decoder_state_for->($_[0])->{limits}); }
sub elements_read { return $decoder_state_for->($_[0])->{elements_read}; }
sub decode_exact {
    my ($self, $input) = @_;
    die "input must be a byte string" if !defined($input) || ref($input);
    my $state = $decoder_state_for->($self);
    $fail->('depth-limit-exceeded', 0) if $state->{limits}->{max_depth} == 0;
    $fail->('element-limit-exceeded', 0) if $state->{elements_read} >= $state->{limits}->{max_total_elements};
    my $snapshot = '' . $input;
    my $framed = $with_framing->(sub {
        return CodingAdventures::DerTlv::decode_exact($snapshot, $state->{limits}->{der});
    });
    my $element = $wrap_element->($framed, 0, $state->{owner});
    ++$state->{elements_read};
    return $element;
}
sub sequence { return $container->($_[0], $_[1], 16); }
sub set { return $container->($_[0], $_[1], 17); }
sub explicit {
    my ($self, $element, $tag_number) = @_;
    my $decoder_state = $decoder_state_for->($self);
    my $element_state = $element_state_for->($element);
    $require_owner->($decoder_state, $element_state);
    $expect_tag->($element_state, 'context-specific', 1, $tag_number);
    $check_child_depth->($element_state->{depth}, $decoder_state->{limits});
    $fail->('element-limit-exceeded', length($element_state->{header}))
        if $decoder_state->{elements_read} >= $decoder_state->{limits}->{max_total_elements};
    my $framed = $with_framing->(sub {
        return CodingAdventures::DerTlv::decode_exact($element_state->{value}, $decoder_state->{limits}->{der});
    });
    my $child = $wrap_element->($framed, $element_state->{depth} + 1, $decoder_state->{owner});
    ++$decoder_state->{elements_read};
    return $child;
}

package CodingAdventures::DerAsn1::Cursor;
sub remaining { return '' . $cursor_state_for->($_[0])->{lower}->remaining; }
sub read {
    my ($self, $decoder) = @_;
    my $state = $cursor_state_for->($self);
    return if length($state->{lower}->remaining) == 0;
    my $decoder_state = $decoder_state_for->($decoder);
    $fail->('decoder-limit-mismatch', 0)
        if Scalar::Util::refaddr($state->{owner}) != Scalar::Util::refaddr($decoder_state->{owner});
    $fail->('element-limit-exceeded', 0)
        if $decoder_state->{elements_read} >= $state->{limits}->{max_total_elements};
    my $framed = $with_framing->(sub { return $state->{lower}->read; });
    return if !defined $framed;
    my $element = $wrap_element->($framed, $state->{depth}, $state->{owner});
    ++$decoder_state->{elements_read};
    return $element;
}
sub finish {
    my ($self) = @_;
    $with_framing->(sub { $cursor_state_for->($self)->{lower}->finish; return; });
    return;
}

package CodingAdventures::DerAsn1::Element;
sub tag { my $tag = $element_state_for->($_[0])->{tag}; return {%$tag}; }
sub header { return '' . $element_state_for->($_[0])->{header}; }
sub value { return '' . $element_state_for->($_[0])->{value}; }
sub encoded { return '' . $element_state_for->($_[0])->{encoded}; }
sub depth { return $element_state_for->($_[0])->{depth}; }
sub value_offset { return length($element_state_for->($_[0])->{header}); }

package CodingAdventures::DerAsn1::DerInteger;
sub signed_bytes { return '' . $integer_state_for->($_[0])->{bytes}; }
sub is_negative { return (ord(substr($integer_state_for->($_[0])->{bytes}, 0, 1)) & 0x80) ? 1 : 0; }
sub negative { return $_[0]->is_negative; }
sub to_u64 {
    my ($self) = @_;
    my $state = $integer_state_for->($self);
    $fail->('negative-integer', $state->{offset}) if $self->is_negative;
    my $bytes = $state->{bytes};
    $bytes = substr($bytes, 1) if length($bytes) > 1 && ord(substr($bytes, 0, 1)) == 0;
    $fail->('integer-overflow', $state->{offset}) if length($bytes) > 8;
    my $value = Math::BigInt->new(0);
    $value->bmul(256)->badd(ord($_)) for split //, $bytes;
    return $value;
}

package CodingAdventures::DerAsn1::DerBitString;
sub bytes { return '' . $bit_string_state_for->($_[0])->{bytes}; }
sub unused_bits { return $bit_string_state_for->($_[0])->{unused_bits}; }
sub bit_length { return $bit_string_state_for->($_[0])->{bit_length}; }

package CodingAdventures::DerAsn1::ObjectIdentifier;
sub encoded { return '' . $oid_state_for->($_[0])->{encoded}; }
sub arcs { return [map { Math::BigInt->new($_) } @{ $oid_state_for->($_[0])->{arcs} }]; }
sub arc_count { return scalar @{ $oid_state_for->($_[0])->{arcs} }; }
sub equals {
    my ($self, $expected) = @_;
    return 0 if ref($expected) ne 'ARRAY' || @$expected != $self->arc_count;
    my $actual = $oid_state_for->($self)->{arcs};
    for my $index (0 .. $#$expected) {
        my $text = "$expected->[$index]";
        return 0 if $text !~ /\A\d+\z/ || Math::BigInt->new($text)->bstr ne $actual->[$index];
    }
    return 1;
}

package CodingAdventures::DerAsn1::Error;
use overload q{""} => 'message', fallback => 1;
sub kind { return $error_state_for->($_[0])->{kind}; }
sub offset { return $error_state_for->($_[0])->{offset}; }
sub framing_kind { return $error_state_for->($_[0])->{framing_kind}; }
sub message {
    my $state = $error_state_for->($_[0]);
    my $suffix = defined($state->{framing_kind}) ? " ($state->{framing_kind})" : '';
    return "DER ASN.1 error $state->{kind}$suffix at byte $state->{offset}";
}

1;

__END__

=head1 NAME

CodingAdventures::DerAsn1 - bounded typed ASN.1 DER decoding

=head1 SYNOPSIS

  use CodingAdventures::DerAsn1 qw(decode_object_identifier);
  my $decoder = CodingAdventures::DerAsn1::Decoder->new;
  my $element = $decoder->decode_exact($der_bytes);
  my $oid = decode_object_identifier($element);

=head1 DESCRIPTION

This module provides canonical, bounded, payload-blind typed decoding above
CodingAdventures::DerTlv. Decoder-owned elements and cursors share exact work
budgets and cannot be forged through public constructors.

=head1 LICENSE

MIT

=cut
