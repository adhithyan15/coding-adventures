use strict;
use warnings;
use Test2::V0;
use CodingAdventures::Twig;

my ($stdout, $value) = CodingAdventures::Twig::run_twig('(define (inc x) (+ x 1)) (inc 41)');
is($value, 42, 'runs functions on the LANG VM');

($stdout, $value) = CodingAdventures::Twig::run_twig('(print (+ 1 2))');
is($stdout, "3\n", 'captures print output');
ok(!defined $value, 'print returns nil');

done_testing();
