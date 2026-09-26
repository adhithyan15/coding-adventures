use strict;
use warnings;
use bytes;
use FindBin qw($Bin);
use File::Spec;
use JSON::PP;
use Test::More;

use CodingAdventures::DerTlv ();
use CodingAdventures::DerAsn1 qw(
    decode_boolean decode_integer decode_bit_string decode_octet_string
    decode_implicit_octet_string decode_ia5_string decode_implicit_ia5_string
    decode_null decode_object_identifier decode_implicit_object_identifier
);

my $json = JSON::PP->new->utf8->canonical;
my $root = File::Spec->catdir($Bin, qw(.. .. .. .. specs fixtures));
my $fixture_path = File::Spec->catfile($root, 'der-asn1-v1', 'cases.json');
my $upstream_path = File::Spec->catfile($root, 'der-tlv-v1', 'cases.json');
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
my %upstream_by_id = map { $_->{id} => $_ } @{ $upstream->{cases} };

sub materialize {
    my ($segments) = @_;
    return join '', map {
        exists($_->{hex}) ? pack('H*', $_->{hex}) : pack('H*', $_->{repeat_hex}) x $_->{count}
    } @$segments;
}

sub configured_limits {
    my ($case) = @_;
    my %limits = (%{ $fixture->{defaults} }, %{ $case->{limits} // {} });
    $limits{der} = {%{ $fixture->{defaults}->{der} }, %{ $case->{limits}->{der} // {} }};
    $limits{der}->{max_value_len} = CodingAdventures::DerTlv::HOST_MAX()
        if $limits{der}->{max_value_len} eq 'host-max';
    return \%limits;
}

sub tag_projection {
    my ($element) = @_;
    my $tag = $element->tag;
    return {class => $tag->{class}, constructed => $tag->{constructed} ? JSON::PP::true : JSON::PP::false,
        number => $tag->{number}};
}

sub failure {
    my ($error, $scope) = @_;
    my $result = {outcome => 'error', error_id => $error->kind, offset => $error->offset,
        offset_scope => $scope // 'operation-input'};
    $result->{framing_error_id} = $error->framing_kind if defined $error->framing_kind;
    return $result;
}

sub run_upstream {
    my ($case) = @_;
    my $row = $upstream_by_id{$case->{der_tlv_case_id}};
    my $input = materialize($row->{input});
    my %limits = (%{ $upstream->{defaults} }, %{ $row->{limits} // {} });
    $limits{max_value_len} = CodingAdventures::DerTlv::HOST_MAX() if $limits{max_value_len} eq 'host-max';
    my $decoder = CodingAdventures::DerAsn1::Decoder->new({der => \%limits});
    my $actual;
    eval {
        my $element = $decoder->decode_exact($input);
        my $tag = $element->tag;
        die 'delegated decode did not consume exactly one ASN.1 element'
            if $decoder->elements_read != 1;
        $actual = {outcome => 'element', element_offset => 0,
            tag => {class => $tag->{class}, constructed => $tag->{constructed} ? JSON::PP::true : JSON::PP::false,
                number => $tag->{number}}, header_len => length($element->header),
            encoded_len => length($element->encoded), remainder_offset => length($element->encoded)};
        1;
    } or do {
        my $error = $@;
        die $error if !ref($error) || !$error->isa('CodingAdventures::DerAsn1::Error')
            || $error->kind ne 'framing' || !defined($error->framing_kind);
        $actual = {outcome => 'error', error_id => $error->framing_kind, offset => $error->offset};
    };
    return $json->encode($actual) eq $json->encode($row->{expected}) ? {outcome => 'upstream'} : $actual;
}

sub primitive_result {
    my ($operation, $element, $limits, $tag_number) = @_;
    if ($operation eq 'decode-boolean') {
        return {outcome => 'value', boolean => decode_boolean($element) ? JSON::PP::true : JSON::PP::false};
    }
    if ($operation eq 'decode-integer' || $operation eq 'integer-to-u64') {
        my $integer = decode_integer($element);
        my $result = {outcome => 'value', signed_hex => unpack('H*', $integer->signed_bytes),
            negative => $integer->is_negative ? JSON::PP::true : JSON::PP::false};
        $result->{u64_decimal} = '' . $integer->to_u64->bstr if $operation eq 'integer-to-u64';
        return $result;
    }
    if ($operation eq 'decode-bit-string') {
        my $bits = decode_bit_string($element);
        return {outcome => 'value', bytes_hex => unpack('H*', $bits->bytes), unused_bits => $bits->unused_bits,
            bit_length => $bits->bit_length};
    }
    return {outcome => 'value', bytes_hex => unpack('H*', decode_octet_string($element))}
        if $operation eq 'decode-octet-string';
    return {outcome => 'value', bytes_hex => unpack('H*', decode_implicit_octet_string($element, $tag_number))}
        if $operation eq 'decode-implicit-octet-string';
    return {outcome => 'value', text => decode_ia5_string($element)} if $operation eq 'decode-ia5-string';
    return {outcome => 'value', text => decode_implicit_ia5_string($element, $tag_number)}
        if $operation eq 'decode-implicit-ia5-string';
    if ($operation eq 'decode-null') { decode_null($element); return {outcome => 'value'}; }
    if ($operation eq 'decode-object-identifier' || $operation eq 'decode-implicit-object-identifier') {
        my $oid = $operation eq 'decode-object-identifier'
            ? decode_object_identifier($element, $limits)
            : decode_implicit_object_identifier($element, $tag_number, $limits);
        return {outcome => 'value', bytes_hex => unpack('H*', $oid->encoded),
            arcs_decimal => [map { '' . $_->bstr } @{ $oid->arcs }], arc_count => $oid->arc_count};
    }
    die "unsupported operation $operation";
}

sub cursor_result {
    my ($case, $decoder, $root_element) = @_;
    my $cursor = $decoder->sequence($root_element);
    my $total = length($cursor->remaining);
    my @events;
    for my $action (@{ $case->{actions} }) {
        if ($action eq 'finish') {
            eval { $cursor->finish; push @events, {outcome => 'finished'}; 1 } or do {
                my $error = $@; push @events, failure($error, 'container-value');
            };
            next;
        }
        my $active = $decoder;
        if ($action eq 'read-with-different-limits') {
            my $limits = $decoder->limits;
            ++$limits->{max_total_elements};
            $active = CodingAdventures::DerAsn1::Decoder->new($limits);
        }
        eval {
            my $child = $cursor->read($active);
            if ($action eq 'read-nested-sequence') {
                die 'nested child required' if !defined $child;
                my $nested = $decoder->sequence($child);
                my $grandchild = $nested->read($decoder);
                die 'nested grandchild required' if !defined $grandchild;
                $nested->finish;
                push @events, {outcome => 'value', tag => tag_projection($grandchild), depth => $grandchild->depth};
            } else {
                push @events, defined($child)
                    ? {outcome => 'value', tag => tag_projection($child), depth => $child->depth}
                    : {outcome => 'end'};
            }
            1;
        } or do {
            my $error = $@; push @events, failure($error, 'container-value');
        };
    }
    return {outcome => 'value', elements_read => $decoder->elements_read,
        remaining_offset => $total - length($cursor->remaining), events => \@events};
}

sub run_case {
    my ($case) = @_;
    return run_upstream($case) if exists $case->{der_tlv_case_id};
    my $limits = configured_limits($case);
    my $decoder = CodingAdventures::DerAsn1::Decoder->new($limits);
    my $operation = $case->{operation};
    my $actual;
    eval {
        my $root_element = $decoder->decode_exact(materialize($case->{input}));
        if ($operation eq 'decode-exact') {
            $actual = {outcome => 'value', tag => tag_projection($root_element),
                header_hex => unpack('H*', $root_element->header), value_hex => unpack('H*', $root_element->value),
                encoded_hex => unpack('H*', $root_element->encoded), depth => $root_element->depth,
                elements_read => $decoder->elements_read};
        } elsif ($operation eq 'cursor-script') {
            $actual = cursor_result($case, $decoder, $root_element);
        } elsif ($operation eq 'sequence' || $operation eq 'set') {
            my $cursor = $operation eq 'sequence' ? $decoder->sequence($root_element) : $decoder->set($root_element);
            $actual = {outcome => 'value', elements_read => $decoder->elements_read,
                remaining_offset => length($root_element->value) - length($cursor->remaining)};
        } elsif ($operation eq 'explicit') {
            my $child = $decoder->explicit($root_element, $case->{tag_number});
            $actual = {outcome => 'value', tag => tag_projection($child), value_hex => unpack('H*', $child->value),
                depth => $child->depth, elements_read => $decoder->elements_read};
        } else {
            $actual = primitive_result($operation, $root_element, $limits, $case->{tag_number});
            $actual->{elements_read} = $decoder->elements_read if exists $case->{expected}->{elements_read};
        }
        1;
    } or do {
        my $error = $@;
        die $error if !ref($error) || !$error->isa('CodingAdventures::DerAsn1::Error');
        my $scope = $operation eq 'explicit' && $error->kind eq 'framing' ? 'container-value' : 'operation-input';
        $actual = failure($error, $scope);
    };
    return $actual;
}

is(scalar @{ $fixture->{cases} }, 122, 'closed fixture has 122 cases');
is(scalar @{ $fixture->{error_ids} }, 22, 'closed fixture has 22 errors');
my %references = map { $_->{der_tlv_case_id} => 1 } grep { exists $_->{der_tlv_case_id} } @{ $fixture->{cases} };
is(scalar keys %references, 46, 'all 46 delegated DER TLV rows are referenced');
for my $case (@{ $fixture->{cases} }) {
    my $actual = run_case($case);
    is($json->encode($actual), $json->encode($case->{expected}), $case->{id});
    unlike($json->encode($actual), qr/\Q$case->{redacted_input_hex}\E/, "$case->{id} redacts input")
        if exists $case->{redacted_input_hex};
}

done_testing;
