# CodingAdventures::RespProtocol

A dependency-free Perl encoder and incremental decoder for the Redis
Serialization Protocol (RESP2). Typed values preserve all five RESP2 kinds and
distinguish null bulk strings from null arrays.

```perl
use CodingAdventures::RespProtocol qw(encode decode);

my $Value = 'CodingAdventures::RespProtocol::Value';
my $command = $Value->array([
    $Value->bulk_string('PING'),
    $Value->bulk_string('hello'),
]);

my $wire = encode($command);
my ($decoded, $next_offset) = @{decode($wire)};
```

Bulk strings are binary-safe. Incomplete frames return `undef` without
consuming input; malformed frames raise an exception. The streaming decoder
handles arbitrary fragmentation and multiple messages in one chunk.
