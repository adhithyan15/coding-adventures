# CodingAdventures::InMemoryDataStore

A pure Perl facade that composes the RESP2 streaming codec, command protocol
IR, and in-memory data store engine. It accepts fragmented or pipelined byte
streams, preserves binary-safe bulk strings, and returns native RESP values or
encoded response streams without opening sockets or using external services.

```perl
use CodingAdventures::InMemoryDataStore;

my $store = CodingAdventures::InMemoryDataStore->new;
my $response = $store->execute_parts(['SET', 'name', 'Ada']);
die 'SET failed' if $response->value ne 'OK';

my $wire = $store->handle("*2\r\n\$3\r\nGET\r\n\$4\r\nname\r\n");
die 'GET failed' if $wire ne "\$3\r\nAda\r\n";
```

The facade depends only on the sibling pure-Perl `RespProtocol`,
`InMemoryDataStoreProtocol`, and `InMemoryDataStoreEngine` packages.
