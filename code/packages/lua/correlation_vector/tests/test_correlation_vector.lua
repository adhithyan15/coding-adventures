-- test_correlation_vector.lua
-- ===========================
--
-- Comprehensive tests for the correlation_vector Lua package, using the
-- busted test framework.  busted follows the same "describe / it" style
-- as RSpec (Ruby) and Jasmine (JavaScript).
--
-- Test groups:
--   1. root lifecycle     — create, contribute, passthrough, delete
--   2. derivation         — parent/child relationships, ancestors, descendants
--   3. merging            — multi-parent CVs, ancestors across merge
--   4. deep ancestry      — A→B→C→D chain, lineage ordering
--   5. disabled log       — enabled=false, IDs still allocated, nothing stored
--   6. serialization      — roundtrip serialize/deserialize fidelity
--   7. id uniqueness      — 1000 CVs, no collisions

-- ---------------------------------------------------------------------------
-- Package path setup
-- ---------------------------------------------------------------------------
-- We prepend the source directories of this package and all its transitive
-- dependencies so that busted loads from source (not the installed .luarocks
-- copies).  This is required because json_lexer navigates 6 directory levels
-- up from its own source file to locate the shared grammars/ directory.
-- When loaded from source (code/packages/lua/json_lexer/src/...), that path
-- resolves correctly.  When loaded from the installed location (~/.luarocks),
-- it would resolve to ~/grammars/ which does not exist.
package.path = (
    "../src/?.lua;"                                           ..
    "../src/?/init.lua;"                                      ..
    "../../sha256/src/?.lua;"                                 ..
    "../../sha256/src/?/init.lua;"                            ..
    "../../json_value/src/?.lua;"                             ..
    "../../json_value/src/?/init.lua;"                        ..
    "../../json_parser/src/?.lua;"                            ..
    "../../json_parser/src/?/init.lua;"                       ..
    "../../grammar_tools/src/?.lua;"                          ..
    "../../grammar_tools/src/?/init.lua;"                     ..
    "../../lexer/src/?.lua;"                                  ..
    "../../lexer/src/?/init.lua;"                             ..
    "../../state_machine/src/?.lua;"                          ..
    "../../state_machine/src/?/init.lua;"                     ..
    "../../directed_graph/src/?.lua;"                         ..
    "../../directed_graph/src/?/init.lua;"                    ..
    "../../json_lexer/src/?.lua;"                             ..
    "../../json_lexer/src/?/init.lua;"                        ..
    "../../parser/src/?.lua;"                                 ..
    "../../parser/src/?/init.lua;"                            ..
    "../../json_serializer/src/?.lua;"                        ..
    "../../json_serializer/src/?/init.lua;"                   ..
    package.path
)

local cv_module = require("coding_adventures.correlation_vector")

-- Helper: count the keys in a table (Lua has no built-in table.size).
local function table_size(t)
  local count = 0
  for _ in pairs(t) do count = count + 1 end
  return count
end

