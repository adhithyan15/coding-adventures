use strict;
use warnings;
use Test::More;

ok(eval { require CodingAdventures::InMemoryDataStoreProtocol; 1 }, 'protocol module loads');
ok(CodingAdventures::InMemoryDataStoreProtocol->VERSION, 'has a VERSION');
ok(CodingAdventures::InMemoryDataStoreProtocol::CommandFrame->can('new'), 'command frame is available');
ok(CodingAdventures::InMemoryDataStoreProtocol::EngineResponse->can('new'), 'engine response is available');

done_testing;
