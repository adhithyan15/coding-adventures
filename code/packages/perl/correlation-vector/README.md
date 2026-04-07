# CodingAdventures::CorrelationVector

A Perl implementation of the Correlation Vector (CV) system — append-only
provenance tracking for any data pipeline.

## What is it?

A Correlation Vector is like a "passport" for your data. When a piece of data
is born (a token in a compiler, a row in an ETL pipeline, a file in a build
system), it gets assigned a unique CV ID. Every system that touches it appends
a **contribution** to its CV record.

At any point you can ask: "Where did this data come from, and what happened to
it?" The CV log gives you a complete, ordered answer.

## Where it fits

This package implements `CV00-correlation-vector.md` from the specs. It is
domain-agnostic — the library knows nothing about compilers, ETL, or any
specific domain. Consumers attach meaning through `source` and `tag` fields.

Dependencies:
- `CodingAdventures::SHA256` — for computing 8-character hex base IDs
- `CodingAdventures::JsonSerializer` — for JSON serialization/deserialization

## Usage

```perl
use CodingAdventures::CorrelationVector;

# Create a CVLog for a pipeline run
my $log = CodingAdventures::CorrelationVector->new(enabled => 1);

# Assign a CV to a new entity
my $cv_id = $log->create(origin_string => "app.ts:5:12");
# Returns something like "a3f1b2c4.0"

# Record that a stage processed it
$log->contribute($cv_id,
    source => 'scope_analysis',
    tag    => 'resolved',
    meta   => { binding => 'local:count:fn_main' },
);

# Stage that made no changes
$log->passthrough($cv_id, source => 'type_checker');

# Split one entity into two
my $child_a = $log->derive($cv_id);
my $child_b = $log->derive($cv_id);

# Combine two entities into one
my $merged = $log->merge([$child_a, $child_b]);

# Mark an entity as intentionally removed
$log->delete($cv_id,
    by   => 'dead_code_eliminator',
    meta => { reason => 'unreachable from entry point' },
);

# Query the history
my $history = $log->history($cv_id);   # arrayref of contributions
my $parents = $log->ancestors($merged); # arrayref of parent cv_ids
my $kids    = $log->descendants($cv_id); # arrayref of child cv_ids
my $chain   = $log->lineage($merged);   # full chain, oldest first

# Cross-process transmission
my $json = $log->serialize();
my $log2 = CodingAdventures::CorrelationVector->deserialize($json);
```

## CV ID format

```
base.N         — root CV (sha256(origin)[:8] + counter)
base.N.M       — derived from base.N
base.N.M.K     — derived from base.N.M
00000000.N     — synthetic CV (no natural origin)
```

Reading parentage from the ID (without consulting the log) is a design goal:
the more dots, the deeper the derivation chain.

## Disabled mode

```perl
my $log = CodingAdventures::CorrelationVector->new(enabled => 0);
```

When `enabled => 0`:
- All write operations are no-ops (zero overhead)
- CV IDs are still generated and returned (entities still need their IDs)
- All query operations return `undef` or empty lists

Use this in production when you need IDs but not history.

## Running tests

```bash
# From this directory, with PERL5LIB set to include dependencies:
PERL5LIB=../sha256/lib:../grammar-tools/lib:../json-lexer/lib:../json-parser/lib:../json-value/lib:../json-serializer/lib prove -l -v t/
```

Or use the BUILD file with the monorepo build tool.

## Test coverage

9 subtests covering:
1. Root lifecycle (create, contribute, passthrough, delete)
2. Derivation (parent-child relationships)
3. Merging (multi-parent CVs)
4. Deep ancestry chains (A → B → C → D)
5. Disabled log (no-op behavior)
6. Serialization roundtrip (JSON encode/decode)
7. ID uniqueness (1000 creates, zero collisions)
8. Error handling (die on bad inputs)
9. Per-entity pass_order tracking

## Spec

See `code/specs/CV00-correlation-vector.md` for the full specification.
