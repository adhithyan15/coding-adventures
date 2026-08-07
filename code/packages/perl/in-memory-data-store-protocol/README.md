# Perl in-memory data store protocol

A dependency-free protocol intermediate representation shared by in-memory data
store engines and transport adapters. It provides normalized command frames over
Perl byte strings and typed engine responses for strings, errors, integers, bulk
strings, and nested arrays.

```perl
my $frame = CodingAdventures::InMemoryDataStoreProtocol::CommandFrame
    ->from_parts(['set', 'key', 'value']);
print $frame->command; # SET

my $response = CodingAdventures::InMemoryDataStoreProtocol::EngineResponse
    ->array([
        CodingAdventures::InMemoryDataStoreProtocol::EngineResponse->ok,
        CodingAdventures::InMemoryDataStoreProtocol::EngineResponse->integer(1),
    ]);
```

Frame arrays and response arrays are defensively copied. This package models the
engine boundary only; RESP parsing and encoding remain separate.

## Development

```bash
bash BUILD
```
