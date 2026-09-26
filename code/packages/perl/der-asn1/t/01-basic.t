use strict;
use warnings;
use bytes;
use Test::More;

use CodingAdventures::DerTlv ();
use CodingAdventures::DerAsn1 qw(default_limits decode_boolean decode_integer
    decode_bit_string decode_octet_string decode_implicit_octet_string
    decode_implicit_ia5_string decode_object_identifier
    decode_implicit_object_identifier decode_null);

sub bytes { return pack('H*', $_[0]); }
## no critic (ProhibitSubroutinePrototypes) -- block syntax keeps failure probes readable.
sub caught (&) {
    my ($code) = @_;
    local $@;
    eval { $code->(); 1 } and return;
    return $@;
}

my $defaults = default_limits();
is($defaults->{max_depth}, 32, 'default depth');
is($defaults->{max_total_elements}, 16_384, 'default total elements');
$defaults->{der}->{max_input_len} = 1;
is(default_limits()->{der}->{max_input_len}, 1_048_576, 'default limits are defensive snapshots');

my $decoder = CodingAdventures::DerAsn1::Decoder->new;
my $element = $decoder->decode_exact(bytes('04012a'));
is($decoder->elements_read, 1, 'root decode increments shared budget');
is(unpack('H*', decode_octet_string($element)), '2a', 'octet string decodes');
is(unpack('H*', $element->header), '0401', 'header accessor');
is(unpack('H*', $element->encoded), '04012a', 'encoded accessor');
is($element->depth, 0, 'root depth');
my $tag = $element->tag;
$tag->{number} = 99;
is($element->tag->{number}, 4, 'tag accessor is defensive');

my $forged_value = 0;
my $forged = bless \$forged_value, 'CodingAdventures::DerAsn1::Element';
like(caught { decode_octet_string($forged) }, qr/validated Element/, 'forged element rejected');
for my $helper (qw(_object _wrap_element _decode_oid _require_decoder _require_element
        _require_owner _expect_tag _check_child_depth _with_framing _fail _normalize_limits _container)) {
    ok(!CodingAdventures::DerAsn1->can($helper), "$helper is not package-public");
}

my $owner_decoder = CodingAdventures::DerAsn1::Decoder->new;
my $root = $owner_decoder->decode_exact(bytes('300430020500'));
my $outer = $owner_decoder->sequence($root);
my $nested = $outer->read($owner_decoder);
my $inner = $owner_decoder->sequence($nested);
my $grandchild = $inner->read($owner_decoder);
is($grandchild->depth, 2, 'nested cursor preserves depth and owner');
my $foreign = CodingAdventures::DerAsn1::Decoder->new;
my $owner_error = caught { $foreign->sequence($nested) };
isa_ok($owner_error, 'CodingAdventures::DerAsn1::Error');
is($owner_error->kind, 'decoder-limit-mismatch', 'foreign decoder cannot consume element');

my $cursor_decoder = CodingAdventures::DerAsn1::Decoder->new;
my $cursor_root = $cursor_decoder->decode_exact(bytes('30020500'));
my $cursor = $cursor_decoder->sequence($cursor_root);
my $cursor_error = caught { $cursor->read($foreign) };
is($cursor_error->kind, 'decoder-limit-mismatch', 'foreign cursor read rejected');
ok(defined $cursor->read($cursor_decoder), 'creating decoder reads child');
is($cursor->read($foreign), undef, 'end precedence is unconditional');

my $transactional = CodingAdventures::DerAsn1::Decoder->new({
    der => {%{ CodingAdventures::DerTlv::default_limits() }, max_elements => 1}
});
my $transactional_root = $transactional->decode_exact(bytes('300405000500'));
my $transactional_cursor = $transactional->sequence($transactional_root);
ok(defined $transactional_cursor->read($transactional), 'first lower element succeeds');
my $before = $transactional_cursor->remaining;
my $framing_error = caught { $transactional_cursor->read($transactional) };
is($framing_error->kind, 'framing', 'lower budget error translated');
is($framing_error->framing_kind, 'element-limit-exceeded', 'lower error identifier retained');
is($transactional_cursor->remaining, $before, 'failed lower read does not advance');

my $explicit_decoder = CodingAdventures::DerAsn1::Decoder->new({max_total_elements => 1});
my $explicit_root = $explicit_decoder->decode_exact(bytes('a3020500'));
my $explicit_error = caught { $explicit_decoder->explicit($explicit_root, 3) };
is($explicit_error->kind, 'element-limit-exceeded', 'explicit child shares root budget');

