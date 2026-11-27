package CodingAdventures::TestBootstrap::Fuzz::Hooks;

# Sabotage hooks: the engine entry points the corpus runner calls, as swappable subs.  These
# are the only production indirections added for the negative controls (VER06q section 9): a
# test installs a broken version with `with`, runs the corpus, and the runner must report
# problems.  Production code never writes them.  `%H` is a documented test seam, restored by
# `with` on every exit (including `die`); the corpus runner COPIES it when a run starts, so
# code that changes it while a run is going cannot change that run.

use strict;
use warnings;

use CodingAdventures::TestBootstrap::Tfv                ();
use CodingAdventures::TestBootstrap::Fuzz::Mutate       ();
use CodingAdventures::TestBootstrap::Fuzz::Bitmap       ();
use CodingAdventures::TestBootstrap::Fuzz::Signature    ();
use CodingAdventures::TestBootstrap::Fuzz::FailureIds   ();

sub defaults {
    return (
        bit_flip         => \&CodingAdventures::TestBootstrap::Fuzz::Mutate::bit_flip,
        byte_flip        => \&CodingAdventures::TestBootstrap::Fuzz::Mutate::byte_flip,
        byte_arithmetic  => \&CodingAdventures::TestBootstrap::Fuzz::Mutate::byte_arithmetic,
        boundary_insert  => \&CodingAdventures::TestBootstrap::Fuzz::Mutate::boundary_insert,
        block_delete     => \&CodingAdventures::TestBootstrap::Fuzz::Mutate::block_delete,
        block_insert     => \&CodingAdventures::TestBootstrap::Fuzz::Mutate::block_insert,
        block_duplicate  => \&CodingAdventures::TestBootstrap::Fuzz::Mutate::block_duplicate,
        splice           => \&CodingAdventures::TestBootstrap::Fuzz::Mutate::splice_bytes,
        mutate_sequence  => \&CodingAdventures::TestBootstrap::Fuzz::Mutate::mutate_sequence_counted,
        corpus_admit     => \&CodingAdventures::TestBootstrap::Fuzz::Bitmap::corpus_admit,
        duplicate_crash  => \&CodingAdventures::TestBootstrap::Fuzz::Signature::duplicate_crash,
        failure_id       => \&CodingAdventures::TestBootstrap::Fuzz::FailureIds::is_fuzz_failure_id,
        format_tap       => \&CodingAdventures::TestBootstrap::Fuzz::Signature::format_tap_failure,
        tfv_valid        => \&CodingAdventures::TestBootstrap::Tfv::valid,
    );
}

our %H = defaults();

sub reset { %H = defaults(); return }

# Runs `$code->(@args)` with the overrides installed, restoring every hook on every exit.
# Returns what `$code` returns (scalar context); rethrows what it throws.
sub with {
    my ( $overrides, $code, @args ) = @_;
    my %known = defaults();
    for my $k ( keys %$overrides ) {
        die "unknown hook\n" if !exists $known{$k};
    }
    local %H = ( %H, %$overrides );
    return $code->(@args);
}

1;
