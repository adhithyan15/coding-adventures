use strict;
use warnings;
use bytes;
use FindBin qw($Bin);
use File::Spec;
use JSON::PP;
use Test::More;

use lib File::Spec->catdir($Bin, '..', 'lib');
use CodingAdventures::DerTlv qw(decode_one decode_exact new_cursor default_limits);

my $fixture_path = File::Spec->catfile(
    $Bin, '..', '..', '..', '..', 'specs', 'fixtures', 'der-tlv-v1', 'cases.json',
);
open my $fixture_handle, '<:encoding(UTF-8)', $fixture_path
    or die "cannot open fixture: $!";
local $/;
my $fixture = JSON::PP->new->decode(<$fixture_handle>);
close $fixture_handle;
my $json = JSON::PP->new->canonical(1);

sub materialize {
    my ($segments) = @_;
    return join '', map {
        exists $_->{hex}
            ? pack('H*', $_->{hex})
            : pack('H*', $_->{repeat_hex}) x $_->{count}
    } @{$segments};
}

sub case_limits {
    my ($test_case) = @_;
    my %values = (%{ $fixture->{defaults} }, %{ $test_case->{limits} // {} });
    $values{max_value_len} = CodingAdventures::DerTlv::HOST_MAX()
        if $values{max_value_len} eq 'host-max';
    return \%values;
}

sub element_projection {
    my ($element, $offset) = @_;
    my $tag = $element->tag;
    return {
        outcome        => 'element',
        element_offset => $offset,
        tag             => {
            class       => $tag->{class},
            constructed => $tag->{constructed} ? JSON::PP::true : JSON::PP::false,
            number      => $tag->{number},
        },
        header_len       => length($element->header),
        encoded_len      => length($element->encoded),
        remainder_offset => $offset + length($element->encoded),
    };
}

sub error_projection {
    my ($error) = @_;
    return {outcome => 'error', error_id => $error->kind, offset => $error->offset};
}

sub capture_error {
    my ($operation) = @_;
    my $result = eval { $operation->(); };
    return ($result, $@);
}

sub run_decode {
    my ($test_case, $input, $limits) = @_;
    my ($result, $error) = capture_error(sub {
        if ($test_case->{operation} eq 'decode-one') {
            my ($element, $remainder) = decode_one($input, $limits);
            my $projection = element_projection($element, 0);
            is(
                $projection->{remainder_offset},
                length($input) - length($remainder),
                "$test_case->{id} remainder",
            );
            return $projection;
        }
        return element_projection(decode_exact($input, $limits), 0);
    });
    return ref($error) ? error_projection($error) : $result;
}

sub run_cursor {
    my ($test_case, $input, $limits) = @_;
    my $cursor = new_cursor($input, $limits);
    my @events;
    for my $action (@{ $test_case->{actions} }) {
        if ($action eq 'finish') {
            my (undef, $error) = capture_error(sub { $cursor->finish; return 1; });
            push @events, ref($error) ? error_projection($error) : {outcome => 'finished'};
            next;
        }
        my $offset = length($input) - length($cursor->remaining);
        my ($element, $error) = capture_error(sub { return $cursor->read; });
        push @events,
            ref($error) ? error_projection($error)
            : defined($element) ? element_projection($element, $offset)
            : {outcome => 'end'};
    }
    return {
        events           => \@events,
        elements_read    => $cursor->elements_read,
        remaining_offset => length($input) - length($cursor->remaining),
    };
}

is(scalar @{ $fixture->{cases} }, 54, 'closed fixture contains 54 cases');
for my $test_case (@{ $fixture->{cases} }) {
    my $input  = materialize($test_case->{input});
    my $limits = case_limits($test_case);
    my $actual = $test_case->{operation} eq 'cursor'
        ? run_cursor($test_case, $input, $limits)
        : run_decode($test_case, $input, $limits);
    is($json->encode($actual), $json->encode($test_case->{expected}), $test_case->{id});
    unlike($json->encode($actual), qr/\Q$test_case->{redacted_input_hex}\E/,
        "$test_case->{id} is payload blind") if $test_case->{redacted_input_hex};
}

is(default_limits()->{max_elements}, 4_096, 'default limits are public');
is(decode_exact("\x04\x01\x2a")->value, "\x2a", 'value slice is exposed');
my $error = CodingAdventures::DerTlv::Error->new('truncated-value', 2);
is("$error", 'DER framing error truncated-value at byte 2', 'stable error message');

for my $invalid (
    {max_elements => -1},
    {max_tag_number => 4_294_967_296},
) {
    my (undef, $failure) = capture_error(sub { decode_exact('', $invalid); });
    like("$failure", qr/(?:non-negative integer|fit u32)/, 'invalid limits rejected');
}

done_testing;
