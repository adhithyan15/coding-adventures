package CodingAdventures::ImageCodecPNG::Filter;

use strict;
use warnings;
use CodingAdventures::ImageCodecPNG::Constants qw(:all);
use CodingAdventures::ImageCodecPNG::Error;

# Paeth predictor (RFC 2083 Section 6.6).
# Normative tie-breaking order: a, then b, then c.
sub paeth {
    my ($a, $b, $c) = @_;
    my $p = $a + $b - $c;
    my $pa = abs($p - $a);
    my $pb = abs($p - $b);
    my $pc = abs($p - $c);

    if ($pa <= $pb && $pa <= $pc) {
        return $a;
    }
    if ($pb <= $pc) {
        return $b;
    }
    return $c;
}

# Applies a single filter to a raw row against the reconstructed prior row.
sub apply_filter {
    my ($filter_id, $raw, $prior, $bpp, $output) = @_;
    my $stride = scalar(@$raw);

    for (my $i = 0; $i < $stride; $i++) {
        my $left = ($i >= $bpp) ? $raw->[$i - $bpp] : 0;
        my $above = $prior->[$i];
        my $above_left = ($i >= $bpp) ? $prior->[$i - $bpp] : 0;

        my $predicted = 0;
        if ($filter_id == FILTER_SUB()) {
            $predicted = $left;
        }
        if ($filter_id == FILTER_UP()) {
            $predicted = $above;
        }
        if ($filter_id == FILTER_AVERAGE()) {
            $predicted = int(($left + $above) / 2);
        }
        if ($filter_id == FILTER_PAETH()) {
            $predicted = paeth($left, $above, $above_left);
        }

        $output->[$i] = ($raw->[$i] - $predicted) & BYTE_MASK();
    }
}

# Chooses the best filter for a row using the sum of signed differences heuristic.
# Lowest sum wins; ties break towards the lowest numbered filter type.
sub choose_filter {
    my ($raw, $prior, $bpp, $scratch, $best) = @_;
    my $best_filter = FILTER_NONE();
    my $best_score;

    for (my $filter_id = 0; $filter_id < FILTER_COUNT(); $filter_id++) {
        apply_filter($filter_id, $raw, $prior, $bpp, $scratch);
        my $score = 0;
        for my $val (@$scratch) {
            $score += ($val < BYTE_MIDPOINT()) ? $val : (BYTE_MODULUS() - $val);
        }
        if (!defined($best_score) || $score < $best_score) {
            $best_score = $score;
            $best_filter = $filter_id;
            @$best = @$scratch;
        }
    }

    return $best_filter;
}

# Inverses a scanline filter in place using already-reconstructed samples.
sub undo_filter {
    my ($filter_id, $row, $prior, $bpp) = @_;
    my $len = scalar(@$row);

    if ($filter_id == FILTER_NONE()) {
        return;
    }
    if ($filter_id == FILTER_SUB()) {
        for (my $i = $bpp; $i < $len; $i++) {
            $row->[$i] = ($row->[$i] + $row->[$i - $bpp]) & BYTE_MASK();
        }
        return;
    }
    if ($filter_id == FILTER_UP()) {
        for (my $i = 0; $i < $len; $i++) {
            $row->[$i] = ($row->[$i] + $prior->[$i]) & BYTE_MASK();
        }
        return;
    }
    if ($filter_id == FILTER_AVERAGE()) {
        for (my $i = 0; $i < $len; $i++) {
            my $left = ($i >= $bpp) ? $row->[$i - $bpp] : 0;
            $row->[$i] = ($row->[$i] + int(($left + $prior->[$i]) / 2)) & BYTE_MASK();
        }
        return;
    }
    if ($filter_id == FILTER_PAETH()) {
        for (my $i = 0; $i < $len; $i++) {
            my $left = ($i >= $bpp) ? $row->[$i - $bpp] : 0;
            my $above = $prior->[$i];
            my $above_left = ($i >= $bpp) ? $prior->[$i - $bpp] : 0;
            $row->[$i] = ($row->[$i] + paeth($left, $above, $above_left)) & BYTE_MASK();
        }
        return;
    }

    CodingAdventures::ImageCodecPNG::Error->throw(ERR_INVALID_FILTER());
}

1;
