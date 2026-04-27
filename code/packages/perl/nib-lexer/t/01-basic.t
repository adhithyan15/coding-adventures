use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::NibLexer; 1 }, 'module loads' );

my $tokens = CodingAdventures::NibLexer->tokenize('fn main() { return 0; }');
is($tokens->[0]{type}, 'FN', 'fn is lexed as promoted keyword token');
is($tokens->[0]{value}, 'fn', 'fn value preserved');
is($tokens->[1]{type}, 'NAME', 'main is lexed as identifier');

my $ops = CodingAdventures::NibLexer->tokenize('1 +% 2 +? 3');
is([map { $_->{value} } @$ops], ['1', '+%', '2', '+?', '3', ''], 'multicharacter operators stay intact');

done_testing;
