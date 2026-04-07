# frozen_string_literal: true

# ================================================================
# CodingAdventures::CorrelationVector -- Main Implementation
# ================================================================
#
# A Correlation Vector (CV) is an append-only provenance record that
# follows a piece of data through every transformation it undergoes.
#
# The Big Picture
# ===============
# Imagine you have a source code file being compiled. You want to
# know: for this variable in the output, which line of source code
# did it come from? And which compiler passes touched it along the
# way? A CV answers these questions.
#
# The same idea works for:
#   - ETL pipelines: where did this record come from, and who cleaned it?
#   - Build systems: which source file produced this object file?
#   - Neural networks: which training example influenced this weight?
#   - Distributed systems: which microservice produced this data?
#
# Core Data Flow
# ==============
#
#   Entity born ──── create ────► cv_id assigned
#       │
#       ├── contribute ──────────► "parser processed me"
#       ├── contribute ──────────► "scope_analysis resolved me"
#       ├── passthrough ─────────► "type_checker saw me (no change)"
#       │
#       ├── derive ──────────────► child_cv_id (split output)
#       ├── merge([a, b]) ───────► merged_cv_id (combined output)
#       │
#       └── delete ──────────────► permanently marked deleted
#
# The CV log accumulates all of this history. At any point you can
# ask "what happened to entity X?" and get a complete answer.
#
# ID Format
# =========
# IDs use a dot-extension scheme that encodes parentage visually:
#
#   a3f1b2c4.1     -- root entity (base=hash of origin, N=1)
#   a3f1b2c4.2     -- second root entity with same origin hash
#   a3f1b2c4.1.1   -- first entity derived from a3f1b2c4.1
#   a3f1b2c4.1.2   -- second entity derived from a3f1b2c4.1
#   00000000.1     -- synthetic entity (no natural origin)
#
# You can read parentage directly from the ID: count the dots,
# subtract 1, and you know the depth of derivation.
#
# Threading the pass_order
# ========================
# The CVLog maintains a `pass_order` array: the ordered, deduplicated
# list of all source names (stages) that have contributed to any CV
# in this log. This gives you a "pipeline topology" view: which stages
# have run and in what order, regardless of which specific entities
# they touched.
# ================================================================

require "json"

