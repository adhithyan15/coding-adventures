use strict;
use warnings;
use CodingAdventures::RankStatistics qw(median median_interval);
die "median failed" unless median([1, 3, 2]) == 2;
my $interval = median_interval([1, 2, 3, 4], 75);
die "interval failed" unless $interval->[0] == 1 && $interval->[1] == 4;
print "installed Perl rank-statistics consumer passed\n";
