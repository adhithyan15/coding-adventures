use strict;
use warnings;
use Test::More;
no warnings 'redefine';

sub is ($$;$) {
    my ($got, $expected, $name) = @_;
    if (ref($got) || ref($expected)) {
        return Test::More::is_deeply($got, $expected, $name);
    }
    return Test::More::is($got, $expected, $name);
}

ok( eval { require CodingAdventures::HuffmanTree; 1 }, 'CodingAdventures::HuffmanTree loads' );

ok defined $CodingAdventures::HuffmanTree::VERSION, 'VERSION is defined';
is $CodingAdventures::HuffmanTree::VERSION, '0.1.0', 'VERSION is 0.1.0';

done_testing;
