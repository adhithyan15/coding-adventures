use strict;
use warnings;
use Test::More;

ok(eval { require CodingAdventures::BinarySearchTree; 1 }, 'CodingAdventures::BinarySearchTree loads');
ok(CodingAdventures::BinarySearchTree->VERSION, 'has a VERSION');
ok(CodingAdventures::BinarySearchTree::Node->can('new'), 'node class is available');

done_testing;