describe("CorrelationVector", function()

  -- -----------------------------------------------------------------------
  -- 1. Root lifecycle
  -- -----------------------------------------------------------------------

  describe("root lifecycle", function()

    it("creates a root CV with an origin and correct ID format", function()
      local log    = cv_module.new()
      local cv_id  = log:create({ origin_string = "app.ts:5:12" })

      -- The ID must match base.N where base is 8 hex chars and N is an integer.
      assert.is_not_nil(cv_id)
      assert.matches("^%x%x%x%x%x%x%x%x%.%d+$", cv_id)

      -- The entry must be stored in the log.
      local entry = log:get(cv_id)
      assert.is_not_nil(entry)
      assert.are.equal(cv_id, entry.cv_id)
    end)

    it("creates a synthetic root CV with base 00000000", function()
      local log   = cv_module.new()
      local cv_id = log:create({ synthetic = true })

      -- Synthetic IDs must always start with "00000000."
      assert.matches("^00000000%.%d+$", cv_id)
    end)

    it("contribute appends to history", function()
      local log   = cv_module.new()
      local cv_id = log:create({ origin_string = "source.ts" })

      log:contribute(cv_id, { source = "scope_analysis", tag = "resolved",
                               meta = { binding = "local:x" } })

      local hist = log:history(cv_id)
      assert.are.equal(1, #hist)
      assert.are.equal("scope_analysis", hist[1].source)
      assert.are.equal("resolved",       hist[1].tag)
      assert.are.equal("local:x",        hist[1].meta.binding)
    end)

    it("passthrough records source in pass_order and returns same cv_id", function()
      local log   = cv_module.new()
      local cv_id = log:create({ origin_string = "file.ts" })

      local returned = log:passthrough(cv_id, { source = "type_checker" })

      -- passthrough must return the same cv_id unchanged.
      assert.are.equal(cv_id, returned)

      local entry = log:get(cv_id)
      assert.are.equal(1, #entry.pass_order)
      assert.are.equal("type_checker", entry.pass_order[1])
    end)

    it("delete marks the entry and blocks further contributions", function()
      local log   = cv_module.new()
      local cv_id = log:create({ origin_string = "dead_code.ts" })

      log:delete(cv_id, { by = "dce" })

      local entry = log:get(cv_id)
      assert.is_not_nil(entry.deleted)
      assert.are.equal("dce", entry.deleted.by)

      -- Attempting to contribute to a deleted CV must raise an error.
      assert.has_error(function()
        log:contribute(cv_id, { source = "anything", tag = "too_late" })
      end)
    end)

    it("passthrough on a deleted CV raises an error", function()
      local log   = cv_module.new()
      local cv_id = log:create()
      log:delete(cv_id, { by = "cleaner" })

      assert.has_error(function()
        log:passthrough(cv_id, { source = "inspector" })
      end)
    end)

    it("contribute to unknown CV raises error", function()
      local log = cv_module.new()
      assert.has_error(function()
        log:contribute("nonexistent.99", { source = "x", tag = "y" })
      end)
    end)

    it("multiple contributions accumulate in order", function()
      local log   = cv_module.new()
      local cv_id = log:create({ origin_string = "pipeline.ts" })

      log:contribute(cv_id, { source = "parser",         tag = "created" })
      log:contribute(cv_id, { source = "scope_analysis", tag = "resolved" })
      log:contribute(cv_id, { source = "renamer",        tag = "renamed" })

      local hist = log:history(cv_id)
      assert.are.equal(3, #hist)
      assert.are.equal("parser",         hist[1].source)
      assert.are.equal("scope_analysis", hist[2].source)
      assert.are.equal("renamer",        hist[3].source)
    end)

    it("global pass_order is deduplicated across multiple CVs", function()
      local log  = cv_module.new()
      local cv1  = log:create()
      local cv2  = log:create()

      log:contribute(cv1, { source = "stage_a", tag = "foo" })
      log:contribute(cv2, { source = "stage_a", tag = "bar" })  -- duplicate source
      log:contribute(cv2, { source = "stage_b", tag = "baz" })

      -- Global pass_order should only list stage_a once.
      assert.are.equal(2, #log._pass_order)
      assert.are.equal("stage_a", log._pass_order[1])
      assert.are.equal("stage_b", log._pass_order[2])
    end)

  end)

  -- -----------------------------------------------------------------------
  -- 2. Derivation
  -- -----------------------------------------------------------------------

  describe("derivation", function()

    it("derives two children from one parent, both IDs have parent prefix", function()
      local log    = cv_module.new()
      local parent = log:create({ origin_string = "parent_file.ts" })
      local child1 = log:derive(parent)
      local child2 = log:derive(parent)

      -- Each child's ID must start with the parent's ID followed by a dot.
      assert.matches("^" .. parent:gsub("%.", "%%.") .. "%.%d+$", child1)
      assert.matches("^" .. parent:gsub("%.", "%%.") .. "%.%d+$", child2)

      -- The two children must be different.
      assert.are_not.equal(child1, child2)
    end)

    it("ancestors(child) returns [parent_cv_id]", function()
      local log    = cv_module.new()
      local parent = log:create({ origin_string = "src.ts" })
      local child  = log:derive(parent)

      local ancs = log:ancestors(child)
      assert.are.equal(1, #ancs)
      assert.are.equal(parent, ancs[1])
    end)

    it("descendants(parent) returns both child IDs", function()
      local log    = cv_module.new()
      local parent = log:create({ origin_string = "src.ts" })
      local child1 = log:derive(parent)
      local child2 = log:derive(parent)

      local descs = log:descendants(parent)
      assert.are.equal(2, #descs)

      -- Both children must be present (order not guaranteed).
      local desc_set = {}
      for _, d in ipairs(descs) do desc_set[d] = true end
      assert.is_true(desc_set[child1])
      assert.is_true(desc_set[child2])
    end)

    it("derive with source records initial contribution", function()
      local log    = cv_module.new()
      local parent = log:create()
      local child  = log:derive(parent, { source = "splitter", tag = "split_left" })

      local hist = log:history(child)
      assert.are.equal(1, #hist)
      assert.are.equal("splitter",   hist[1].source)
      assert.are.equal("split_left", hist[1].tag)
    end)

    it("derive from deleted parent raises error", function()
      local log    = cv_module.new()
      local parent = log:create()
      log:delete(parent, { by = "cleaner" })

      assert.has_error(function()
        log:derive(parent)
      end)
    end)

    it("derive from unknown parent raises error", function()
      local log = cv_module.new()
      assert.has_error(function()
        log:derive("ghost.0")
      end)
    end)

  end)

  -- -----------------------------------------------------------------------
  -- 3. Merging
  -- -----------------------------------------------------------------------

  describe("merging", function()

    it("merges three CVs into one, parent_ids lists all three", function()
      local log = cv_module.new()
      local a   = log:create({ origin_string = "a.ts" })
      local b   = log:create({ origin_string = "b.ts" })
      local c   = log:create({ origin_string = "c.ts" })

      local merged = log:merge({ a, b, c })

      local entry = log:get(merged)
      assert.is_not_nil(entry)
      assert.are.equal(3, #entry.merged_from)

      -- All three parents must be listed.
      local pset = {}
      for _, pid in ipairs(entry.merged_from) do pset[pid] = true end
      assert.is_true(pset[a])
      assert.is_true(pset[b])
      assert.is_true(pset[c])
    end)

    it("ancestors(merged) returns all three parents", function()
      local log = cv_module.new()
      local a   = log:create({ origin_string = "a.ts" })
      local b   = log:create({ origin_string = "b.ts" })
      local c   = log:create({ origin_string = "c.ts" })

      local merged = log:merge({ a, b, c })
      local ancs   = log:ancestors(merged)

      -- There should be exactly 3 ancestors (the three parents).
      -- They are all roots so their own ancestor lists are empty.
      assert.are.equal(3, #ancs)
      local aset = {}
      for _, aid in ipairs(ancs) do aset[aid] = true end
      assert.is_true(aset[a])
      assert.is_true(aset[b])
      assert.is_true(aset[c])
    end)

    it("merge is commutative — same parents in different order produce same base", function()
      local log = cv_module.new()
      local a   = log:create({ origin_string = "x.ts" })
      local b   = log:create({ origin_string = "y.ts" })

      local m1  = log:merge({ a, b })
      local m2  = log:merge({ b, a })

      -- The base segments (before the last dot) must be identical.
      local base1 = m1:match("^(.-)%.[^%.]+$")
      local base2 = m2:match("^(.-)%.[^%.]+$")
      assert.are.equal(base1, base2)
    end)

    it("merge with unknown parent raises error", function()
      local log = cv_module.new()
      local a   = log:create()
      assert.has_error(function()
        log:merge({ a, "ghost.99" })
      end)
    end)

    it("merge with source records initial contribution", function()
      local log    = cv_module.new()
      local a      = log:create()
      local b      = log:create()
      local merged = log:merge({ a, b }, { source = "joiner", tag = "joined" })

      local hist = log:history(merged)
      assert.are.equal(1, #hist)
      assert.are.equal("joiner", hist[1].source)
      assert.are.equal("joined", hist[1].tag)
    end)

  end)

  -- -----------------------------------------------------------------------
  -- 4. Deep ancestry chain: A → B → C → D
  -- -----------------------------------------------------------------------

  describe("deep ancestry", function()

    -- Build a four-level chain: A is the root, B derives from A,
    -- C derives from B, D derives from C.
    local function build_chain()
      local log = cv_module.new()
      local a   = log:create({ origin_string = "root.ts" })
      local b   = log:derive(a)
      local c   = log:derive(b)
      local d   = log:derive(c)
      return log, a, b, c, d
    end

    it("ancestors(D) returns [C, B, A] — nearest first", function()
      local log, a, b, c, d = build_chain()
      local ancs = log:ancestors(d)

      assert.are.equal(3, #ancs)
      assert.are.equal(c, ancs[1])  -- immediate parent (nearest)
      assert.are.equal(b, ancs[2])
      assert.are.equal(a, ancs[3])  -- most distant ancestor
    end)

    it("lineage(D) returns entries for [A, B, C, D] — oldest first", function()
      local log, a, b, c, d = build_chain()
      local lin = log:lineage(d)

      assert.are.equal(4, #lin)
      assert.are.equal(a, lin[1].cv_id)
      assert.are.equal(b, lin[2].cv_id)
      assert.are.equal(c, lin[3].cv_id)
      assert.are.equal(d, lin[4].cv_id)
    end)

    it("ancestors(A) is empty — root has no parents", function()
      local log, a = build_chain()
      local ancs   = log:ancestors(a)
      assert.are.equal(0, #ancs)
    end)

    it("descendants(A) returns B, C, D", function()
      local log, a, b, c, d = build_chain()
      local descs = log:descendants(a)

      assert.are.equal(3, #descs)
      local dset = {}
      for _, id in ipairs(descs) do dset[id] = true end
      assert.is_true(dset[b])
      assert.is_true(dset[c])
      assert.is_true(dset[d])
    end)

    it("lineage on a root returns just that entry", function()
      local log, a = build_chain()
      local lin    = log:lineage(a)
      assert.are.equal(1, #lin)
      assert.are.equal(a, lin[1].cv_id)
    end)

  end)

  -- -----------------------------------------------------------------------
  -- 5. Disabled log
  -- -----------------------------------------------------------------------

  describe("disabled log", function()

    it("all operations complete without error", function()
      local log = cv_module.new({ enabled = false })

      -- These must not raise.
      local cv1    = log:create({ origin_string = "file.ts" })
      local cv2    = log:create({ synthetic = true })
      local child  = log:derive(cv1)
      local merged = log:merge({ cv1, cv2 })
      log:delete(cv1, { by = "dce" })

      -- passthrough and contribute should also be no-ops, not errors.
      log:passthrough(cv2, { source = "checker" })
      log:contribute(cv2, { source = "anything", tag = "ignored" })

      -- All four IDs must have been returned (non-nil strings).
      assert.is_not_nil(cv1)
      assert.is_not_nil(cv2)
      assert.is_not_nil(child)
      assert.is_not_nil(merged)
    end)

    it("all CV IDs are still allocated and unique", function()
      local log = cv_module.new({ enabled = false })
      local ids = {}
      for i = 1, 20 do
        ids[i] = log:create({ origin_string = "file" .. i .. ".ts" })
      end

      -- Verify uniqueness.
      local seen = {}
      for _, id in ipairs(ids) do
        assert.is_nil(seen[id], "Duplicate ID: " .. id)
        seen[id] = true
      end
    end)

    it("get(cv_id) returns nil — nothing was stored", function()
      local log   = cv_module.new({ enabled = false })
      local cv_id = log:create({ origin_string = "ghost.ts" })
      assert.is_nil(log:get(cv_id))
    end)

    it("history(cv_id) returns empty list", function()
      local log   = cv_module.new({ enabled = false })
      local cv_id = log:create()
      local hist  = log:history(cv_id)
      assert.are.equal(0, #hist)
    end)

    it("derive still returns id with parent prefix", function()
      local log    = cv_module.new({ enabled = false })
      local parent = log:create({ origin_string = "p.ts" })
      local child  = log:derive(parent)

      -- Even when disabled, derive must still append to the parent ID.
      assert.matches("^" .. parent:gsub("%.", "%%.") .. "%.%d+$", child)
    end)

  end)

  -- -----------------------------------------------------------------------
  -- 6. Serialization roundtrip
  -- -----------------------------------------------------------------------

  describe("serialization roundtrip", function()

    it("roundtrips a log with roots, derivations, merges, deletions", function()
      local log = cv_module.new()

      -- Build a varied log.
      local root1  = log:create({ origin_string = "file_a.ts" })
      local root2  = log:create({ origin_string = "file_b.ts" })
      local child1 = log:derive(root1, { source = "splitter", tag = "split" })
      local child2 = log:derive(root1)
      local merged = log:merge({ root2, child1 }, { source = "joiner", tag = "join" })

      log:contribute(root1,  { source = "parser",   tag = "created" })
      log:contribute(child2, { source = "analyzer",  tag = "analyzed" })
      log:delete(child2, { by = "dce" })
      log:passthrough(root2, { source = "checker" })

      -- Serialize to JSON string.
      local json_str = log:serialize()
      assert.is_string(json_str)
      assert.is_true(#json_str > 10)

      -- Deserialize back.
      local log2 = cv_module.deserialize(json_str)

      -- Verify all entries are present.
      assert.is_not_nil(log2:get(root1))
      assert.is_not_nil(log2:get(root2))
      assert.is_not_nil(log2:get(child1))
      assert.is_not_nil(log2:get(child2))
      assert.is_not_nil(log2:get(merged))

      -- Verify history is preserved.
      local hist1 = log2:history(root1)
      assert.are.equal(1, #hist1)
      assert.are.equal("parser", hist1[1].source)

      local hist2 = log2:history(child2)
      assert.are.equal(1, #hist2)
      assert.are.equal("analyzer", hist2[1].source)

      -- Verify deletion is preserved.
      local e_child2 = log2:get(child2)
      assert.is_not_nil(e_child2.deleted)
      assert.are.equal("dce", e_child2.deleted.by)

      -- Verify merge ancestry.
      local ancs = log2:ancestors(merged)
      local aset = {}
      for _, aid in ipairs(ancs) do aset[aid] = true end
      assert.is_true(aset[root2])
      assert.is_true(aset[child1])

      -- Verify derivation parent links.
      local e_child1 = log2:get(child1)
      assert.are.equal(root1, e_child1.parent_cv_id)
    end)

    it("deserialized log can continue to create new unique IDs", function()
      local log   = cv_module.new()
      local root  = log:create({ origin_string = "base.ts" })
      log:contribute(root, { source = "s", tag = "t" })

      local log2   = cv_module.deserialize(log:serialize())
      local new_id = log2:create({ origin_string = "new.ts" })

      -- The new ID must not collide with the existing root.
      assert.are_not.equal(root, new_id)
      assert.is_not_nil(log2:get(new_id))
    end)

    it("serialize/deserialize preserves enabled flag", function()
      local log  = cv_module.new({ enabled = true })
      log:create()
      local log2 = cv_module.deserialize(log:serialize())
      assert.is_true(log2._enabled)
    end)

    it("serialize/deserialize preserves pass_order", function()
      local log   = cv_module.new()
      local cv_id = log:create()
      log:contribute(cv_id, { source = "stage_a", tag = "x" })
      log:contribute(cv_id, { source = "stage_b", tag = "y" })

      local log2 = cv_module.deserialize(log:serialize())
      assert.are.equal(2, #log2._pass_order)
      assert.are.equal("stage_a", log2._pass_order[1])
      assert.are.equal("stage_b", log2._pass_order[2])
    end)

  end)

  -- -----------------------------------------------------------------------
  -- 7. ID uniqueness
  -- -----------------------------------------------------------------------

  describe("id uniqueness", function()

    it("creates 1000 CVs with same origin and all IDs are unique", function()
      local log = cv_module.new()
      local ids = {}

      for _ = 1, 1000 do
        local cv_id = log:create({ origin_string = "same_file.ts" })
        ids[cv_id] = (ids[cv_id] or 0) + 1
      end

      -- Count unique IDs.
      local unique_count = table_size(ids)
      assert.are.equal(1000, unique_count)

      -- Verify no ID was allocated more than once.
      for id, count in pairs(ids) do
        assert.are.equal(1, count, "Duplicate ID found: " .. id)
      end
    end)

    it("creates 500 CVs mixing origins and all IDs are unique", function()
      local log = cv_module.new()
      local ids = {}

      for i = 1, 500 do
        -- Alternate between different origins so bases differ.
        local origin = "file_" .. (i % 5) .. ".ts"
        local cv_id  = log:create({ origin_string = origin })
        ids[cv_id] = (ids[cv_id] or 0) + 1
      end

      local unique_count = table_size(ids)
      assert.are.equal(500, unique_count)
    end)

    it("derive IDs are unique across many derivations", function()
      local log  = cv_module.new()
      local root = log:create({ origin_string = "tree.ts" })
      local ids  = { [root] = true }

      local function derive_many(parent, depth)
        if depth == 0 then return end
        local child = log:derive(parent)
        assert.is_nil(ids[child], "Duplicate derived ID: " .. child)
        ids[child] = true
        derive_many(child, depth - 1)
      end

      -- Create a chain of 100 derivations.
      for _ = 1, 10 do
        derive_many(root, 10)
      end

      -- All IDs (root + 100 derived) must be unique.
      assert.is_true(table_size(ids) >= 101)
    end)

  end)

end)
