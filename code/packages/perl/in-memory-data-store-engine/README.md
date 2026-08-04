# In-Memory Data Store Engine (Perl)

A pure Perl execution engine for the repository's in-memory data store stack.
It consumes `CommandFrame` objects from the sibling protocol package and
returns the shared `EngineResponse` IR.

```perl
use CodingAdventures::InMemoryDataStoreEngine;

my $engine = CodingAdventures::InMemoryDataStoreEngine->new;
$engine->execute_parts(['SET', 'answer', '41']);
die unless $engine->execute_parts(['INCR', 'answer'])->value == 42;
```

The engine implements binary-safe strings, hashes, lists, sets, sorted sets,
HyperLogLog, expiry and persistence, globbed key lookup, 16 logical databases,
and administrative commands. Its 57-command surface matches the Redis-style
string, hash, list, set, sorted-set, HLL, TTL, database, and server operations
implemented by the other language lanes.

The constructor accepts optional `store`, `database_count`, and `time_provider`
options. The clock hook makes TTL behavior deterministic in tests without
filesystem, network, process, environment, or randomness access.

Run the package gate from this directory with `BUILD` or `BUILD_windows`.
