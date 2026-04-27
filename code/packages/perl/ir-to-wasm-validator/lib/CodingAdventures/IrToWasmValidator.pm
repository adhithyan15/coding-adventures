package CodingAdventures::IrToWasmValidator;

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(validate);

use CodingAdventures::IrToWasmCompiler qw(compile);

sub validate {
    my ($program, $function_signatures) = @_;
    eval { compile($program, $function_signatures); 1 }
        ? []
        : [ { rule => 'lowering', message => "$@" } ];
}

1;
