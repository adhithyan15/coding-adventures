use strict;
use warnings;
use Test::More;

ok(eval { require CodingAdventures::BinaryTree; 1 }, 'CodingAdventures::BinaryTree loads');
ok(CodingAdventures::BinaryTree->VERSION, 'has a VERSION');
ok(CodingAdventures::BinaryTree::Node->can('new'), 'node class is available');

done_testing;
