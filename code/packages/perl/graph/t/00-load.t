#!/usr/bin/env perl

use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::Graph }, 'CodingAdventures::Graph loads');

my $graph = CodingAdventures::Graph->new();
ok(defined $graph, 'Graph constructor works');

done_testing();
