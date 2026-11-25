use strict;
use warnings;
use CodingAdventures::Microbenchmark qw(analyze report_from_json);
my $pair = { baseline_ns => 100, candidate_ns => 100, order => 'baseline-first' };
my $result = analyze([$pair], 95, 1.05);
die "installed consumer failed" unless $result->{ratio} == 1 && $result->{verdict} eq 'inconclusive';
print "installed Perl packages consumed successfully\n";
if (@ARGV) {
    open my $fh, '<:raw', $ARGV[0] or die "cannot open report: $!";
    local $/;
    my $report = report_from_json(<$fh>);
    close $fh or die "cannot close report: $!";
    die "installed consumer expected 40 pairs" unless @{$report->{pairs}} == 40;
    print "installed Perl consumer replayed 40 pairs\n";
}
