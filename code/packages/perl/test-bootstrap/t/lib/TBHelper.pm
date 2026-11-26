package TBHelper;

# Shared helpers for the test files: the repository root, raw file reads, an
# `alarm` bound for code that could loop, and trap objects that record and die in
# every overload and tie method (VER05p section 10).

use strict;
use warnings;
use Cwd ();
use File::Spec ();
use FindBin ();
use Exporter 'import';

use CodingAdventures::TestBootstrap;

our @EXPORT_OK = qw(repo_root read_file bounded_by $TB dec int_of lives holds_all);

our $TB = 'CodingAdventures::TestBootstrap';

sub dec { return CodingAdventures::TestBootstrap::Int::tostring( $_[0] ) }
sub int_of { return CodingAdventures::TestBootstrap::Int::parse( $_[0] ) }

# Walks up from the test directory to the repository root (the directory that
# holds code/specs/fixtures/pbt-v1/cases.json).
sub repo_root {
    my $dir = Cwd::abs_path($FindBin::Bin);
    while (1) {
        return $dir if -f File::Spec->catfile( $dir, 'code', 'specs', 'fixtures', 'pbt-v1', 'cases.json' );
        my $up = File::Spec->catdir( $dir, File::Spec->updir );
        my $abs = Cwd::abs_path($up);
        die "repository root not found above $FindBin::Bin\n" if !defined $abs || $abs eq $dir;
        $dir = $abs;
    }
}

sub read_file {
    my ($path) = @_;
    open my $fh, '<:raw', $path or die "cannot read $path: $!\n";
    local $/;
    my $data = <$fh>;
    close $fh;
    return $data;
}

# Runs `$code->()` under an alarm that dies, so a regression fails instead of
# hanging.  Returns what `$code` returns; rethrows what it throws.
sub bounded_by {
    my ( $seconds, $code ) = @_;
    my @r;
    my $have_alarm = eval { alarm 0; 1 };    # not every platform implements alarm
    return $code->() if !$have_alarm;
    my $ok = eval {
        local $SIG{ALRM} = sub { die "TIMEOUT: still running after ${seconds}s\n" };
        alarm $seconds;
        @r = $code->();
        alarm 0;
        1;
    };
    my $err = $@;
    alarm 0;
    die $err if !$ok;
    return wantarray ? @r : $r[0];
}

sub lives { my ($c) = @_; my $ok = eval { $c->(); 1 }; return $ok ? 1 : 0 }

1;
