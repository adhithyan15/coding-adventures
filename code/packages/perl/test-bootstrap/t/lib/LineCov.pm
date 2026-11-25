package LineCov;

# A small line-coverage recorder for the Fuzz tree (VER06q section 9), so the package's
# coverage gate needs no `Devel::Cover` (which is not core).
#
# It is a DEBUGGER: perl calls `DB::DB` before every statement when started with `-d` and
# `PERL5DB` set to load this file, and `@{"main::_<$file"}` then holds the source with a
# non-zero element for every BREAKABLE line, which is exactly "an executable line" (anonymous
# subs and file-level code included).  At exit each process appends its findings to the file
# named by `LINECOV_OUT`: one `file<TAB>line<TAB>hit` row per breakable line.  `report` merges
# any number of those files (a line is hit if any process hit it) and prints the percentage
# per file and in total.
#
#   for t in t/*.t; do
#     PERL5DB='BEGIN { require "./t/lib/LineCov.pm" }' LINECOV_OUT=/tmp/lc.out perl -d -Ilib $t
#   done
#   perl -It/lib -MLineCov -e 'LineCov::report("/tmp/lc.out")'
#
# This is a TEST tool: the engine never uses the debugger hooks (spec section 4.2).

use strict;
use warnings;

our ( %HIT, $FILTER );
BEGIN { $FILTER = qr{/TestBootstrap/Fuzz(?:/[A-Za-z]+)?\.pm\z} }

package DB;

sub DB {
    my ( undef, $file, $line ) = caller;
    $LineCov::HIT{$file}{$line} = 1 if defined $file && $file =~ $LineCov::FILTER;
    return;
}

# perl insists on a DB::sub when the sub-tracing flag is set; ours just calls through.
sub sub {
    no strict 'refs';
    return &$DB::sub;
}

package LineCov;

# line events (0x02) and the saved source arrays (0x10); `$DB::trace` makes every statement call DB::DB
BEGIN { $^P = 0x12 if $^P; $DB::trace = 1 }

END {
    my $out = $ENV{LINECOV_OUT};
    if ( defined $out && $^P ) {
        $^P = 0;
        if ( open my $fh, '>>', $out ) {
            no strict 'refs';
            for my $key ( sort keys %main:: ) {
                next if $key !~ /\A_<(.*)\z/s;
                my $file = $1;
                next if $file !~ $FILTER;
                my $lines = *{"main::$key"}{ARRAY} or next;
                for my $i ( 1 .. $#$lines ) {
                    next if !defined $lines->[$i] || !( $lines->[$i] + 0 );
                    print $fh join( "\t", $file, $i, $HIT{$file}{$i} ? 1 : 0 ), "\n";
                }
            }
            close $fh;
        }
    }
}

sub report {
    my @files = @_;
    my ( %lines, %hit );
    for my $f (@files) {
        open my $fh, '<', $f or die "cannot read $f: $!\n";
        while (<$fh>) {
            chomp;
            my ( $file, $line, $h ) = split /\t/;
            $lines{$file}{$line} = 1;
            $hit{$file}{$line} ||= $h;
        }
    }
    my ( $total, $covered ) = ( 0, 0 );
    for my $file ( sort keys %lines ) {
        my @l = keys %{ $lines{$file} };
        my $c = grep { $hit{$file}{$_} } @l;
        ( my $short = $file ) =~ s{.*/TestBootstrap/}{};
        printf "%6.1f%%  %4d/%-4d  %s\n", 100 * $c / @l, $c, scalar @l, $short;
        printf "          missed: %s\n", join( ' ', sort { $a <=> $b } grep { !$hit{$file}{$_} } @l ) if $ENV{LINECOV_MISSED} && $c < @l;
        $total   += @l;
        $covered += $c;
    }
    printf "%6.1f%%  %4d/%-4d  TOTAL\n", $total ? 100 * $covered / $total : 0, $covered, $total;
    return $total ? 100 * $covered / $total : 0;
}

1;
