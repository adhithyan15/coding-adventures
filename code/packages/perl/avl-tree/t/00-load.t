use strict;
use warnings;
use Test::More;

ok(eval { require CodingAdventures::AVLTree; 1 }, 'CodingAdventures::AVLTree loads');
ok(CodingAdventures::AVLTree->VERSION, 'has a VERSION');
ok(CodingAdventures::AVLTree::Node->can('new'), 'node class is available');

done_testing;
