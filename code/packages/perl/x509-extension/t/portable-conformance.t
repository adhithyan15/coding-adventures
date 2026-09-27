use strict;
use warnings;
use bytes;
use FindBin qw($Bin);
use File::Spec;
use JSON::PP;
use Scalar::Util qw(blessed);
use Test::More;

use CodingAdventures::DerTlv ();
use CodingAdventures::DerAsn1 ();
use CodingAdventures::X509Extension qw(decode_x509_extension);

my $json = JSON::PP->new->utf8->canonical;
my $root = File::Spec->catdir($Bin, qw(.. .. .. .. specs fixtures));
my $fixture_path = File::Spec->catfile($root, 'x509-extension-v1', 'cases.json');
my $upstream_path = File::Spec->catfile($root, 'der-asn1-v1', 'cases.json');
plan skip_all => 'author conformance fixtures are unavailable outside the monorepo'
    if !-f $fixture_path || !-f $upstream_path;

sub load_json {
    my ($path) = @_;
    open my $handle, '<:raw', $path or die "cannot read $path: $!";
    local $/;
    return $json->decode(<$handle>);
}

my $fixture = load_json($fixture_path);
my $upstream = load_json($upstream_path);

sub materialize {
    my ($segments) = @_;
    return join '', map {
        exists($_->{hex}) ? pack('H*', $_->{hex}) : pack('H*', $_->{repeat_hex}) x $_->{count}
    } @$segments;
}

sub configured_limits {
    my ($case) = @_;
    my %limits = (%{ $upstream->{defaults} }, %{ $case->{limits} // {} });
    $limits{der} = {%{ $upstream->{defaults}->{der} }, %{ $case->{limits}->{der} // {} }};
    $limits{der}->{max_value_len} = CodingAdventures::DerTlv::HOST_MAX()
        if $limits{der}->{max_value_len} eq 'host-max';
    return \%limits;
}

sub attempt {
    my ($decoder, $root_element) = @_;
    my $actual;
    my ($ok, $caught);
    {
        local $@;
        $ok = eval {
            my $value = decode_x509_extension($decoder, $root_element);
            $actual = {outcome => 'value',
                extension_id_arcs_decimal => [map { "$_" } @{ $value->extension_id->arcs }],
                critical => $value->critical ? JSON::PP::true : JSON::PP::false,
                extension_value_hex => unpack('H*', $value->extension_value),
                elements_read => $decoder->elements_read};
            1;
        };
        $caught = $@;
    }
    if (!$ok) {
        die $caught if !blessed($caught) || !$caught->isa('CodingAdventures::X509Extension::Error');
        $actual = {outcome => 'error', error_id => $caught->kind, offset => $caught->offset,
            offset_scope => 'extension-element', elements_read => $decoder->elements_read};
        $actual->{asn1_error_id} = $caught->asn1_kind if defined $caught->asn1_kind;
        $actual->{framing_error_id} = $caught->framing_kind if defined $caught->framing_kind;
    }
    return $actual;
}

sub run_case {
    my ($case) = @_;
    my $decoder = CodingAdventures::DerAsn1::Decoder->new(configured_limits($case));
    my $root_element = $decoder->decode_exact(materialize($case->{input}));
    return attempt($decoder, $root_element) if $case->{operation} ne 'extension-script';
    return {outcome => 'script',
        events => [map { attempt($decoder, $root_element) } @{ $case->{actions} }]};
}

is(scalar @{ $fixture->{cases} }, 48, 'closed fixture has 48 cases');
is(scalar @{ $fixture->{error_ids} }, 8, 'closed fixture has eight errors');
for my $case (@{ $fixture->{cases} }) {
    my $actual = run_case($case);
    is($json->encode($actual), $json->encode($case->{expected}), $case->{id});
    if (exists $case->{redacted_input_hex}) {
        unlike($json->encode($actual), qr/\Q$case->{redacted_input_hex}\E/i, "$case->{id} projection redacts input");
        my $decoder = CodingAdventures::DerAsn1::Decoder->new(configured_limits($case));
        my $root_element = $decoder->decode_exact(materialize($case->{input}));
        my $error;
        { local $@; eval { decode_x509_extension($decoder, $root_element); 1 } or $error = $@; }
        unlike("$error", qr/\Q$case->{redacted_input_hex}\E/i, "$case->{id} error text redacts input");
    }
}

done_testing;
