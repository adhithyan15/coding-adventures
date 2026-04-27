package GradientDescent;
use strict;
use warnings;
use Exporter 'import';
our @EXPORT_OK = qw(sgd);

sub sgd {
    my ($weights, $gradients, $learning_rate) = @_;
    die "Arrays must have the same non-zero length" if @$weights != @$gradients || @$weights == 0;
    
    my @res;
    for my $i (0 .. $#$weights) {
        push @res, $weights->[$i] - ($learning_rate * $gradients->[$i]);
    }
    return \@res;
}

1;
