-- correlation_vector — Append-only provenance tracking for any entity
-- ====================================================================
--
-- A Correlation Vector (CV) follows a piece of data through every
-- transformation it undergoes.  Assign a CV to anything at birth; every
-- system, stage, or function that touches it appends its contribution.
-- At any point you can ask "where did this come from and what happened
-- to it?" and get a complete, ordered answer.
--
-- # Why would I want this?
--
-- Imagine a compiler that transforms source code through a dozen passes.
-- Without provenance tracking, when a variable disappears from the output
-- you have to add debugging prints and re-run — maybe several times.
-- With CVs, you query `cvlog:history(var_cv_id)` and instantly see:
--
--   parser         → created as IDENTIFIER
--   scope_analysis → resolved to local binding
--   dce            → deleted: unreachable from entry: main
--
-- The same pattern applies to ETL pipelines, build systems, ML preprocessing,
-- document transformations, or any pipeline where data flows through stages.
--
-- # Core idea: the CV ID
--
-- Every entity gets a stable, globally unique string called a CV ID at birth:
--
--   base.N           a root CV (no parents)
--   base.N.M         derived from base.N
--   base.N.M.K       derived from base.N.M
--
-- The dot-extension scheme encodes parentage directly in the ID.  You can
-- read the depth of nesting without consulting the log.
--
-- # Dependency graph
--
--   correlation_vector  ← this package
--        ↓
--   sha256  (for deterministic base-ID generation)
--   json_serializer  (for serialize / deserialize)

local sha256 = require("coding_adventures.sha256")
local json   = require("coding_adventures.json_serializer")

-- The module table.  All public symbols live here.
local M = {}
M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- Internal helpers
-- ---------------------------------------------------------------------------

-- set_contains(set, value)
-- Returns true when value is already present in the set (a Lua table used as
-- a lookup map, where set[value] = true means "present").
--
-- We implement "deduplicated ordered lists" with two parallel structures:
--   array  — keeps insertion order for iteration
--   set    — O(1) membership test
local function set_contains(set, value)
  return set[value] == true
end

