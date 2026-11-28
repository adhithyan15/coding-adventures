use strict;
use warnings;

my ($report_path, $expected_file) = @ARGV;
die "usage: perl check_coverage.pl REPORT EXPECTED_FILE\n" unless $report_path && $expected_file;
open my $report_fh, '<', $report_path or die "cannot open coverage report: $!\n";
my $report = do { local $/; <$report_fh> };
close $report_fh or die "cannot close coverage report: $!\n";

my ($percent, $aggregate);
for my $line (split /\R/, $report) {
    my ($file, $statement, $all) = $line =~ /^\s*(\S+)\s+([\d.]+)\s+([\d.]+)\s*$/;
    next unless defined $file;
    next if $file eq 'Total';
    die "unexpected Perl source in coverage denominator: $file\n" if $file ne $expected_file;
    ($percent, $aggregate) = ($statement, $all);
}

die "expected Perl source file has no measurable statements\n" unless defined($percent) && defined($aggregate);
die "statement coverage must exceed 80%, got $percent%\n" unless $percent > 80;
print "Production statement coverage: $percent%\n";
