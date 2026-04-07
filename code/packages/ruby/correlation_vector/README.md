# coding_adventures_correlation_vector

Append-only provenance tracking for any entity in any pipeline.

## What Is a Correlation Vector?

A Correlation Vector (CV) is a lightweight record that follows a piece of data through every transformation it undergoes. Assign a CV to any entity when it is born. Every system, stage, or function that touches it appends its contribution. At any point you can ask "where did this come from and what happened to it?" and get a complete, ordered answer.

The concept originated in distributed systems tracing. This implementation generalizes it to any pipeline — compiler passes, data ETL, document transformations, build systems, ML preprocessing, or anywhere that data flows through a sequence of transformations.

**This gem is domain-agnostic.** It knows nothing about compilers, JavaScript, or pipelines. It is a data structure and a set of operations. Consumers attach semantic meaning through the `source` and `tag` fields.

## How It Fits in the Stack

```
coding_adventures_sha256          -- ID generation (hashing origin strings)
coding_adventures_json_serializer -- Serialization (CVLog -> JSON)
         ↓
coding_adventures_correlation_vector   ← YOU ARE HERE
         ↓
Your pipeline (compiler, ETL, build system, etc.)
```

## ID Format

CV IDs use a dot-extension scheme that encodes parentage visually:

```
a3f1b2c4.1       root CV — first entity born with this hash base
a3f1b2c4.2       root CV — second entity born with this hash base
a3f1b2c4.1.1     first entity derived from a3f1b2c4.1
a3f1b2c4.1.2     second entity derived from a3f1b2c4.1
a3f1b2c4.1.1.1   entity derived from a3f1b2c4.1.1
00000000.1       synthetic entity — no natural origin
```

You can read parentage directly from the ID by counting the dots.

## Installation

In your Gemfile:

```ruby
gem "coding_adventures_correlation_vector"
```

## Usage Examples

### Compiler pipeline

```ruby
require "coding_adventures_correlation_vector"

log = CodingAdventures::CorrelationVector::CVLog.new

# Each AST node gets a CV at parse time
cv_id = log.create(origin_string: "app.ts:5:12")

# Every pass appends its contribution
log.contribute(cv_id, source: "scope_analysis", tag: "resolved",
               meta: { "binding" => "local:count:fn_main" })
log.contribute(cv_id, source: "variable_renamer", tag: "renamed",
               meta: { "from" => "count", "to" => "a" })

# A stage that saw the entity but changed nothing
log.passthrough(cv_id, source: "type_checker")

# Dead code elimination marks it deleted
log.delete(cv_id, by: "dead_code_eliminator")

# Full history of what happened
log.history(cv_id).each do |c|
  puts "#{c.source} (#{c.tag}): #{c.meta}"
end
```

### ETL pipeline with derivation

```ruby
log = CodingAdventures::CorrelationVector::CVLog.new

# Row arrives from the source
row_cv = log.create(origin_string: "orders_table:row_id:8472")
log.contribute(row_cv, source: "validator", tag: "schema_checked",
               meta: { "schema" => "orders_v2", "result" => "pass" })

# Splitter produces two narrower records
left_cv = log.derive(row_cv, source: "splitter", tag: "left_columns")
right_cv = log.derive(row_cv, source: "splitter", tag: "right_columns")

# Joining two records into one output
joined_cv = log.merge([left_cv, right_cv], source: "joiner", tag: "joined")

# Ancestry: who are the parents of joined_cv?
puts log.ancestors(joined_cv)
# => [left_cv, right_cv, row_cv]

# Descendants: what came from row_cv?
puts log.descendants(row_cv)
# => [left_cv, right_cv, joined_cv]
```

### Serialization

```ruby
# Serialize to JSON (for storage or cross-process transmission)
json = log.serialize

# Restore from JSON
restored = CodingAdventures::CorrelationVector::CVLog.deserialize(json)
```

### Disabled mode (production, zero overhead)

```ruby
# Create a log with tracing off
log = CodingAdventures::CorrelationVector::CVLog.new(enabled: false)

# create() still returns cv_ids (needed by entities)
# but nothing is stored in the log
cv_id = log.create(origin_string: "file.ts")
log.get(cv_id)  # => nil (nothing stored)
```

## API Reference

### `CVLog.new(enabled: true)`

Create a new log. Pass `enabled: false` to disable all write operations.

### `create(origin_string: nil, synthetic: false, meta: nil) → String`

Born a new root CV. Returns the cv_id.
- `origin_string` — human-readable origin identifier (file path, table name, etc.)
- `synthetic: true` — entity has no natural origin; uses `00000000` base

### `contribute(cv_id, source:, tag:, meta: nil) → nil`

Record that a stage processed this entity. Raises if entity not found or deleted.

### `derive(parent_cv_id, source:, tag:, meta: nil) → String`

Create a child entity. Returns the new child's cv_id.

### `merge(cv_ids, source:, tag:, meta: nil) → String`

Combine multiple entities into one. Returns the merged entity's cv_id.

### `delete(cv_id, by:) → nil`

Mark an entity as deleted. The entry remains in the log permanently.

### `passthrough(cv_id, source:) → String`

Record that a stage saw this entity but changed nothing. Returns the same cv_id.

### `get(cv_id) → CVEntry | nil`

Look up a CV entry by ID. Returns nil if not found.

### `ancestors(cv_id) → Array<String>`

Walk the parent chain. Returns ancestor IDs nearest-first.

### `descendants(cv_id) → Array<String>`

Find all entities that descend from this one.

### `history(cv_id) → Array<Contribution>`

Return the contributions for an entity in order.

### `lineage(cv_id) → Array<CVEntry>`

Return the entity and all its ancestors, oldest-first.

### `serialize → String`

Serialize the entire log to JSON.

### `CVLog.deserialize(json_string) → CVLog`

Reconstruct a log from its JSON representation.

## Test Coverage

100% line coverage. 72 tests covering:
- Root lifecycle (create, contribute, passthrough, delete, errors)
- Derivation (child IDs, ancestors, descendants)
- Merging (3-way merge, multi-parent ancestry)
- Deep ancestry chains (4-level, nearest-first / oldest-first)
- Disabled log (enabled: false, no storage, IDs still generated)
- Serialization roundtrip (JSON persistence)
- ID uniqueness (1000+ creates with no collisions)

## Related Specs

- `code/specs/CV00-correlation-vector.md` — Full specification
- `code/specs/IR00-semantic-ir.md` — How IR nodes use CV for provenance
