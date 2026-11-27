package CodingAdventures::TestBootstrap::Fuzz::FailureIds;

# The closed failure vocabulary (VER06 section 6.2).  Nothing else is ever printed
# in a public failure line.

use strict;
use warnings;

use constant {
    CRASH   => 'fuzz-crash',
    TIMEOUT => 'fuzz-timeout',
    OOM     => 'fuzz-oom',
    LEAK    => 'fuzz-leak',
    PANIC   => 'fuzz-panic',
};

our @ALL = ( CRASH, TIMEOUT, OOM, LEAK, PANIC );

# True only for a plain string equal to one of the five registered ids.
#
#   is_fuzz_failure_id('fuzz-crash')           --> 1
#   is_fuzz_failure_id('fuzz-arbitrary-error') --> 0
#   is_fuzz_failure_id(undef)                  --> 0
sub is_fuzz_failure_id {
    my ($candidate) = @_;
    return 0 if !defined $candidate || ref $candidate;
    for my $id (@ALL) {
        return 1 if $candidate eq $id;
    }
    return 0;
}

1;
