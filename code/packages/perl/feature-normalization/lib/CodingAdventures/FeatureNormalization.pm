package CodingAdventures::FeatureNormalization;

use strict;
use warnings;
use Exporter 'import';
use POSIX qw();

our $VERSION = '0.01';
our @EXPORT_OK = qw(
    fit_standard_scaler transform_standard
    fit_min_max_scaler transform_min_max
);

sub _validate_matrix {
    my ($rows) = @_;
    die "matrix must have at least one row and one column\n"
        unless @$rows && @{$rows->[0]};

    my $width = scalar @{$rows->[0]};
    for my $row (@$rows) {
        die "all rows must have the same number of columns\n"
            unless scalar(@$row) == $width;
    }
    return $width;
}

sub fit_standard_scaler {
    my ($rows) = @_;
    my $width = _validate_matrix($rows);
    my $count = scalar @$rows;

    my @means = (0.0) x $width;
    for my $row (@$rows) {
        for my $col (0 .. $width - 1) {
            $means[$col] += $row->[$col];
        }
    }
    $_ /= $count for @means;

    my @stds = (0.0) x $width;
    for my $row (@$rows) {
        for my $col (0 .. $width - 1) {
            my $diff = $row->[$col] - $means[$col];
            $stds[$col] += $diff * $diff;
        }
    }
    $_ = sqrt($_ / $count) for @stds;

    return { means => \@means, standard_deviations => \@stds };
}

sub transform_standard {
    my ($rows, $scaler) = @_;
    my $width = _validate_matrix($rows);
    die "matrix width must match scaler width\n"
        unless $width == scalar @{$scaler->{means}};

    my @out;
    for my $row (@$rows) {
        my @scaled;
        for my $col (0 .. $width - 1) {
            my $std = $scaler->{standard_deviations}->[$col];
            push @scaled, $std == 0.0 ? 0.0 : ($row->[$col] - $scaler->{means}->[$col]) / $std;
        }
        push @out, \@scaled;
    }
    return \@out;
}

sub fit_min_max_scaler {
    my ($rows) = @_;
    my $width = _validate_matrix($rows);

    my @minimums = @{$rows->[0]};
    my @maximums = @{$rows->[0]};
    for my $row (@$rows[1 .. $#$rows]) {
        for my $col (0 .. $width - 1) {
            $minimums[$col] = $row->[$col] if $row->[$col] < $minimums[$col];
            $maximums[$col] = $row->[$col] if $row->[$col] > $maximums[$col];
        }
    }

    return { minimums => \@minimums, maximums => \@maximums };
}

sub transform_min_max {
    my ($rows, $scaler) = @_;
    my $width = _validate_matrix($rows);
    die "matrix width must match scaler width\n"
        unless $width == scalar @{$scaler->{minimums}};

    my @out;
    for my $row (@$rows) {
        my @scaled;
        for my $col (0 .. $width - 1) {
            my $span = $scaler->{maximums}->[$col] - $scaler->{minimums}->[$col];
            push @scaled, $span == 0.0 ? 0.0 : ($row->[$col] - $scaler->{minimums}->[$col]) / $span;
        }
        push @out, \@scaled;
    }
    return \@out;
}

1;
