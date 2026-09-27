package CodingAdventures::X509Extension;

use strict;
use warnings;
use bytes;
use Exporter qw(import);
use Hash::Util::FieldHash qw(fieldhash);
use Scalar::Util qw(blessed);
use CodingAdventures::DerAsn1 qw(decode_boolean decode_object_identifier decode_octet_string);

our $VERSION = '0.1.0';
our @EXPORT_OK = qw(decode_x509_extension);

fieldhash my %VALUE_STATE;
fieldhash my %ERROR_STATE;

my $make_object = sub {
    my ($class) = @_;
    my $value = 0;
    return bless \$value, $class;
};

my $value_state_for = sub {
    my ($value) = @_;
    die 'invalid X.509 extension value'
        if !blessed($value) || !$value->isa('CodingAdventures::X509Extension::Value')
            || !exists $VALUE_STATE{$value};
    return $VALUE_STATE{$value};
};

my $error_state_for = sub {
    my ($error) = @_;
    die 'invalid X.509 extension error'
        if !blessed($error) || !$error->isa('CodingAdventures::X509Extension::Error')
            || !exists $ERROR_STATE{$error};
    return $ERROR_STATE{$error};
};

my $make_error = sub {
    my ($kind, $offset, $asn1_kind, $framing_kind) = @_;
    my $error = $make_object->('CodingAdventures::X509Extension::Error');
    $ERROR_STATE{$error} = {kind => $kind, offset => $offset,
        asn1_kind => $asn1_kind, framing_kind => $framing_kind};
    return $error;
};

my $fail = sub { die $make_error->(@_); };

my $asn1_error = sub {
    my ($error) = @_;
    die $error if !blessed($error) || !$error->isa('CodingAdventures::DerAsn1::Error');
    return $error;
};

my $capture = sub {
    my ($code) = @_;
    my ($ok, $value, $caught);
    { local $@; $ok = eval { $value = $code->(); 1 }; $caught = $@; }
    return (1, $value) if $ok;
    return (0, $asn1_error->($caught));
};

my $child_offset = sub {
    my ($element, $cursor) = @_;
    return $element->value_offset + length($element->value) - length($cursor->remaining);
};

my $read_child = sub {
    my ($decoder, $root, $cursor) = @_;
    my $start = $child_offset->($root, $cursor);
    my ($ok, $result) = $capture->(sub { return $cursor->read($decoder); });
    if (!$ok) {
        my $offset = $result->kind eq 'framing'
            ? $root->value_offset + $result->offset
            : $start + $result->offset;
        $fail->('structure', $offset, $result->kind, $result->framing_kind);
    }
    return $result;
};

my $decode_typed = sub {
    my ($kind, $offset, $code) = @_;
    my ($ok, $result) = $capture->($code);
    $fail->($kind, $offset + $result->offset, $result->kind, $result->framing_kind) if !$ok;
    return $result;
};

sub decode_x509_extension {
    my ($decoder, $root) = @_;
    my ($opened, $cursor) = $capture->(sub { return $decoder->sequence($root); });
    $fail->('structure', $cursor->offset, $cursor->kind, $cursor->framing_kind) if !$opened;

    my $extension_id_offset = $child_offset->($root, $cursor);
    my $extension_id_element = $read_child->($decoder, $root, $cursor);
    $fail->('missing-extension-id', $extension_id_offset) if !defined $extension_id_element;
    my $extension_id = $decode_typed->('invalid-extension-id', $extension_id_offset,
        sub { return decode_object_identifier($extension_id_element, $decoder->limits); });

    my $second_offset = $child_offset->($root, $cursor);
    my $second = $read_child->($decoder, $root, $cursor);
    $fail->('missing-extension-value', $second_offset) if !defined $second;

    my ($critical, $value_element, $value_offset) = (0, $second, $second_offset);
    if ($second->tag->{number} == 1) {
        $critical = $decode_typed->('invalid-critical', $second_offset,
            sub { return decode_boolean($second); });
        $fail->('encoded-default-critical', $second_offset) if !$critical;
        $value_offset = $child_offset->($root, $cursor);
        $value_element = $read_child->($decoder, $root, $cursor);
        $fail->('missing-extension-value', $value_offset) if !defined $value_element;
    }

    my $extension_value = $decode_typed->('invalid-extension-value', $value_offset,
        sub { return decode_octet_string($value_element); });
    my $trailing_offset = $child_offset->($root, $cursor);
    my $trailing = $read_child->($decoder, $root, $cursor);
    $fail->('trailing-element', $trailing_offset) if defined $trailing;

    my $value = $make_object->('CodingAdventures::X509Extension::Value');
    $VALUE_STATE{$value} = {extension_id => $extension_id, critical => $critical ? 1 : 0,
        extension_value => '' . $extension_value};
    return $value;
}

package CodingAdventures::X509Extension::Value;

sub extension_id { return $value_state_for->($_[0])->{extension_id}; }
sub critical { return $value_state_for->($_[0])->{critical}; }
sub extension_value { return '' . $value_state_for->($_[0])->{extension_value}; }

package CodingAdventures::X509Extension::Error;

use overload '""' => sub {
    my $state = $error_state_for->($_[0]);
    return "X.509 extension error $state->{kind} at byte $state->{offset}";
}, fallback => 1;

sub kind { return $error_state_for->($_[0])->{kind}; }
sub offset { return $error_state_for->($_[0])->{offset}; }
sub asn1_kind { return $error_state_for->($_[0])->{asn1_kind}; }
sub framing_kind { return $error_state_for->($_[0])->{framing_kind}; }

1;

__END__

=head1 NAME

CodingAdventures::X509Extension - bounded generic X.509 Extension decoding

=head1 SYNOPSIS

  use CodingAdventures::X509Extension qw(decode_x509_extension);
  my $extension = decode_x509_extension($decoder, $root);

=head1 DESCRIPTION

Validates the generic RFC 5280 Extension sequence with shared caller limits,
opaque extension bytes, private immutable values, and payload-redacted errors.

=cut
