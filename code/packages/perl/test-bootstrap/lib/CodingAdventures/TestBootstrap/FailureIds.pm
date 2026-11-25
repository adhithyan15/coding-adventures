package CodingAdventures::TestBootstrap::FailureIds;

# The closed failure vocabulary (VER05 5.1).  `property-timeout` is registered
# but never produced: there is no wall-clock timeout, so a property that loops
# forever hangs the run (tests bound it with `alarm`).

use strict;
use warnings;

use constant {
    COUNTEREXAMPLE => 'property-counterexample',
    EXHAUSTED      => 'property-exhausted',
    TIMEOUT        => 'property-timeout',
    PANIC          => 'property-panic',
};

our @ALL = ( COUNTEREXAMPLE, EXHAUSTED, TIMEOUT, PANIC );

# True only for a plain string equal to one of the four registered ids.
sub is_failure_id {
    my ($candidate) = @_;
    return 0 if !defined $candidate || ref $candidate;
    for my $id (@ALL) {
        return 1 if $candidate eq $id;
    }
    return 0;
}

1;
