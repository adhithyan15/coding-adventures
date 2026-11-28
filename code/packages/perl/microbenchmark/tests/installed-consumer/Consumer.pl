use strict;
use warnings;
use CodingAdventures::Microbenchmark qw(analyze report_from_json);
my $pair = { baseline_ns => 100, candidate_ns => 100, order => 'baseline-first' };
my $result = analyze([$pair], 95, 1.05);
die "installed consumer failed" unless $result->{ratio} == 1 && $result->{verdict} eq 'inconclusive';
print "installed Perl packages consumed successfully\n";
if (@ARGV) {
    open my $fh, '<:raw', $ARGV[0] or die "cannot open report: $!";
    my $raw = '';
    while (length($raw) <= 4 * 1024 * 1024) {
        my $chunk = '';
        my $read = sysread($fh, $chunk, 4 * 1024 * 1024 + 1 - length($raw));
        die "cannot read report: $!" unless defined $read;
        last if $read == 0;
        $raw .= $chunk;
    }
    close $fh or die "cannot close report: $!";
    die "report exceeds JSON byte limit" if length($raw) > 4 * 1024 * 1024;
    my $report = report_from_json($raw);
    die "installed consumer expected 40 pairs" unless @{$report->{pairs}} == 40;
    print "installed Perl consumer replayed 40 pairs\n";
}