-- dedup_append(array, set, value)
-- Appends value to array (and marks it in set) only if it is not already present.
-- This gives us a deduplicated, order-preserving list without O(n) scans.
local function dedup_append(array, set, value)
  if not set_contains(set, value) then
    array[#array + 1] = value
    set[value] = true
  end
end

-- now_iso()
-- Returns the current UTC time as an ISO 8601 string, e.g. "2026-04-06T13:00:00Z".
-- We use `!` prefix in os.date to force UTC (the `!` tells Lua to use gmtime).
local function now_iso()
  return os.date("!%Y-%m-%dT%H:%M:%SZ")
end

-- make_base(origin_string, synthetic)
-- Computes the 8-character hex prefix used as the "base" segment of a CV ID.
--
-- For synthetic entities (no natural origin), the base is always "00000000".
-- This is a deliberate constant: it signals "this entity was created
-- programmatically, not derived from any real-world source."
--
-- For real entities, we SHA-256 hash the origin string and take the first
-- 8 hex characters.  8 hex chars = 4 bytes = 2^32 possible bases.  Collisions
-- are possible for very large graphs but extremely unlikely in practice.
local function make_base(origin_string, synthetic)
  if synthetic then
    return "00000000"
  end
  -- sha256_hex returns a 64-character lowercase hex string.
  -- We take only the first 8 characters as the base segment.
  local hex = sha256.sha256_hex(origin_string or "")
  return hex:sub(1, 8)
end

-- ---------------------------------------------------------------------------
-- CVLog constructor
-- ---------------------------------------------------------------------------

-- M.new(opts) → cvlog
--
-- Creates and returns a new CVLog object.  The CVLog is the central data
-- structure that holds all CV entries for a pipeline run.
--
-- opts (optional table):
--   enabled (boolean, default true) — when false, mutating operations
--     (create, contribute, derive, merge, delete, passthrough) are no-ops.
--     CV IDs are still allocated and returned so entities can carry their
--     cv_id field, but nothing is written to the log.  This lets you ship
--     the same code in production (tracing off) and debugging (tracing on)
--     with zero branching in application code.
--
-- The returned object has the following private fields (prefixed with _):
--   _enabled  — boolean, controls whether writes are stored
--   _entries  — table mapping cv_id (string) → entry table
--   _counter  — integer, monotonically increasing per-log sequence number
--   _pass_order       — array of source names in contribution order
--   _pass_order_set   — set (lookup table) for O(1) dedup of _pass_order
function M.new(opts)
  -- Determine enabled flag.  Default is true (tracing on).
  -- We use the explicit `~= nil` check so that passing `enabled = false`
  -- works even when other fields in opts are absent.
  local enabled
  if opts ~= nil and opts.enabled ~= nil then
    enabled = opts.enabled
  else
    enabled = true
  end

  local self = {
    _enabled        = enabled,
    _entries        = {},   -- cv_id string → entry table
    _counter        = 0,    -- next sequence number to assign
    _pass_order     = {},   -- ordered, deduplicated list of source names
    _pass_order_set = {},   -- set companion for O(1) membership test
  }

  -- -----------------------------------------------------------------------
  -- Core mutation operations
  -- -----------------------------------------------------------------------

  -- cvlog:create(opts) → cv_id string
  --
  -- Born a new root CV.  The entity has no parents — it was created from
  -- scratch or from an external source.
  --
  -- opts (optional):
  --   origin_string (string) — identifies the birth source, e.g. a file
  --     path + position.  Hashed to produce the base segment of the ID.
  --   synthetic (boolean) — if true, use "00000000" as base regardless
  --     of origin_string.  Use this for entities with no real-world origin.
  --   meta (table) — arbitrary key-value context stored on the entry.
  --
  -- ID scheme:
  --   base = synthetic ? "00000000" : sha256(origin_string)[0..8]
  --   cv_id = base .. "." .. counter (where counter starts at 0)
  function self:create(opts)
    opts = opts or {}
    local base = make_base(opts.origin_string, opts.synthetic)

    -- Allocate the next sequence number.  We increment first so that the
    -- counter always reflects "how many IDs have been allocated so far."
    self._counter = self._counter + 1
    local n      = self._counter - 1  -- IDs start at 0

    local cv_id = base .. "." .. n

    -- If tracing is disabled, we still allocate the ID (the entity needs it)
    -- but we do NOT write an entry into the log.  This is the zero-overhead
    -- production mode.
    if not self._enabled then
      return cv_id
    end

    -- Build the origin record.  nil if no origin was supplied.
    local origin = nil
    if opts.origin_string then
      origin = {
        string    = opts.origin_string,
        synthetic = opts.synthetic or false,
      }
    elseif opts.synthetic then
      origin = { string = nil, synthetic = true }
    end

    -- Create the entry.  Fields follow the CVEntry shape from the spec:
    --   cv_id         — stable identity
    --   origin        — where/when born
    --   parent_cv_id  — nil for roots
    --   merged_from   — empty for roots
    --   contributions — append-only history
    --   deleted       — nil until the entity is deleted
    --   pass_order    — deduplicated source names that touched this entity
    self._entries[cv_id] = {
      cv_id         = cv_id,
      origin        = origin,
      parent_cv_id  = nil,
      merged_from   = {},
      contributions = {},
      deleted       = nil,
      pass_order    = {},
      _pass_order_set = {},  -- private set for dedup (not serialized)
    }

    return cv_id
  end

  -- cvlog:contribute(cv_id, opts) → nil
  --
  -- Records that a stage processed this entity.  Contributions are the
  -- primary narrative of what happened to an entity over its lifetime.
  --
  -- opts (required):
  --   source (string) — who/what contributed, e.g. "scope_analysis"
  --   tag    (string) — what happened, e.g. "resolved"
  --   meta   (table, optional) — arbitrary detail, e.g. {binding = "local:x"}
  --
  -- Errors:
  --   CV not found — the cv_id was never created in this log
  --   Deleted CV   — you cannot add contributions to a deleted entity
  --
  -- Pass order: we also maintain a global pass_order on the log itself
  -- (which sources have contributed to ANY entity in this log, in order of
  -- first appearance).  This mirrors the CVLog.pass_order field in the spec.
  function self:contribute(cv_id, opts)
    if not self._enabled then return nil end

    local entry = self._entries[cv_id]
    if not entry then
      error("CV not found: " .. tostring(cv_id))
    end
    if entry.deleted then
      error("Cannot contribute to deleted CV: " .. tostring(cv_id))
    end

    opts = opts or {}
    local contribution = {
      source    = opts.source,
      tag       = opts.tag,
      meta      = opts.meta or {},
      timestamp = now_iso(),
    }

    -- Append to this entry's contribution history.
    entry.contributions[#entry.contributions + 1] = contribution

    -- Update this entry's own pass_order (deduplicated).
    dedup_append(entry.pass_order, entry._pass_order_set, opts.source)

    -- Update the log-wide pass_order (deduplicated).
    dedup_append(self._pass_order, self._pass_order_set, opts.source)

    return nil
  end

  -- cvlog:derive(parent_cv_id, opts) → new_cv_id string
  --
  -- Creates a new CV descended from an existing one.  Use this when:
  --   - One entity is split into multiple outputs (destructuring, splitting)
  --   - A transformation produces a "new" entity that descended from the old one
  --
  -- The derived CV's ID is the parent ID with a new numeric suffix:
  --   parent_cv_id = "a3f1.1"
  --   derived      = "a3f1.1.2"  (if 2 is the next counter value)
  --
  -- opts (optional):
  --   source (string) — the stage that performed the derivation
  --   tag    (string) — classification of the derivation
  --   meta   (table)  — extra context
  --
  -- Errors:
  --   Parent not found — parent_cv_id was never created
  --   Parent deleted   — cannot derive from a deleted entity
  function self:derive(parent_cv_id, opts)
    opts = opts or {}

    self._counter = self._counter + 1
    local n = self._counter - 1

    local new_cv_id = parent_cv_id .. "." .. n

    if not self._enabled then
      return new_cv_id
    end

    local parent = self._entries[parent_cv_id]
    if not parent then
      error("Parent CV not found: " .. tostring(parent_cv_id))
    end
    if parent.deleted then
      error("Cannot derive from deleted CV: " .. tostring(parent_cv_id))
    end

    -- Create the derived entry with its parent link.
    self._entries[new_cv_id] = {
      cv_id           = new_cv_id,
      origin          = nil,
      parent_cv_id    = parent_cv_id,
      merged_from     = {},
      contributions   = {},
      deleted         = nil,
      pass_order      = {},
      _pass_order_set = {},
    }

    -- If the caller also wants to record an initial contribution from the
    -- deriving stage, append it now.
    if opts.source then
      local contribution = {
        source    = opts.source,
        tag       = opts.tag or "derived",
        meta      = opts.meta or {},
        timestamp = now_iso(),
      }
      local entry = self._entries[new_cv_id]
      entry.contributions[#entry.contributions + 1] = contribution
      dedup_append(entry.pass_order, entry._pass_order_set, opts.source)
      dedup_append(self._pass_order, self._pass_order_set, opts.source)
    end

    return new_cv_id
  end

  -- cvlog:merge(cv_ids, opts) → merged_cv_id string
  --
  -- Creates a new CV descended from multiple existing CVs.  Use this when
  -- multiple entities are combined into one output:
  --   - Inlining a function (call site + body → merged expression)
  --   - Joining two database tables into one result row
  --   - Concatenating multiple source files into a bundle
  --
  -- The merged CV's base is determined by hashing the sorted parent IDs.
  -- Sorting ensures that merge(["a", "b"]) and merge(["b", "a"]) produce
  -- the same base — merge is commutative.
  --
  -- cv_ids (table/array of strings) — the parent CV IDs to merge
  -- opts (optional):
  --   source, tag, meta — initial contribution from the merging stage
  function self:merge(cv_ids, opts)
    opts = opts or {}

    -- Sort the parent IDs for deterministic base computation.
    -- This is critical: the same set of parents must always produce the same
    -- merged ID, regardless of the order they were passed in.
    local sorted = {}
    for i, id in ipairs(cv_ids) do sorted[i] = id end
    table.sort(sorted)

    local hash_input = table.concat(sorted, ",")
    local base       = sha256.sha256_hex(hash_input):sub(1, 8)

    self._counter = self._counter + 1
    local n = self._counter - 1

    local merged_cv_id = base .. "." .. n

    if not self._enabled then
      return merged_cv_id
    end

    -- Validate all parents exist (deleted parents are allowed by spec — you
    -- might merge a tombstone with an active entity to produce a "merge of
    -- deletion" record).
    for _, pid in ipairs(cv_ids) do
      if not self._entries[pid] then
        error("Merge parent CV not found: " .. tostring(pid))
      end
    end

    -- Create the merged entry.  merged_from stores the ORIGINAL order
    -- (not sorted) so callers can see exactly what was passed.
    self._entries[merged_cv_id] = {
      cv_id           = merged_cv_id,
      origin          = nil,
      parent_cv_id    = nil,
      merged_from     = cv_ids,
      contributions   = {},
      deleted         = nil,
      pass_order      = {},
      _pass_order_set = {},
    }

    -- Optionally record an initial contribution from the merging stage.
    if opts.source then
      local contribution = {
        source    = opts.source,
        tag       = opts.tag or "merged",
        meta      = opts.meta or {},
        timestamp = now_iso(),
      }
      local entry = self._entries[merged_cv_id]
      entry.contributions[#entry.contributions + 1] = contribution
      dedup_append(entry.pass_order, entry._pass_order_set, opts.source)
      dedup_append(self._pass_order, self._pass_order_set, opts.source)
    end

    return merged_cv_id
  end

  -- cvlog:delete(cv_id, opts) → nil
  --
  -- Records that an entity was intentionally removed.  The CV entry remains
  -- in the log permanently — this is how you answer "why did this disappear?"
  -- long after the fact.
  --
  -- The deletion record contains who deleted it and when, but not a reason
  -- in this implementation (the reason can go in opts.meta).  The spec's
  -- CVEntry has a deleted field; we set it to a table with `by` and `at`.
  --
  -- After deletion:
  --   contribute(deleted_cv_id) → error
  --   passthrough(deleted_cv_id) → error
  --   derive(deleted_cv_id) → allowed (spec says "derive or merge with a
  --     deleted CV as a parent is allowed")
  --   get(deleted_cv_id) → returns the entry (with deleted field set)
  --
  -- opts:
  --   by (string) — identifier of who/what performed the deletion
  function self:delete(cv_id, opts)
    if not self._enabled then return nil end

    local entry = self._entries[cv_id]
    if not entry then
      error("CV not found: " .. tostring(cv_id))
    end

    opts = opts or {}
    entry.deleted = {
      by = opts.by or "unknown",
      at = now_iso(),
    }

    return nil
  end

  -- cvlog:passthrough(cv_id, opts) → cv_id string
  --
  -- Records that a stage examined this entity but made no changes.  This is
  -- the identity contribution — useful for reconstructing which stages an
  -- entity passed through even when nothing was transformed.
  --
  -- Example: a type-checker that verifies but does not modify a variable.
  -- Without passthrough, the type-checker is invisible in the history for
  -- that variable.
  --
  -- Returns the cv_id unchanged — this allows passthrough to be used in
  -- pipelines where you chain calls: cv_id = cvlog:passthrough(cv_id, ...).
  --
  -- opts:
  --   source (string) — the stage that passed through
  function self:passthrough(cv_id, opts)
    if not self._enabled then return cv_id end

    local entry = self._entries[cv_id]
    if not entry then
      error("CV not found: " .. tostring(cv_id))
    end
    if entry.deleted then
      error("Cannot passthrough deleted CV: " .. tostring(cv_id))
    end

    opts = opts or {}
    if opts.source then
      dedup_append(entry.pass_order, entry._pass_order_set, opts.source)
      dedup_append(self._pass_order, self._pass_order_set, opts.source)
    end

    return cv_id
  end

  -- -----------------------------------------------------------------------
  -- Query operations
  -- -----------------------------------------------------------------------

  -- cvlog:get(cv_id) → entry table or nil
  --
  -- Returns the full internal entry for a CV ID, or nil if not found.
  -- The entry table contains: cv_id, origin, parent_cv_id, merged_from,
  -- contributions, deleted, pass_order.
  function self:get(cv_id)
    return self._entries[cv_id]
  end

  -- cvlog:ancestors(cv_id) → array of cv_ids, nearest-first
  --
  -- Walks the parent chain recursively and returns all ancestor CV IDs,
  -- ordered from immediate parent to most distant ancestor.
  --
  -- Uses breadth-first search (BFS) rather than depth-first to produce a
  -- "nearest first" ordering:
  --
  --   Given: D → C → B → A (each derived from the previous)
  --   ancestors(D) = { C, B, A }
  --
  -- For merged CVs, all parents are visited at the same BFS level, then
  -- their parents, and so on.  Cycles are impossible by construction (a CV
  -- cannot be its own ancestor) but we guard against them anyway.
  function self:ancestors(cv_id)
    local result  = {}
    local visited = {}    -- guard against impossible cycles
    local queue   = {}    -- BFS queue

    -- Seed the queue with the direct parents of cv_id.
    local entry = self._entries[cv_id]
    if not entry then return result end

    -- A CV can have parents from two sources:
    --   parent_cv_id  — single parent (for derive)
    --   merged_from   — multiple parents (for merge)
    -- We add both types to the initial queue.
    if entry.parent_cv_id then
      queue[#queue + 1] = entry.parent_cv_id
    end
    for _, pid in ipairs(entry.merged_from) do
      -- Avoid duplicates if the same ID appears in both fields.
      if not visited[pid] then
        queue[#queue + 1] = pid
      end
    end

    local head = 1  -- index of the next item to dequeue
    while head <= #queue do
      local current_id = queue[head]
      head = head + 1

      -- Skip if already visited (protects against impossible cycles and
      -- diamond-shaped DAGs where one ancestor appears via multiple paths).
      if not visited[current_id] then
        visited[current_id] = true
        result[#result + 1] = current_id

        local current = self._entries[current_id]
        if current then
          if current.parent_cv_id and not visited[current.parent_cv_id] then
            queue[#queue + 1] = current.parent_cv_id
          end
          for _, pid in ipairs(current.merged_from) do
            if not visited[pid] then
              queue[#queue + 1] = pid
            end
          end
        end
      end
    end

    return result
  end

  -- cvlog:descendants(cv_id) → array of cv_ids
  --
  -- Returns all CV IDs that have this CV ID in their ancestor chain.
  -- This is the inverse of `ancestors` — it scans the entire log.
  --
  -- Implementation note: we do a single pass over all entries and collect
  -- those whose parent_cv_id or merged_from includes cv_id, then recurse.
  -- For large logs, this is O(entries × depth); a production system would
  -- maintain an inverted index.
  function self:descendants(cv_id)
    local result  = {}
    local visited = {}

    -- BFS: start with direct children of cv_id.
    local queue = {}

    -- Build a reverse-index on the fly: for each entry, check if cv_id is
    -- a direct parent.
    local function find_direct_children(parent_id)
      local children = {}
      for id, entry in pairs(self._entries) do
        if entry.parent_cv_id == parent_id then
          children[#children + 1] = id
        else
          for _, pid in ipairs(entry.merged_from) do
            if pid == parent_id then
              children[#children + 1] = id
              break
            end
          end
        end
      end
      return children
    end

    -- Seed queue with direct children.
    local direct = find_direct_children(cv_id)
    for _, child_id in ipairs(direct) do
      queue[#queue + 1] = child_id
    end

    local head = 1
    while head <= #queue do
      local current_id = queue[head]
      head = head + 1

      if not visited[current_id] then
        visited[current_id] = true
        result[#result + 1] = current_id

        -- Find children of the current node and enqueue them.
        local children = find_direct_children(current_id)
        for _, child_id in ipairs(children) do
          if not visited[child_id] then
            queue[#queue + 1] = child_id
          end
        end
      end
    end

    return result
  end

  -- cvlog:history(cv_id) → contributions array (or empty table)
  --
  -- Returns the list of contributions for a CV ID in insertion order.
  -- Each contribution is a table with: source, tag, meta, timestamp.
  --
  -- Returns an empty table if the CV is not found (rather than erroring),
  -- because querying history for a disabled-log CV should gracefully return
  -- empty.
  function self:history(cv_id)
    local entry = self._entries[cv_id]
    if not entry then return {} end
    return entry.contributions
  end

  -- cvlog:lineage(cv_id) → array of entries, oldest ancestor first
  --
  -- Returns the full CV entries for the entity and all its ancestors,
  -- ordered from oldest ancestor to the entity itself.
  --
  -- This is the "complete provenance chain" — you see the full genealogy
  -- from the origin of the data to its current form.
  --
  -- Example: A → B → C → D
  --   lineage(D) = { entry(A), entry(B), entry(C), entry(D) }
  --
  -- Implementation: call ancestors (which returns nearest-first), reverse
  -- the result, then append the entry for cv_id itself.
  function self:lineage(cv_id)
    local ancestor_ids = self:ancestors(cv_id)

    -- Reverse the ancestors array so that oldest is first.
    local n = #ancestor_ids
    for i = 1, math.floor(n / 2) do
      local j = n - i + 1
      ancestor_ids[i], ancestor_ids[j] = ancestor_ids[j], ancestor_ids[i]
    end

    local result = {}
    for _, aid in ipairs(ancestor_ids) do
      local entry = self._entries[aid]
      if entry then
        result[#result + 1] = entry
      end
    end

    -- Append the entry for the requested cv_id itself.
    local own_entry = self._entries[cv_id]
    if own_entry then
      result[#result + 1] = own_entry
    end

    return result
  end

  -- -----------------------------------------------------------------------
  -- Serialization
  -- -----------------------------------------------------------------------

  -- cvlog:serialize() → JSON string
  --
  -- Converts the entire CVLog to a JSON string suitable for storage or
  -- cross-process / cross-language transmission.
  --
  -- The JSON structure follows the spec's CVLog JSON format:
  --   {
  --     "entries": { "<cv_id>": { ... }, ... },
  --     "pass_order": [ "parser", "scope_analysis", ... ],
  --     "enabled": true
  --   }
  --
  -- Private fields (those with leading _ in the entry) are NOT included
  -- in the serialized form — they are implementation details.
  function self:serialize()
    local entries_tbl = {}

    for cv_id, entry in pairs(self._entries) do
      -- Build a clean, serializable version of the entry.
      -- We skip _pass_order_set (private implementation detail) and include
      -- only the public fields.
      local clean_entry = {
        cv_id         = entry.cv_id,
        origin        = entry.origin,
        parent_cv_id  = entry.parent_cv_id,
        merged_from   = entry.merged_from,
        contributions = entry.contributions,
        deleted       = entry.deleted,
        pass_order    = entry.pass_order,
      }
      entries_tbl[cv_id] = clean_entry
    end

    local log_tbl = {
      entries    = entries_tbl,
      pass_order = self._pass_order,
      enabled    = self._enabled,
    }

    return json.encode(log_tbl)
  end

  return self
end

-- ---------------------------------------------------------------------------
-- Deserialization (static factory)
-- ---------------------------------------------------------------------------

-- M.deserialize(json_str) → cvlog object
--
-- Reconstructs a CVLog from a JSON string previously produced by
-- cvlog:serialize().
--
-- This is a static method on the module (not on instances) because it
-- creates a new instance rather than updating an existing one.
--
-- The reconstructed log is fully functional — you can continue calling
-- contribute, derive, merge, etc. on it.
function M.deserialize(json_str)
  local data = json.decode(json_str)

  -- Create a new CVLog with the same enabled setting.
  -- We use enabled=true during reconstruction so that operations below
  -- can write to the entries table, then set _enabled to the deserialized
  -- value at the end.
  local cvlog = M.new({ enabled = true })

  -- Restore the counter.  We need the counter to be higher than the highest
  -- numeric suffix in any cv_id, otherwise future creates would collide.
  -- The simplest approach: scan all entries and find the max.
  local max_counter = 0

  if data.entries then
    for cv_id, entry_data in pairs(data.entries) do
      -- Reconstruct the entry with all public fields.
      local entry = {
        cv_id           = entry_data.cv_id or cv_id,
        origin          = entry_data.origin,
        parent_cv_id    = entry_data.parent_cv_id,
        merged_from     = entry_data.merged_from or {},
        contributions   = entry_data.contributions or {},
        deleted         = entry_data.deleted,
        pass_order      = entry_data.pass_order or {},
        _pass_order_set = {},  -- rebuild the private set
      }

      -- Rebuild the _pass_order_set for this entry.
      for _, source in ipairs(entry.pass_order) do
        entry._pass_order_set[source] = true
      end

      cvlog._entries[cv_id] = entry

      -- Find the max numeric suffix across all parts of the cv_id.
      -- e.g. "a3f1.5.12" → check 5 and 12, find max 12.
      for num_str in cv_id:gmatch("%.(%d+)") do
        local num = tonumber(num_str)
        if num and num > max_counter then
          max_counter = num
        end
      end
    end
  end

  -- Restore the global pass_order.
  if data.pass_order then
    for _, source in ipairs(data.pass_order) do
      cvlog._pass_order[#cvlog._pass_order + 1] = source
      cvlog._pass_order_set[source] = true
    end
  end

  -- Set the counter to one past the highest observed suffix, so the next
  -- allocated ID does not collide with any existing ID.
  cvlog._counter = max_counter + 1

  -- Finally, apply the serialized enabled flag.
  if data.enabled ~= nil then
    cvlog._enabled = data.enabled
  end

  return cvlog
end

return M
