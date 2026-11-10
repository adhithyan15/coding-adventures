use strict;
use warnings;
use Test2::V0;
no warnings q{portable};
use FindBin;
use JSON::PP ();

use CodingAdventures::CtCompare qw(
    ct_eq ct_eq_fixed ct_select_bytes ct_eq_u64
    ct_eq_bit ct_eq_fixed_bit ct_eq_u64_bit
);

# Replays every case of the shared CT01 corpus
# (code/specs/fixtures/ct-compare-v1/cases.json), located relative to this
# test file: t/ -> ct-compare -> perl -> packages -> code.
my $path = "$FindBin::Bin/../../../../specs/fixtures/ct-compare-v1/cases.json";
open my $fh, '<:raw', $path or die "cannot open $path: $!";
my $text = do { local $/; <$fh> };
close $fh;
my $doc = JSON::PP->new->decode($text);

is($doc->{profile}, 'ct-compare-v1', 'ct-compare-v1 profile');
is(scalar @{ $doc->{cases} }, 21, '21 corpus cases');

sub from_hex { pack 'H*', $_[0] }
sub to_hex   { unpack 'H*', $_[0] }

for my $case (@{ $doc->{cases} }) {
    my $id = $case->{id};
    my $op = $case->{operation};

    if ($op eq 'eq' || $op eq 'eq_fixed') {
        my $l = from_hex($case->{left_hex});
        my $r = from_hex($case->{right_hex});
        my $want = $case->{expected}{equal} ? 1 : 0;    # corpus data is public
        my ($got, $bit);
        if ($op eq 'eq') {
            ($got, $bit) = (ct_eq($l, $r), ct_eq_bit($l, $r));
        }
        else {
            is(length $l, $case->{fixed_size_bytes}, "$id: fixed width");
            ($got, $bit) = (ct_eq_fixed($l, $r), ct_eq_fixed_bit($l, $r));
        }
        is($got ? 1 : 0, $want, "$id: boolean");
        is($bit, $want, "$id: bit");
    }
    elsif ($op eq 'select_bytes') {
        my $l = from_hex($case->{left_hex});
        my $r = from_hex($case->{right_hex});
        my $choice = $case->{choice} ? 1 : 0;
        if (exists $case->{expected}{error_id}) {
            is($case->{expected}{error_id}, 'CT_SELECT_LENGTH_MISMATCH', "$id: id");
            like(dies { ct_select_bytes($l, $r, $choice) },
                qr/CT_SELECT_LENGTH_MISMATCH/, "$id: error");
        }
        else {
            my $out = ct_select_bytes($l, $r, $choice);
            is(to_hex($out), $case->{expected}{output_hex}, "$id: output");
            is(to_hex($l), $case->{left_hex},  "$id: left unchanged");
            is(to_hex($r), $case->{right_hex}, "$id: right unchanged");
            # Fresh storage: mutating the result must not touch the inputs.
            if (length $out) {
                substr($out, 0, 1) = "\x5A";
                is(to_hex($l), $case->{left_hex},  "$id: left still unchanged");
                is(to_hex($r), $case->{right_hex}, "$id: right still unchanged");
            }
        }
    }
    elsif ($op eq 'eq_u64') {
        my $l = hex $case->{left_u64};
        my $r = hex $case->{right_u64};
        my $want = $case->{expected}{equal} ? 1 : 0;
        is(ct_eq_u64($l, $r) ? 1 : 0, $want, "$id: boolean");
        is(ct_eq_u64_bit($l, $r), $want, "$id: bit");
    }
    else {
        fail("unknown corpus operation $op");
    }
}

done_testing;
