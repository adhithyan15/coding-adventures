package ActivationFunctions;
use strict;
use warnings;
use Exporter 'import';
our @EXPORT_OK = qw(sigmoid sigmoid_derivative relu relu_derivative tanh tanh_derivative);

sub sigmoid {
    my $x = shift;
    return 0.0 if $x < -709;
    return 1.0 if $x > 709;
    return 1.0 / (1.0 + exp(-$x));
}

sub sigmoid_derivative {
    my $x = shift;
    my $sig = sigmoid($x);
    return $sig * (1.0 - $sig);
}

sub relu {
    my $x = shift;
    return $x > 0.0 ? $x : 0.0;
}

sub relu_derivative {
    my $x = shift;
    return $x > 0.0 ? 1.0 : 0.0;
}

sub tanh {
    my $x = shift;
    my $ex = exp($x);
    my $emx = exp(-$x);
    return ($ex - $emx) / ($ex + $emx);
}

sub tanh_derivative {
    my $x = shift;
    my $t = tanh($x);
    return 1.0 - ($t * $t);
}

1;