my $integer = decode_integer(CodingAdventures::DerAsn1::Decoder->new->decode_exact(bytes('020900ffffffffffffffff')));
is($integer->to_u64->bstr, '18446744073709551615', 'u64 maximum is exact');
ok(!$integer->is_negative, 'positive integer reports sign');
my $negative = decode_integer(CodingAdventures::DerAsn1::Decoder->new->decode_exact(bytes('0201ff')));
ok($negative->is_negative, 'negative integer reports sign');
is(caught { $negative->to_u64 }->kind, 'negative-integer', 'negative integer cannot convert to u64');

my $bits = decode_bit_string(CodingAdventures::DerAsn1::Decoder->new->decode_exact(bytes('030203a8')));
is(unpack('H*', $bits->bytes), 'a8', 'bit payload');
is($bits->unused_bits, 3, 'unused bit count');
is($bits->bit_length, 5, 'bit length');

my $oid = decode_object_identifier(CodingAdventures::DerAsn1::Decoder->new->decode_exact(bytes('06032a0304')));
is_deeply([map { $_->bstr } @{ $oid->arcs }], [qw(1 2 3 4)], 'OID arcs are exact');
ok($oid->equals([1, 2, 3, 4]), 'OID equality accepts exact arcs');
ok(!$oid->equals([1, 2, 3, 5]), 'OID equality rejects different arcs');
my $arcs = $oid->arcs;
$arcs->[0]->badd(9);
is($oid->arcs->[0]->bstr, '1', 'OID arc snapshots are defensive');

like(caught { CodingAdventures::DerAsn1::Decoder->new({max_depth => -1}) }, qr/non-negative integer/,
    'negative limits rejected');
like(caught { CodingAdventures::DerAsn1::Decoder->new({unknown => 1}) }, qr/unknown limit/, 'unknown limits rejected');
like(caught { CodingAdventures::DerAsn1::Decoder->new({der => {unknown => 1}}) }, qr/unknown DER limit/,
    'unknown lower limits rejected');
like(caught { CodingAdventures::DerAsn1::Decoder->new({max_depth => '9' x 1_000}) },
    qr/exceeds supported host maximum/, 'huge limits cannot become infinity');
like(caught { CodingAdventures::DerAsn1::Decoder->new->decode_exact([]) }, qr/byte string/, 'non-byte input rejected');

my $implicit_element = CodingAdventures::DerAsn1::Decoder->new->decode_exact(bytes('8000'));
my $tag_decoder = CodingAdventures::DerAsn1::Decoder->new;
my $explicit_element = $tag_decoder->decode_exact(bytes('a0020500'));
for my $invalid_tag (undef, 'not-a-number', -1, '1.5', [], '4294967296') {
    like(caught { decode_implicit_octet_string($implicit_element, $invalid_tag) }, qr/tag_number must be a u32/,
        'implicit OCTET STRING rejects invalid tag number');
    like(caught { decode_implicit_ia5_string($implicit_element, $invalid_tag) }, qr/tag_number must be a u32/,
        'implicit IA5String rejects invalid tag number');
    like(caught { decode_implicit_object_identifier($implicit_element, $invalid_tag) }, qr/tag_number must be a u32/,
        'implicit OID rejects invalid tag number');
    like(caught { $tag_decoder->explicit($explicit_element, $invalid_tag) },
        qr/tag_number must be a u32/, 'EXPLICIT rejects invalid tag number');
}

my $depth_first = caught {
    CodingAdventures::DerAsn1::Decoder->new({max_depth => 0,
        der => {%{ CodingAdventures::DerTlv::default_limits() }, max_input_len => 1}})->decode_exact(bytes('0500'));
};
is($depth_first->kind, 'depth-limit-exceeded', 'depth failure precedes framing input limit');
my $count_first = caught {
    CodingAdventures::DerAsn1::Decoder->new({max_total_elements => 0,
        der => {%{ CodingAdventures::DerTlv::default_limits() }, max_input_len => 1}})->decode_exact(bytes('0500'));
};
is($count_first->kind, 'element-limit-exceeded', 'count failure precedes framing input limit');

my $null = CodingAdventures::DerAsn1::Decoder->new->decode_exact(bytes('0500'));
is(decode_null($null), undef, 'NULL decodes to undef');
is(decode_boolean(CodingAdventures::DerAsn1::Decoder->new->decode_exact(bytes('0101ff'))), 1, 'BOOLEAN true');

my $message = "$framing_error";
like($message, qr/^DER ASN\.1 error framing \(element-limit-exceeded\) at byte 2/, 'stable diagnostic');
unlike($message, qr/secret|300405000500/, 'diagnostic is payload blind');

done_testing;