module CodingAdventures
  module CorrelationVector
    # ================================================================
    # Origin -- Where was an entity born?
    # ================================================================
    #
    # An Origin records the source and position of an entity at the
    # moment it was created. Think of it as a birth certificate.
    #
    # Fields:
    #   string     -- human-readable identifier (file path, table name,
    #                 service name, ingestion batch ID, etc.)
    #   synthetic  -- true if this entity has no natural origin (was
    #                 programmatically generated, not read from data)
    #
    # For synthetic entities (synthetic: true), the string field is
    # usually empty. The CV ID base will be "00000000" regardless.
    # ================================================================
    Origin = Struct.new(:string, :synthetic, keyword_init: true) do
      def initialize(string: nil, synthetic: false)
        super
      end
    end

    # ================================================================
    # Contribution -- What happened to an entity at one stage?
    # ================================================================
    #
    # Every time a stage processes an entity, it appends a Contribution.
    # Contributions are append-only -- they can never be removed.
    #
    # Fields:
    #   source     -- who contributed (stage name, service, pass name)
    #   tag        -- what kind of change (domain-defined label)
    #   meta       -- arbitrary key-value detail
    #   timestamp  -- ISO 8601 wall-clock time (optional; useful for
    #                 measuring latency between stages)
    #
    # The CV library imposes NO constraints on source, tag, or meta.
    # Those are entirely defined by the consuming domain. A compiler
    # uses different tags than an ETL pipeline. That's intentional.
    # ================================================================
    Contribution = Struct.new(:source, :tag, :meta, :timestamp, keyword_init: true) do
      def initialize(source:, tag:, meta: {}, timestamp: nil)
        super
      end
    end

    # ================================================================
    # DeletionRecord -- Who deleted an entity, and when?
    # ================================================================
    #
    # When an entity is deleted, we record WHO deleted it and WHEN.
    # The CV entry itself remains in the log permanently -- this is
    # the "append-only" guarantee. You can always ask "who deleted X
    # and why?" long after the deletion.
    #
    # Fields:
    #   by   -- the source (stage/actor) that performed the deletion
    #   at   -- ISO 8601 timestamp of deletion
    # ================================================================
    DeletionRecord = Struct.new(:by, :at, keyword_init: true)

    # ================================================================
    # CVEntry -- The complete record for one entity
    # ================================================================
    #
    # This is the "dossier" for a single tracked entity. Everything
    # that ever happened to it lives here.
    #
    # Fields:
    #   cv_id         -- stable identity string (never changes)
    #   origin        -- where/how this entity was born (nil=synthetic)
    #   parent_cv_id  -- single parent (for derived entities) or nil
    #   merged_from   -- array of parent IDs (for merged entities)
    #   contributions -- ordered history of stage contributions
    #   deleted       -- non-nil if entity was deleted
    #   pass_order    -- deduplicated ordered list of sources that
    #                    contributed to THIS entry specifically
    # ================================================================
    CVEntry = Struct.new(
      :cv_id,
      :origin,
      :parent_cv_id,
      :merged_from,
      :contributions,
      :deleted,
      :pass_order,
      keyword_init: true
    ) do
      def initialize(cv_id:, origin: nil, parent_cv_id: nil,
        merged_from: [], contributions: [], deleted: nil,
        pass_order: [])
        super
      end
    end

    # ================================================================
    # CVLog -- The append-only log of all CV entries
    # ================================================================
    #
    # The CVLog is the central data structure. It travels alongside
    # your data through the entire pipeline, accumulating the history
    # of every entity.
    #
    # Design principles:
    #
    # 1. **Append-only**: Nothing is ever deleted from the log. Even
    #    "deleted" entities stay in the log with a DeletionRecord.
    #    This ensures you can always reconstruct the full history.
    #
    # 2. **Zero cost when disabled**: When enabled: false, all write
    #    operations are no-ops. The cv_id is still generated and
    #    returned (the caller needs it), but no storage occurs.
    #    Production code can run with tracing off at near-zero cost.
    #
    # 3. **Globally unique IDs**: The combination of hash-based base
    #    and sequential counter ensures IDs never collide within a log.
    #
    # 4. **Deterministic**: Same inputs produce same IDs. This is
    #    important for reproducible builds and cross-process comparison.
    # ================================================================
    class CVLog
      attr_reader :enabled, :pass_order

      # Initialize a new CVLog.
      #
      # @param enabled [Boolean] when false, all write ops are no-ops
      def initialize(enabled: true)
        @enabled = enabled

        # The central store: cv_id (String) -> CVEntry
        # We use a plain Hash because CV IDs are strings and Hash
        # lookups are O(1) with very low constant factor.
        @entries = {}

        # Global pipeline ordering: every unique source name that
        # has contributed to any CV in this log, in order of first
        # appearance. This answers "what stages has this pipeline run?"
        @pass_order = []

        # Global counter for all IDs in this log.
        # Rather than per-base counters, we use one global counter
        # to guarantee uniqueness regardless of origin hash collisions.
        @counter = 0
      end

      # ================================================================
      # create -- Born a new root CV
      # ================================================================
      #
      # Every entity starts here. Call create when you first encounter
      # an entity and want to begin tracking it.
      #
      # The ID format is: <8-hex-base>.<N>
      #
      # The base is derived from a SHA-256 hash of the origin string.
      # This means two entities from the same source file will share
      # a base prefix, making the log easier to scan visually.
      #
      # For synthetic entities (no natural origin), the base is always
      # "00000000" -- eight zeros. This is the "born from nothing" marker.
      #
      # @param origin_string [String, nil] human-readable origin identifier
      # @param synthetic [Boolean] true if entity has no natural origin
      # @param meta [Hash] additional metadata stored on the origin
      # @return [String] the cv_id for this new entity
      def create(origin_string: nil, synthetic: false, meta: nil)
        # Compute the 8-character base from the origin.
        # For synthetic entities, use all zeros (the "null base").
        # For real entities, take the first 8 hex chars of SHA-256.
        base = compute_base(origin_string, synthetic)

        # Increment and capture the global counter.
        # This counter is the uniqueness guarantee -- even if two
        # entities have the same origin hash (a collision), they get
        # different N values.
        n = next_counter

        cv_id = "#{base}.#{n}"

        # If tracing is disabled, return the ID without storing anything.
        # The caller still needs the ID to attach it to their entity --
        # they just won't have any history if they query the log.
        return cv_id unless @enabled

        # Build the Origin struct if we have origin information.
        origin = if origin_string || synthetic
          Origin.new(string: origin_string, synthetic: synthetic)
        end

        # Store the entry with empty contributions. History is built
        # incrementally as stages call `contribute`.
        @entries[cv_id] = CVEntry.new(
          cv_id: cv_id,
          origin: origin,
          parent_cv_id: nil,
          merged_from: [],
          contributions: [],
          deleted: nil,
          pass_order: []
        )

        cv_id
      end

      # ================================================================
      # contribute -- Record that a stage processed this entity
      # ================================================================
      #
      # Call this every time a stage touches an entity. The contribution
      # is appended to the entity's history in call order.
      #
      # Error conditions:
      # - Entity not found: raises RuntimeError (programmer error;
      #   you should always call create before contribute)
      # - Entity deleted: raises RuntimeError (logical error; deleted
      #   entities cannot receive new contributions)
      #
      # @param cv_id [String] the entity's identifier
      # @param source [String] who/what is contributing
      # @param tag [String] what kind of contribution
      # @param meta [Hash] arbitrary detail about this contribution
      # @return [nil]
      def contribute(cv_id, source:, tag:, meta: nil)
        # If tracing is disabled, don't record anything.
        return nil unless @enabled

        entry = fetch_entry!(cv_id)
        raise_if_deleted!(entry)

        contribution = Contribution.new(
          source: source,
          tag: tag,
          meta: meta || {},
          timestamp: current_timestamp
        )

        entry.contributions << contribution

        # Deduplicated append to BOTH the entry-level pass_order
        # and the log-level pass_order.
        # Why deduplicated? A stage may contribute to the same entity
        # multiple times (e.g., a loop unroller making multiple passes).
        # We track "visited" not "visit count" in pass_order.
        entry.pass_order << source unless entry.pass_order.include?(source)
        @pass_order << source unless @pass_order.include?(source)

        nil
      end

      # ================================================================
      # derive -- Create a child entity from a parent
      # ================================================================
      #
      # Use derive when one entity splits into multiple outputs, or
      # when a transformation produces a "new version" of an entity.
      #
      # Example: destructuring {a, b} = x creates two derived entities.
      # Example: an ETL splitter creates two narrower records from one wide one.
      #
      # The derived ID is: <parent_cv_id>.<M>
      # where M is the next global counter value.
      #
      # This ID format makes parentage visible: "a3f1.1.2" is clearly
      # a grandchild of "a3f1" and a child of "a3f1.1".
      #
      # @param parent_cv_id [String] the parent entity's ID
      # @param source [String] who is performing the derivation
      # @param tag [String] the tag for the initial contribution
      # @param meta [Hash] additional metadata
      # @return [String] the new child entity's cv_id
      def derive(parent_cv_id, source:, tag:, meta: nil)
        if @enabled
          entry = fetch_entry!(parent_cv_id)
          raise_if_deleted!(entry)
        end

        # Derived ID appends a counter to the parent ID.
        # Example: "a3f1.1" -> "a3f1.1.1", "a3f1.1.2", etc.
        m = next_counter
        child_cv_id = "#{parent_cv_id}.#{m}"

        return child_cv_id unless @enabled

        # Record the initial contribution from whoever caused the derivation.
        initial_contribution = Contribution.new(
          source: source,
          tag: tag,
          meta: meta || {},
          timestamp: current_timestamp
        )

        @entries[child_cv_id] = CVEntry.new(
          cv_id: child_cv_id,
          origin: nil,
          parent_cv_id: parent_cv_id,
          merged_from: [],
          contributions: [initial_contribution],
          deleted: nil,
          pass_order: [source]
        )

        @pass_order << source unless @pass_order.include?(source)

        child_cv_id
      end

      # ================================================================
      # merge -- Combine multiple entities into one
      # ================================================================
      #
      # Use merge when multiple entities are combined into a single output.
      #
      # Example: inlining a function body into a call site merges two CVs.
      # Example: a JOIN operation in SQL merges two table rows.
      # Example: summing three neural network activations.
      #
      # The merged ID uses a fresh base derived from the sorted parent IDs.
      # This is deterministic: the same set of parents always produces the
      # same base (regardless of input order), which is important for
      # reproducible builds.
      #
      # @param cv_ids [Array<String>] the parent entity IDs to merge
      # @param source [String] who is performing the merge
      # @param tag [String] the tag for the initial contribution
      # @param meta [Hash] additional metadata
      # @return [String] the new merged entity's cv_id
      def merge(cv_ids, source:, tag:, meta: nil)
        if @enabled
          cv_ids.each do |pid|
            fetch_entry!(pid)  # must exist (but can be deleted -- see spec)
          end
        end

        # Compute a deterministic base from the sorted parent IDs.
        # Sort first so merge([a,b]) == merge([b,a]) in terms of ID.
        #
        # Why SHA-256? Because the base must be short (8 chars) and
        # stable. SHA-256 of the sorted joined IDs gives us a
        # content-addressed, collision-resistant identifier.
        sorted_ids = cv_ids.sort
        base = CodingAdventures::Sha256.sha256_hex(sorted_ids.join(","))[0, 8]

        n = next_counter
        merged_cv_id = "#{base}.#{n}"

        return merged_cv_id unless @enabled

        initial_contribution = Contribution.new(
          source: source,
          tag: tag,
          meta: meta || {},
          timestamp: current_timestamp
        )

        @entries[merged_cv_id] = CVEntry.new(
          cv_id: merged_cv_id,
          origin: nil,
          parent_cv_id: nil,
          merged_from: cv_ids,
          contributions: [initial_contribution],
          deleted: nil,
          pass_order: [source]
        )

        @pass_order << source unless @pass_order.include?(source)

        merged_cv_id
      end

      # ================================================================
      # delete -- Mark an entity as deleted
      # ================================================================
      #
      # Calling delete DOES NOT remove the CVEntry from the log.
      # It marks it with a DeletionRecord. This is the key property:
      # you can always ask "who deleted X?" even after the deletion.
      #
      # After deletion:
      # - contribute raises an error
      # - derive is still allowed (you can derive a tombstone record)
      # - merge is still allowed
      #
      # @param cv_id [String] the entity to mark as deleted
      # @param by [String] who is deleting it
      # @return [nil]
      def delete(cv_id, by:)
        return nil unless @enabled

        entry = fetch_entry!(cv_id)

        # Record the deletion with a timestamp for forensics.
        # Who deleted it? When? These questions become answerable.
        entry.deleted = DeletionRecord.new(
          by: by,
          at: current_timestamp
        )

        nil
      end

      # ================================================================
      # passthrough -- Record that a stage saw this entity but changed nothing
      # ================================================================
      #
      # A passthrough is the "identity contribution" -- it records that
      # a stage processed the entity but found nothing to change.
      #
      # Why track passthroughs at all? Because "which stages DID NOT
      # change this entity?" is just as useful as "which stages did."
      # If the output is wrong, you want to know which stages were even
      # aware of the entity.
      #
      # Passthrough is cheaper than contribute -- it only updates pass_order,
      # not the contributions array. It answers "this stage saw me" but
      # not "this stage changed me."
      #
      # @param cv_id [String] the entity being passed through
      # @param source [String] the stage doing the passing
      # @return [String] the same cv_id (passthrough by definition)
      def passthrough(cv_id, source:)
        return cv_id unless @enabled

        entry = fetch_entry!(cv_id)
        raise_if_deleted!(entry)

        # Deduplicated append to pass_order at both entry and log level.
        entry.pass_order << source unless entry.pass_order.include?(source)
        @pass_order << source unless @pass_order.include?(source)

        cv_id
      end

      # ================================================================
      # Querying the Log
      # ================================================================

      # get -- Return the full CVEntry for a cv_id, or nil if not found
      #
      # This is the basic lookup. Returns nil rather than raising if
      # the ID is unknown, to support "does this entity exist?" checks.
      #
      # @param cv_id [String]
      # @return [CVEntry, nil]
      def get(cv_id)
        @entries[cv_id]
      end

      # ancestors -- Walk the parent chain and return all ancestor IDs
      #
      # Returns ancestors nearest-first (immediate parent first,
      # most distant ancestor last). Uses BFS (breadth-first search)
      # so that for merged nodes (multiple parents), we explore
      # all immediate parents before going deeper.
      #
      # For a derivation chain A -> B -> C -> D:
      #   ancestors("D") => ["C", "B", "A"]
      #
      # For a merge M <- [A, B]:
      #   ancestors("M") => ["A", "B"] (or ["B", "A"] -- BFS order)
      #
      # @param cv_id [String]
      # @return [Array<String>] ancestor IDs, nearest-first
      def ancestors(cv_id)
        result = []
        # Use a queue (Array used as FIFO) for BFS.
        # BFS gives us nearest-first ordering naturally.
        queue = []
        visited = {}

        # Seed the queue with the immediate parents of cv_id.
        # We don't include cv_id itself in ancestors.
        entry = @entries[cv_id]
        return [] unless entry

        # Collect direct parents: either parent_cv_id (single) or
        # merged_from (multiple). These are the "level 1" ancestors.
        direct_parents = []
        direct_parents << entry.parent_cv_id if entry.parent_cv_id
        direct_parents.concat(entry.merged_from)

        direct_parents.each do |pid|
          unless visited[pid]
            visited[pid] = true
            queue << pid
            result << pid
          end
        end

        # BFS: for each queued ancestor, find ITS parents and enqueue them.
        until queue.empty?
          current = queue.shift
          current_entry = @entries[current]
          next unless current_entry

          grandparents = []
          grandparents << current_entry.parent_cv_id if current_entry.parent_cv_id
          grandparents.concat(current_entry.merged_from)

          grandparents.each do |gp|
            unless visited[gp]
              visited[gp] = true
              queue << gp
              result << gp
            end
          end
        end

        result
      end

      # descendants -- Return all IDs that descend from a given CV
      #
      # This is the inverse of ancestors. We scan the entire log for
      # entries that include cv_id in their parent chain.
      #
      # This is O(n) where n is the number of entries in the log. For
      # large logs, consider building an index. For typical pipeline
      # sizes, O(n) is fine.
      #
      # @param cv_id [String]
      # @return [Array<String>] all descendant IDs
      def descendants(cv_id)
        result = []

        @entries.each_value do |entry|
          next if entry.cv_id == cv_id

          # Check if cv_id appears in this entry's direct parents.
          # We use ancestors() recursively, but cache to avoid recomputation.
          # Actually, let's just check parent_cv_id and merged_from directly
          # and then do a graph walk.
          direct_parents = []
          direct_parents << entry.parent_cv_id if entry.parent_cv_id
          direct_parents.concat(entry.merged_from)

          if direct_parents.include?(cv_id)
            result << entry.cv_id
          end
        end

        # Now find indirect descendants: entries that descend from
        # any of the direct descendants.
        # We do this iteratively until no new descendants are found.
        known = Set.new(result)
        known.add(cv_id)

        loop do
          new_found = []
          @entries.each_value do |entry|
            next if known.include?(entry.cv_id)

            direct_parents = []
            direct_parents << entry.parent_cv_id if entry.parent_cv_id
            direct_parents.concat(entry.merged_from)

            if direct_parents.any? { |p| known.include?(p) }
              new_found << entry.cv_id
            end
          end

          break if new_found.empty?

          new_found.each { |id| known.add(id) }
          result.concat(new_found)
        end

        result
      end

      # history -- Return the contributions for a CV entry
      #
      # Returns the contributions in order (chronological as appended).
      # If the entity was never found or has no contributions, returns [].
      #
      # @param cv_id [String]
      # @return [Array<Contribution>]
      def history(cv_id)
        entry = @entries[cv_id]
        return [] unless entry

        entry.contributions.dup
      end

      # lineage -- Return the entity and all its ancestors, oldest first
      #
      # This is the "complete provenance chain": from the root ancestor
      # down to the entity itself. Oldest first means the root is at
      # index 0, the entity itself is at the last index.
      #
      # This is the inverse ordering of ancestors():
      #   ancestors(D) = [C, B, A]   (nearest first)
      #   lineage(D)   = [A, B, C, D] (oldest first)
      #
      # Only includes entries that exist in the log. If an ancestor's
      # ID is known but not stored (e.g., log is disabled or truncated),
      # it is silently skipped.
      #
      # @param cv_id [String]
      # @return [Array<CVEntry>] entries oldest-first, ending with cv_id
      def lineage(cv_id)
        ancestor_ids = ancestors(cv_id)

        # ancestors() returns nearest-first, so reverse for oldest-first.
        oldest_first_ids = ancestor_ids.reverse

        result = []
        oldest_first_ids.each do |id|
          entry = @entries[id]
          result << entry if entry
        end

        # Append the entity itself at the end.
        self_entry = @entries[cv_id]
        result << self_entry if self_entry

        result
      end

      # ================================================================
      # Serialization
      # ================================================================

      # serialize -- Convert the entire CVLog to a JSON string
      #
      # The JSON format is the canonical interchange format. It allows
      # a CVLog to be:
      # - Persisted to disk between runs
      # - Transmitted across process boundaries
      # - Compared between language implementations
      #
      # We use our own CodingAdventures::JsonSerializer to exercise it
      # (dogfooding our own libraries), then round-trip verify with
      # Ruby's stdlib JSON for deserialization.
      #
      # @return [String] JSON representation of the log
      def serialize
        hash = {
          "enabled" => @enabled,
          "counter" => @counter,
          "pass_order" => @pass_order,
          "entries" => @entries.transform_values { |e| serialize_entry(e) }
        }

        # Use our own JsonSerializer as required by the spec.
        # This exercises the full JsonValue -> JSON pipeline.
        # CodingAdventures::JsonValue.from_native converts Ruby
        # Hash/Array/String/Number/Boolean/nil into typed JsonValue nodes.
        # CodingAdventures::JsonSerializer.serialize then renders
        # those nodes to compact JSON text.
        jv = CodingAdventures::JsonValue.from_native(hash)
        CodingAdventures::JsonSerializer.serialize(jv)
      end

      # self.deserialize -- Reconstruct a CVLog from JSON
      #
      # The inverse of serialize. Uses Ruby's stdlib JSON.parse for
      # deserialization (fast and correct for this direction).
      #
      # @param json_string [String] JSON produced by serialize
      # @return [CVLog] the reconstructed log
      def self.deserialize(json_string)
        data = JSON.parse(json_string)

        log = new(enabled: data["enabled"])
        log.instance_variable_set(:@counter, data["counter"] || 0)
        log.instance_variable_set(:@pass_order, data["pass_order"] || [])

        entries = {}
        (data["entries"] || {}).each do |cv_id, entry_data|
          entries[cv_id] = deserialize_entry(entry_data)
        end
        log.instance_variable_set(:@entries, entries)

        log
      end

      # ================================================================
      # Internal helpers (private)
      # ================================================================

      private

      # Compute the 8-character hex base for a new root CV.
      #
      # Algorithm:
      # - Synthetic entity -> "00000000" (eight zeros, the null base)
      # - Real entity with origin string -> SHA-256(origin_string)[0..7]
      # - No origin string -> "00000000" (treat as synthetic)
      #
      # The base is 8 hex characters = 32 bits of the hash.
      # This provides ~4 billion distinct bases before collision.
      # In practice, origins are usually unique (file paths, timestamps)
      # so collisions at the base level are rare.
      def compute_base(origin_string, synthetic)
        return "00000000" if synthetic
        return "00000000" unless origin_string && !origin_string.empty?

        CodingAdventures::Sha256.sha256_hex(origin_string)[0, 8]
      end

      # Increment and return the global counter.
      #
      # The counter is the global uniqueness guarantee. Even if two
      # entities have the same hash base, they get different N values.
      # The counter never resets within a log's lifetime.
      def next_counter
        @counter += 1
      end

      # Return the current wall-clock time as an ISO 8601 string.
      #
      # We use UTC to avoid timezone ambiguity. All timestamps in a
      # CVLog are in UTC, making cross-process comparison meaningful.
      def current_timestamp
        Time.now.utc.iso8601
      end

      # Fetch a CVEntry or raise a descriptive error.
      #
      # We raise RuntimeError (not KeyError) because:
      # 1. RuntimeError is the conventional Ruby error for "you did
      #    something wrong at runtime"
      # 2. KeyError would be odd coming from a public API
      def fetch_entry!(cv_id)
        entry = @entries[cv_id]
        raise "CVEntry not found: #{cv_id.inspect}" unless entry

        entry
      end

      # Raise if the entry has been deleted.
      #
      # Calling contribute on a deleted entity is a logical error.
      # The entity is gone -- there's nothing to contribute to.
      def raise_if_deleted!(entry)
        return unless entry.deleted

        raise "Cannot contribute to deleted CV entry: #{entry.cv_id.inspect} " \
              "(deleted by #{entry.deleted.by.inspect} at #{entry.deleted.at.inspect})"
      end

      # Serialize one CVEntry to a plain Ruby Hash for JSON output.
      def serialize_entry(entry)
        {
          "cv_id" => entry.cv_id,
          "origin" => serialize_origin(entry.origin),
          "parent_cv_id" => entry.parent_cv_id,
          "merged_from" => entry.merged_from,
          "contributions" => entry.contributions.map { |c| serialize_contribution(c) },
          "deleted" => serialize_deletion(entry.deleted),
          "pass_order" => entry.pass_order
        }
      end

      # Serialize an Origin struct (or nil) to a Hash.
      def serialize_origin(origin)
        return nil unless origin

        {
          "string" => origin.string,
          "synthetic" => origin.synthetic
        }
      end

      # Serialize a Contribution struct to a Hash.
      def serialize_contribution(contribution)
        {
          "source" => contribution.source,
          "tag" => contribution.tag,
          "meta" => contribution.meta,
          "timestamp" => contribution.timestamp
        }
      end

      # Serialize a DeletionRecord struct (or nil) to a Hash.
      def serialize_deletion(deletion)
        return nil unless deletion

        {
          "by" => deletion.by,
          "at" => deletion.at
        }
      end

      # Deserialize one CVEntry from a plain Ruby Hash.
      #
      # We reconstruct each Struct field carefully, using safe defaults
      # for missing keys (for forward compatibility if the schema evolves).
      def self.deserialize_entry(data)
        origin = if data["origin"]
          o = data["origin"]
          Origin.new(
            string: o["string"],
            synthetic: o["synthetic"] || false
          )
        end

        contributions = (data["contributions"] || []).map do |c|
          Contribution.new(
            source: c["source"],
            tag: c["tag"],
            meta: c["meta"] || {},
            timestamp: c["timestamp"]
          )
        end

        deleted = if data["deleted"]
          d = data["deleted"]
          DeletionRecord.new(by: d["by"], at: d["at"])
        end

        CVEntry.new(
          cv_id: data["cv_id"],
          origin: origin,
          parent_cv_id: data["parent_cv_id"],
          merged_from: data["merged_from"] || [],
          contributions: contributions,
          deleted: deleted,
          pass_order: data["pass_order"] || []
        )
      end

      private_class_method :deserialize_entry
    end
  end
end
