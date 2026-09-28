use strict;
use warnings;
use bytes;
use Scalar::Util qw(blessed);
use Test::More;

use CodingAdventures::DerAsn1 ();
use CodingAdventures::X509Extension qw(decode_x509_extension);

sub decode_value {
    my ($hex) = @_;
    my $decoder = CodingAdventures::DerAsn1::Decoder->new;
    my $root = $decoder->decode_exact(pack('H*', $hex));
    return ($decoder, decode_x509_extension($decoder, $root));
}

my ($decoder, $value) = decode_value('30090603551d1104023000');
is_deeply([map { $_->bstr } @{ $value->extension_id->arcs }], [qw(2 5 29 17)], 'OID is decoded');
ok(!$value->critical, 'critical defaults to false');
is(unpack('H*', $value->extension_value), '3000', 'opaque extension bytes are preserved');
is($decoder->elements_read, 3, 'shared decoder budget is charged');

my $copy = $value->extension_value;
substr($copy, 0, 1, "\xff");
is(unpack('H*', $value->extension_value), '3000', 'returned bytes are defensive snapshots');

for my $class (qw(CodingAdventures::X509Extension::Value CodingAdventures::X509Extension::Error)) {
    my $scalar = 0;
    my $forged = bless \$scalar, $class;
    my $method = $class =~ /Value\z/ ? 'critical' : 'kind';
    my $ok = eval { $forged->$method; 1 };
    ok(!$ok, "$class cannot be forged");
}
ok(!CodingAdventures::X509Extension::Value->can('new'), 'Value has no public constructor');
ok(!CodingAdventures::X509Extension::Error->can('new'), 'Error has no public constructor');

my $hostile_decoder = CodingAdventures::DerAsn1::Decoder->new;
my $hostile_root = $hostile_decoder->decode_exact(pack('H*', '30080601800403deadbe'));
my $error;
{ local $@; eval { decode_x509_extension($hostile_decoder, $hostile_root); 1 } or $error = $@; }
ok(blessed($error) && $error->isa('CodingAdventures::X509Extension::Error'), 'typed error is thrown');
is($error->kind, 'invalid-extension-id', 'error kind is stable');
is($error->asn1_kind, 'non-minimal-object-identifier', 'nested error kind is stable');
is($error->offset, 4, 'offset is extension-local');
unlike("$error", qr/deadbe/i, 'stringification is payload blind');

done_testing;
