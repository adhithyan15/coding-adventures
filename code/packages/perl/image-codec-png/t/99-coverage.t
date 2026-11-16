use strict;
use warnings;
use 5.026;
use File::Spec;
use Test2::V0;

# ============================================================================
# t/99-coverage.t
# ============================================================================
# Zero-dependency, offline line-coverage test harness.
# Executes the test suite under the built-in debugger profiler (t/lib/Devel/LineCov.pm),
# calculates exact statement-level line coverage across all package modules,
# prints a formatted coverage table, and asserts >= 95.0% coverage.
# Fully cross-platform on both POSIX (macOS/Linux) and Windows (Strawberry Perl).
# ============================================================================

my @modules = qw(
    lib/CodingAdventures/ImageCodecPNG.pm
    lib/CodingAdventures/ImageCodecPNG/Constants.pm
    lib/CodingAdventures/ImageCodecPNG/Messages.pm
    lib/CodingAdventures/ImageCodecPNG/Error.pm
    lib/CodingAdventures/ImageCodecPNG/Adler32.pm
    lib/CodingAdventures/ImageCodecPNG/Chunk.pm
    lib/CodingAdventures/ImageCodecPNG/Filter.pm
    lib/CodingAdventures/ImageCodecPNG/Decoder.pm
    lib/CodingAdventures/ImageCodecPNG/Encoder.pm
    lib/CodingAdventures/ImageCodecPNG/Codec.pm
);

my @test_files = qw(
    t/00-load.t
    t/01-corpus.t
    t/02-image-codec-png.t
    t/03-cve-resilience.t
);

my @inc_args = (
    '-Ilib',
    '-It/lib',
    '-I../pixel-container/lib',
    '-I../zip/lib',
    '-I../lzss/lib',
);

my $devnull = File::Spec->devnull();
my $tmp_cov = File::Spec->catfile(File::Spec->tmpdir(), "png_cov_$$.txt");
unlink $tmp_cov if -e $tmp_cov;
$ENV{PERL_LINECOV_OUT} = $tmp_cov;

# Run test files under -d:LineCov using list-form system() with stdio redirected to devnull
for my $t (@test_files) {
    open my $orig_out, '>&', STDOUT or die "Cannot dup STDOUT: $!";
    open my $orig_err, '>&', STDERR or die "Cannot dup STDERR: $!";
    open STDOUT, '>', $devnull or die "Cannot redirect STDOUT: $!";
    open STDERR, '>', $devnull or die "Cannot redirect STDERR: $!";

    my $rc = system($^X, @inc_args, '-d:LineCov', $t);

    open STDOUT, '>&', $orig_out or die "Cannot restore STDOUT: $!";
    open STDERR, '>&', $orig_err or die "Cannot restore STDERR: $!";
    close $orig_out;
    close $orig_err;

    is($rc, 0, "Test file $t succeeded under LineCov profiler");
}

# Collect hits from $tmp_cov
my %hits;
if (-e $tmp_cov) {
    open my $fh, '<', $tmp_cov or die "Cannot open $tmp_cov: $!";
    while (<$fh>) {
        chomp;
        my ($f, $l) = split /\t/;
        next unless defined $f && defined $l;
        $f =~ tr{\\}{/};
        for my $mod (@modules) {
            if ($f =~ m{\Q$mod\E$}) {
                $hits{$mod}{$l}++;
            }
        }
    }
    close $fh;
    unlink $tmp_cov;
    delete $ENV{PERL_LINECOV_OUT};
}

# Dump statement lines via cross-platform helper script
my $tmp_lines = File::Spec->catfile(File::Spec->tmpdir(), "png_lines_$$.txt");
unlink $tmp_lines if -e $tmp_lines;

{
    open my $orig_out, '>&', STDOUT or die "Cannot dup STDOUT: $!";
    open my $orig_err, '>&', STDERR or die "Cannot dup STDERR: $!";
    open STDOUT, '>', $devnull or die "Cannot redirect STDOUT: $!";
    open STDERR, '>', $devnull or die "Cannot redirect STDERR: $!";

    my $helper = File::Spec->catfile('t', 'lib', 'dump_cov_lines.pl');
    system($^X, @inc_args, '-d:LineCov', $helper, $tmp_lines);

    open STDOUT, '>&', $orig_out or die "Cannot restore STDOUT: $!";
    open STDERR, '>&', $orig_err or die "Cannot restore STDERR: $!";
    close $orig_out;
    close $orig_err;
}

my %total;
my %uncovered;

if (-e $tmp_lines) {
    open my $fh, '<', $tmp_lines or die "Cannot open $tmp_lines: $!";
    while (<$fh>) {
        chomp;
        my ($mod, $lstr) = split /\t/;
        next unless defined $mod && defined $lstr;
        $mod =~ tr{\\}{/};
        my @lines = split /,/, $lstr;
        $total{$mod} = \@lines;
        for my $l (@lines) {
            if (!$hits{$mod}{$l}) {
                push @{$uncovered{$mod}}, $l;
            }
        }
    }
    close $fh;
    unlink $tmp_lines;
}

my $grand_total = 0;
my $grand_hits = 0;

diag("-" x 76);
diag(sprintf("%-48s %8s %8s %8s", "File", "Covered", "Total", "Percent"));
diag("-" x 76);

for my $f (@modules) {
    my $tot = scalar(@{$total{$f} || []});
    my $cov = $tot - scalar(@{$uncovered{$f} || []});
    my $pct = $tot ? ($cov / $tot * 100) : 100;
    $grand_total += $tot;
    $grand_hits += $cov;
    diag(sprintf("%-48s %8d %8d %7.2f%%", $f, $cov, $tot, $pct));
    if ($uncovered{$f} && @{$uncovered{$f}}) {
        diag("  Uncovered in $f: " . join(", ", @{$uncovered{$f}}));
    }
    cmp_ok($pct, '>=', 95.0, "Module $f line coverage >= 95.0% ($pct%)");
}

diag("-" x 76);
my $grand_pct = $grand_total ? ($grand_hits / $grand_total * 100) : 100;
diag(sprintf("%-48s %8d %8d %7.2f%%", "TOTAL", $grand_hits, $grand_total, $grand_pct));
diag("-" x 76);

cmp_ok($grand_pct, '>=', 95.0, "Overall line coverage >= 95.0% ($grand_pct%)");

done_testing;
