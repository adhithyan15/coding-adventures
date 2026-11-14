package Devel::LineCov;

use strict;
use warnings;

our %HITS;

END {
    if (my $out = $ENV{PERL_LINECOV_OUT}) {
        open my $fh, '>>', $out or die "Cannot open $out: $!";
        for my $file (keys %HITS) {
            next unless $file =~ m{CanonicalCbor};
            for my $line (keys %{$HITS{$file}}) {
                print $fh "$file\t$line\n";
            }
        }
        close $fh;
    }
}

package DB;

sub DB {
    my ($pkg, $file, $line) = caller();
    $Devel::LineCov::HITS{$file}{$line}++;
}

1;
