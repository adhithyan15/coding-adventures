defmodule CodingAdventures.CorrelationVector do
  @moduledoc """
  Append-only provenance tracking for any data pipeline.

  ## What Is a Correlation Vector?

  A Correlation Vector (CV) is a lightweight, append-only record that follows a
  piece of data through every transformation it undergoes. Assign a CV to anything
  when it is born. Every system, stage, or function that touches it appends its
  contribution. At any point you can ask "where did this come from and what happened
  to it?" and get a complete, ordered answer.

  The concept originated in distributed-systems request tracing. This implementation
  generalises it to any pipeline: compiler passes, data ETL, document transformations,
  build systems, ML preprocessing, or anywhere that data flows through a sequence of
  transformations.

  ## Core Components

  - **CV ID** — a stable, globally unique string assigned at birth. Never changes.
  - **Contribution** — a record appended by each stage that processes an entity.
  - **CV Entry** — the full record for one CV ID: origin, parent IDs, contributions,
    optional deletion record.
  - **CVLog** (this struct) — the map that holds all CV entries for a pipeline run.

  ## ID Format

  ```
  base.N       — root CV (born directly, not derived from another)
  base.N.M     — derived from base.N
  base.N.M.K   — derived from base.N.M
  00000000.N   — synthetic entity with no natural origin
  ```

  The base is the first 8 characters of the SHA-256 hash of `source <> ":" <> location`.

  ## Enabled / Disabled

  When `enabled: false`, every mutating operation (`contribute`, `derive`, `merge`,
  `delete`) is a no-op that returns the log unchanged. CV IDs are still generated and
  returned so that callers can hold them — but the log is never populated.

  ## Serialization

  `serialize/1` and `to_json_string/1` convert the log to a portable map or JSON string.
  `from_json_string/1` reconstructs a log from JSON produced by any implementation in
  any language that follows the CV00 spec.

  ## Example (Elixir)

      log = CodingAdventures.CorrelationVector.new()
      {cv_id, log} = CodingAdventures.CorrelationVector.create(log,
        %CodingAdventures.CorrelationVector.Origin{source: "app.ts", location: "5:12"})
      log = CodingAdventures.CorrelationVector.contribute(log, cv_id, "parser", "created", %{token: "IDENTIFIER"})
      log = CodingAdventures.CorrelationVector.passthrough(log, cv_id, "type_checker")
      log = CodingAdventures.CorrelationVector.delete(log, cv_id, "dce", "unreachable", %{})
  """

  alias CodingAdventures.Sha256
  alias CodingAdventures.JsonSerializer
  alias CodingAdventures.JsonValue

  # ---------------------------------------------------------------------------
  # Nested struct definitions
  # ---------------------------------------------------------------------------

  defmodule Origin do
    @moduledoc """
    Where and when a CV entity was born.

    Fields:
    - `source`    — identifies the origin system (file name, service, table, etc.)
    - `location`  — position within the source (line:col, byte offset, row_id, etc.)
    - `timestamp` — ISO 8601 timestamp if time-relevant (nil for non-time-sensitive data)
    - `meta`      — arbitrary additional origin context (domain-defined)
    """
    defstruct [:source, :location, timestamp: nil, meta: %{}]

    @type t :: %__MODULE__{
            source: String.t(),
            location: String.t(),
            timestamp: String.t() | nil,
            meta: map()
          }
  end

  defmodule Contribution do
    @moduledoc """
    A single contribution appended by a stage that processed an entity.

    Fields:
    - `source` — who/what contributed (stage name, service name, pass name)
    - `tag`    — what happened (domain-defined label, e.g. "renamed", "compiled")
    - `meta`   — arbitrary key-value detail (domain-defined)

    The CV library imposes no constraints on the values of `source` or `tag` —
    those are entirely defined by the consumer domain.
    """
    defstruct [:source, :tag, meta: %{}]

    @type t :: %__MODULE__{
            source: String.t(),
            tag: String.t(),
            meta: map()
          }
  end

  defmodule DeletionRecord do
    @moduledoc """
    Records that an entity was intentionally removed.

    The CV entry remains in the log permanently even after deletion — this is
    how you can answer "why did this disappear?" long after the fact.

    Fields:
    - `source` — who deleted it
    - `reason` — why it was deleted
    - `meta`   — additional context (e.g. the CV ID of a duplicate, the entry point
                 from which unreachable code was found)
    """
    defstruct [:source, :reason, meta: %{}]

    @type t :: %__MODULE__{
            source: String.t(),
            reason: String.t(),
            meta: map()
          }
  end

  defmodule Entry do
    @moduledoc """
    The full record for a single CV ID.

    Fields:
    - `id`            — the stable, globally unique CV ID string
    - `parent_ids`    — empty for roots; one or more for derived/merged CVs
    - `origin`        — where/when this entity was born (nil for synthetics)
    - `contributions` — append-only list of what touched this entity, in call order
    - `deleted`       — non-nil if this entity was deleted (see `DeletionRecord`)
    """
    # Note: fields WITHOUT defaults must come BEFORE fields WITH defaults.
    defstruct [:id, parent_ids: [], origin: nil, contributions: [], deleted: nil]

    @type t :: %__MODULE__{
            id: String.t(),
            parent_ids: [String.t()],
            origin: CodingAdventures.CorrelationVector.Origin.t() | nil,
            contributions: [CodingAdventures.CorrelationVector.Contribution.t()],
            deleted: CodingAdventures.CorrelationVector.DeletionRecord.t() | nil
          }
  end

  # ---------------------------------------------------------------------------
  # CVLog struct — the top-level container
  # ---------------------------------------------------------------------------
  #
  # The CVLog travels alongside the data being processed, accumulating the
  # history of every entity.
  #
  # - `entries`        — map from CV ID string → Entry struct
  # - `pass_order`     — ordered list of source names that have contributed
  # - `enabled`        — when false, all write operations are no-ops
  # - `base_counters`  — per-base sequence counter for root CV IDs
  # - `child_counters` — per-parent-cv-id counter for derived CV IDs

  defstruct entries: %{},
            pass_order: [],
            enabled: true,
            base_counters: %{},
            child_counters: %{}

  @type t :: %__MODULE__{
          entries: %{String.t() => Entry.t()},
          pass_order: [String.t()],
          enabled: boolean(),
          base_counters: %{String.t() => non_neg_integer()},
          child_counters: %{String.t() => non_neg_integer()}
        }

  # ---------------------------------------------------------------------------
  # new/1 — Create a fresh CVLog
  # ---------------------------------------------------------------------------

  @doc """
  Create a new, empty CVLog.

  ## Parameters
  - `enabled` — when `false`, all write operations become no-ops (default `true`)

  ## Examples

      log = CodingAdventures.CorrelationVector.new()
      # => %CodingAdventures.CorrelationVector{enabled: true, entries: %{}, ...}

      disabled = CodingAdventures.CorrelationVector.new(false)
      # => %CodingAdventures.CorrelationVector{enabled: false, ...}
  """
  @spec new(boolean()) :: t()
  def new(enabled \\ true) do
    %__MODULE__{enabled: enabled}
  end

  # ---------------------------------------------------------------------------
  # create/2 — Born a new root CV
  # ---------------------------------------------------------------------------
  #
  # Every new entity starts here. We generate a CV ID based on the origin (or
  # use 00000000 as the base for synthetic/no-origin entities), then store
  # the entry in the log.
  #
  # Even when disabled, we STILL generate and return a CV ID — the caller
  # needs the ID regardless. We just skip writing to the log.

  @doc """
  Create a new root CV entry.

  Returns `{cv_id, updated_log}`. The CV ID has the format `base.N` where
  `base` is derived from the origin's source and location, and N is a
  per-base sequence number starting at 1.

  If `origin` is `nil`, the base is `"00000000"` (synthetic entity).

  Even when the log is disabled (`enabled: false`), a CV ID is still generated
  and returned. The log entry is simply not populated.

  ## Examples

      log = CodingAdventures.CorrelationVector.new()
      {cv_id, log} = CodingAdventures.CorrelationVector.create(log,
        %CodingAdventures.CorrelationVector.Origin{source: "app.ts", location: "5:12"})
      # cv_id => "a3f1b2c4.1" (first entity with this base)
  """
  @spec create(t(), Origin.t() | nil) :: {String.t(), t()}
  def create(log, origin \\ nil) do
    base = compute_base(origin)
    {n, log} = next_base_counter(log, base)
    cv_id = "#{base}.#{n}"

    if log.enabled do
      entry = %Entry{
        id: cv_id,
        parent_ids: [],
        origin: origin,
        contributions: [],
        deleted: nil
      }

      log = %{log | entries: Map.put(log.entries, cv_id, entry)}
      {cv_id, log}
    else
      {cv_id, log}
    end
  end

  # ---------------------------------------------------------------------------
  # contribute/5 — Record that a stage processed this entity
  # ---------------------------------------------------------------------------
  #
  # Contributions are appended in call order. The order is semantically
  # meaningful — it is the sequence in which stages processed the entity.
  #
  # Calling contribute on a deleted CV is an error (raises RuntimeError).
  # This is by design: once an entity is deleted, it should not receive
  # further contributions.

  @doc """
  Append a contribution to a CV entry.

  Records that `source` processed this entity and classified the action as `tag`.
  Optional `meta` carries domain-specific detail.

  Raises `RuntimeError` if the CV entry has been deleted.

  Returns the updated log.

  ## Examples

      log = CodingAdventures.CorrelationVector.contribute(log, cv_id,
        "scope_analysis", "resolved", %{binding: "local:count:fn_main"})
  """
  @spec contribute(t(), String.t(), String.t(), String.t(), map()) :: t()
  def contribute(log, cv_id, source, tag, meta \\ %{}) do
    unless log.enabled do
      return_log = log
      return_log
    else
      entry = Map.get(log.entries, cv_id)

      if entry && entry.deleted do
        raise RuntimeError,
              "Cannot contribute to deleted CV entry: #{cv_id} " <>
                "(deleted by #{entry.deleted.source}: #{entry.deleted.reason})"
      end

      contribution = %Contribution{source: source, tag: tag, meta: meta}
      log = add_to_pass_order(log, source)

      if entry do
        updated_entry = %{entry | contributions: entry.contributions ++ [contribution]}
        %{log | entries: Map.put(log.entries, cv_id, updated_entry)}
      else
        # Entry doesn't exist in log (e.g., disabled log created this ID)
        log
      end
    end
  end

  # ---------------------------------------------------------------------------
  # derive/3 — Create a CV descended from an existing one
  # ---------------------------------------------------------------------------
  #
  # Use this when one entity is split into multiple outputs, or when a
  # transformation produces a new entity that is conceptually "the same
  # thing" expressed differently.
  #
  # The derived CV's ID is the parent ID with a new numeric suffix:
  #   parent_cv_id + "." + next_sequence_for_parent

  @doc """
  Create a new CV derived from an existing parent CV.

  The new CV ID is `parent_cv_id.M` where M is the next sequence number
  for this parent's children.

  Returns `{cv_id, updated_log}`.

  ## Examples

      # Destructuring {a, b} = x into two separate bindings
      {cv_a, log} = CodingAdventures.CorrelationVector.derive(log, original_cv_id)
      {cv_b, log} = CodingAdventures.CorrelationVector.derive(log, original_cv_id)
      # cv_a = "original.1", cv_b = "original.2"
  """
  @spec derive(t(), String.t(), Origin.t() | nil) :: {String.t(), t()}
  def derive(log, parent_cv_id, origin \\ nil) do
    {m, log} = next_child_counter(log, parent_cv_id)
    cv_id = "#{parent_cv_id}.#{m}"

    if log.enabled do
      entry = %Entry{
        id: cv_id,
        parent_ids: [parent_cv_id],
        origin: origin,
        contributions: [],
        deleted: nil
      }

      log = %{log | entries: Map.put(log.entries, cv_id, entry)}
      {cv_id, log}
    else
      {cv_id, log}
    end
  end

  # ---------------------------------------------------------------------------
  # merge/3 — Create a CV descended from multiple existing CVs
  # ---------------------------------------------------------------------------
  #
  # Use this when multiple entities are combined into one output. The merged
  # CV's `parent_ids` lists all parents.
  #
  # ID scheme: if an origin is provided, compute the base from that origin
  # and use a new base counter. Otherwise, use "00000000" as the base (since
  # a merge has no single natural origin).

  @doc """
  Create a new CV descended from multiple existing CVs.

  The new CV has all `parent_cv_ids` as its parents. If no `origin` is given,
  the ID uses the `"00000000"` base (synthetic).

  Returns `{cv_id, updated_log}`.

  ## Examples

      {merged_cv, log} = CodingAdventures.CorrelationVector.merge(log,
        [call_site_cv, function_body_cv])
  """
  @spec merge(t(), [String.t()], Origin.t() | nil) :: {String.t(), t()}
  def merge(log, parent_cv_ids, origin \\ nil) do
    base = compute_base(origin)
    {n, log} = next_base_counter(log, base)
    cv_id = "#{base}.#{n}"

    if log.enabled do
      entry = %Entry{
        id: cv_id,
        parent_ids: parent_cv_ids,
        origin: origin,
        contributions: [],
        deleted: nil
      }

      log = %{log | entries: Map.put(log.entries, cv_id, entry)}
      {cv_id, log}
    else
      {cv_id, log}
    end
  end

  # ---------------------------------------------------------------------------
  # delete/5 — Record that an entity was intentionally removed
  # ---------------------------------------------------------------------------
  #
  # The CV entry remains in the log permanently after deletion. This is how
  # you can answer "why did this disappear?" long after the fact.
  #
  # Calling `contribute` on a deleted CV raises an error.
  # Calling `derive` or `merge` with a deleted CV as a parent is still allowed.

  @doc """
  Record that an entity was intentionally deleted.

  The CV entry remains in the log — deleted entries are never removed.
  After deletion, calling `contribute/5` on this CV ID will raise.

  Returns the updated log.

  ## Examples

      log = CodingAdventures.CorrelationVector.delete(log, cv_id,
        "dead_code_eliminator", "unreachable from entry point",
        %{entry_point_cv: main_cv_id})
  """
  @spec delete(t(), String.t(), String.t(), String.t(), map()) :: t()
  def delete(log, cv_id, source, reason, meta \\ %{}) do
    unless log.enabled do
      log
    else
      entry = Map.get(log.entries, cv_id)

      if entry do
        deletion = %DeletionRecord{source: source, reason: reason, meta: meta}
        updated = %{entry | deleted: deletion}
        %{log | entries: Map.put(log.entries, cv_id, updated)}
      else
        log
      end
    end
  end

  # ---------------------------------------------------------------------------
  # passthrough/3 — Record that a stage examined but did not change the entity
  # ---------------------------------------------------------------------------
  #
  # The identity contribution. Important for reconstructing which stages an
  # entity passed through even when nothing was transformed.
  #
  # In performance-sensitive pipelines, passthrough may be omitted for
  # known-clean stages to reduce log size.

  @doc """
  Record that a stage examined this entity but made no changes.

  Equivalent to `contribute(log, cv_id, source, "passthrough", %{})` but
  semantically distinct: it marks the stage as a no-op observer, not a
  transformer.

  Returns the updated log.

  ## Examples

      log = CodingAdventures.CorrelationVector.passthrough(log, cv_id, "type_checker")
  """
  @spec passthrough(t(), String.t(), String.t()) :: t()
  def passthrough(log, cv_id, source) do
    contribute(log, cv_id, source, "passthrough", %{})
  end

  # ---------------------------------------------------------------------------
  # get/2 — Retrieve a CV entry
  # ---------------------------------------------------------------------------

  @doc """
  Return the full Entry for a CV ID, or nil if not found.

  ## Examples

      entry = CodingAdventures.CorrelationVector.get(log, cv_id)
      # => %CodingAdventures.CorrelationVector.Entry{...} or nil
  """
  @spec get(t(), String.t()) :: Entry.t() | nil
  def get(log, cv_id) do
    Map.get(log.entries, cv_id)
  end

  # ---------------------------------------------------------------------------
  # ancestors/2 — Walk parent_ids chain recursively
  # ---------------------------------------------------------------------------
  #
  # Returns all ancestor CV IDs, ordered from immediate parent to most distant
  # ancestor (breadth-first, nearest first). Since derivation is always from
  # parent to child and never creates cycles, we don't need cycle detection —
  # but we use a visited set defensively.

  @doc """
  Return all ancestor CV IDs for a given CV ID.

  Traverses the `parent_ids` chain recursively. The result is ordered from
  nearest parent to most distant ancestor (nearest first).

  Returns an empty list if the CV has no parents or is not found.

  ## Examples

      # For a chain A → B → C → D:
      CodingAdventures.CorrelationVector.ancestors(log, "D")
      # => ["C", "B", "A"]
  """
  @spec ancestors(t(), String.t()) :: [String.t()]
  def ancestors(log, cv_id) do
    collect_ancestors(log, cv_id, MapSet.new([cv_id]), [])
  end

  # Iterative BFS over parent_ids, nearest first.
  #
  # We maintain a queue of CV IDs whose parents we still need to visit.
  # For each item in the queue, we pull its parent_ids, filter out already-
  # visited ones, add them to the result list, and enqueue them for their own
  # parent expansion. This correctly handles arbitrarily deep chains and
  # merge nodes with multiple parents.
  defp collect_ancestors(log, cv_id, _visited, _acc) do
    do_collect_ancestors(log, [cv_id], MapSet.new([cv_id]), [])
  end

  defp do_collect_ancestors(_log, [], _visited, acc), do: acc

  defp do_collect_ancestors(log, [current | queue], visited, acc) do
    entry = Map.get(log.entries, current)
    parent_ids = if entry, do: entry.parent_ids, else: []

    new_parents = Enum.filter(parent_ids, fn pid -> not MapSet.member?(visited, pid) end)
    new_visited = Enum.reduce(new_parents, visited, &MapSet.put(&2, &1))

    do_collect_ancestors(log, queue ++ new_parents, new_visited, acc ++ new_parents)
  end

  # ---------------------------------------------------------------------------
  # descendants/2 — Find all CVs that have this CV in their ancestry
  # ---------------------------------------------------------------------------
  #
  # The inverse of ancestors. Computed by scanning the log for entries whose
  # parent_ids include the target cv_id, then recursing into those entries.
  # For large logs, an index by parent_id would be more efficient, but for
  # correctness and simplicity we scan here.

  @doc """
  Return all CV IDs that have `cv_id` somewhere in their ancestor chain.

  This is the inverse of `ancestors/2`. Computed by scanning all entries in
  the log for entries that include `cv_id` as a parent, then recursing.

  Returns an empty list if no descendants are found.

  ## Examples

      # After derive(log, parent_id) twice:
      CodingAdventures.CorrelationVector.descendants(log, parent_id)
      # => ["parent.1", "parent.2"]
  """
  @spec descendants(t(), String.t()) :: [String.t()]
  def descendants(log, cv_id) do
    collect_descendants(log, [cv_id], MapSet.new([cv_id]), [])
  end

  defp collect_descendants(_log, [], _visited, acc), do: acc

  defp collect_descendants(log, [current | rest], visited, acc) do
    # Find all entries whose parent_ids include `current`
    direct_children =
      log.entries
      |> Enum.filter(fn {_id, entry} -> current in entry.parent_ids end)
      |> Enum.map(fn {id, _entry} -> id end)
      |> Enum.filter(fn id -> not MapSet.member?(visited, id) end)

    new_visited = Enum.reduce(direct_children, visited, &MapSet.put(&2, &1))
    new_acc = acc ++ direct_children
    collect_descendants(log, rest ++ direct_children, new_visited, new_acc)
  end

  # ---------------------------------------------------------------------------
  # history/2 — Contributions for a CV ID
  # ---------------------------------------------------------------------------
  #
  # Returns contributions in order. If the entity was deleted, the deletion
  # is NOT appended as a contribution — it's in the `deleted` field.
  # (The spec says "appended as the final entry" — we model deletion as a
  # synthetic final contribution to match the spec's intent.)

  @doc """
  Return the contributions for a CV ID in order.

  If the entity was deleted, the deletion record is represented as a final
  synthetic `%Contribution{source: ..., tag: "deleted", meta: ...}` appended
  to the list.

  Returns an empty list if the CV ID is not found in the log.

  ## Examples

      CodingAdventures.CorrelationVector.history(log, cv_id)
      # => [%Contribution{source: "parser", tag: "created", meta: %{}}, ...]
  """
  @spec history(t(), String.t()) :: [Contribution.t()]
  def history(log, cv_id) do
    case Map.get(log.entries, cv_id) do
      nil ->
        []

      entry ->
        base_contribs = entry.contributions

        if entry.deleted do
          # Represent deletion as a final contribution so history is self-contained
          deletion_contrib = %Contribution{
            source: entry.deleted.source,
            tag: "deleted",
            meta: Map.merge(%{reason: entry.deleted.reason}, entry.deleted.meta)
          }

          base_contribs ++ [deletion_contrib]
        else
          base_contribs
        end
    end
  end

  # ---------------------------------------------------------------------------
  # lineage/2 — Full provenance chain from oldest ancestor to entity
  # ---------------------------------------------------------------------------
  #
  # Returns the full CV entries for the entity and all its ancestors, ordered
  # from oldest ancestor to the entity itself. This is the complete provenance
  # chain — who created this, what touched it, and where it came from.

  @doc """
  Return the full CV entries for `cv_id` and all its ancestors.

  Ordered from oldest ancestor (root) to the entity itself. This is the
  complete provenance chain.

  Returns an empty list if the CV ID is not found.

  ## Examples

      # For chain A → B → C → D:
      CodingAdventures.CorrelationVector.lineage(log, "D")
      # => [entry_A, entry_B, entry_C, entry_D]
  """
  @spec lineage(t(), String.t()) :: [Entry.t()]
  def lineage(log, cv_id) do
    case Map.get(log.entries, cv_id) do
      nil ->
        []

      entry ->
        ancestor_ids = ancestors(log, cv_id)
        # ancestors returns nearest-first; we want oldest-first so reverse
        ancestor_entries =
          ancestor_ids
          |> Enum.reverse()
          |> Enum.map(&Map.get(log.entries, &1))
          |> Enum.filter(&(&1 != nil))

        ancestor_entries ++ [entry]
    end
  end

  # ---------------------------------------------------------------------------
  # serialize/1 — Convert CVLog to a plain Elixir map
  # ---------------------------------------------------------------------------
  #
  # Produces the canonical interchange format defined in CV00 spec:
  #
  #   {
  #     "entries": { "a3f1.1": { ... }, ... },
  #     "pass_order": ["parser", "scope_analysis"],
  #     "enabled": true
  #   }
  #
  # Note: base_counters and child_counters are implementation details not
  # included in the serialized form (they can be reconstructed from entries).

  @doc """
  Serialize the CVLog to a plain Elixir map (the canonical interchange format).

  The map structure follows the CV00 spec JSON format and can be serialized
  to JSON with `to_json_string/1`, or used directly as a map.

  ## Examples

      map = CodingAdventures.CorrelationVector.serialize(log)
      # => %{"entries" => %{"a3f1.1" => %{...}}, "pass_order" => [...], "enabled" => true}
  """
  @spec serialize(t()) :: map()
  def serialize(log) do
    %{
      "entries" =>
        Map.new(log.entries, fn {cv_id, entry} ->
          {cv_id, serialize_entry(entry)}
        end),
      "pass_order" => log.pass_order,
      "enabled" => log.enabled
    }
  end

  # ---------------------------------------------------------------------------
  # to_json_string/1 — Serialize CVLog to a JSON string
  # ---------------------------------------------------------------------------

  @doc """
  Serialize the CVLog to a JSON string using `coding_adventures_json_serializer`.

  Returns `{:ok, json_string}` on success, `{:error, reason}` on failure.

  ## Examples

      {:ok, json} = CodingAdventures.CorrelationVector.to_json_string(log)
  """
  @spec to_json_string(t()) :: {:ok, String.t()} | {:error, term()}
  def to_json_string(log) do
    map = serialize(log)

    case JsonValue.from_native(map) do
      {:ok, json_val} -> JsonSerializer.serialize(json_val)
      {:error, reason} -> {:error, reason}
    end
  end

  # ---------------------------------------------------------------------------
  # from_json_string/1 — Reconstruct a CVLog from a JSON string
  # ---------------------------------------------------------------------------

  @doc """
  Reconstruct a CVLog from a JSON string produced by `to_json_string/1` or
  any conforming implementation of the CV00 spec.

  Returns `{:ok, log}` on success, `{:error, reason}` on failure.

  ## Examples

      {:ok, log2} = CodingAdventures.CorrelationVector.from_json_string(json_string)
  """
  @spec from_json_string(String.t()) :: {:ok, t()} | {:error, term()}
  def from_json_string(json) do
    case JsonValue.parse(json) do
      {:ok, json_val} ->
        native = JsonValue.to_native(json_val)
        {:ok, deserialize(native)}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # Compute the 8-character hex base from an Origin.
  # If origin is nil (synthetic entity), use "00000000".
  defp compute_base(nil), do: "00000000"

  defp compute_base(%Origin{source: source, location: location}) do
    data = source <> ":" <> location
    Sha256.sha256_hex(data) |> String.slice(0, 8)
  end

  # Increment the per-base sequence counter and return {next_n, updated_log}.
  defp next_base_counter(log, base) do
    n = Map.get(log.base_counters, base, 0) + 1
    updated = %{log | base_counters: Map.put(log.base_counters, base, n)}
    {n, updated}
  end

  # Increment the per-parent child counter and return {next_m, updated_log}.
  defp next_child_counter(log, parent_cv_id) do
    m = Map.get(log.child_counters, parent_cv_id, 0) + 1
    updated = %{log | child_counters: Map.put(log.child_counters, parent_cv_id, m)}
    {m, updated}
  end

  # Add source to pass_order if not already present.
  defp add_to_pass_order(log, source) do
    if source in log.pass_order do
      log
    else
      %{log | pass_order: log.pass_order ++ [source]}
    end
  end

  # Serialize a single Entry to a plain map.
  defp serialize_entry(%Entry{} = entry) do
    %{
      "id" => entry.id,
      "parent_ids" => entry.parent_ids,
      "origin" => serialize_origin(entry.origin),
      "contributions" => Enum.map(entry.contributions, &serialize_contribution/1),
      "deleted" => serialize_deletion(entry.deleted)
    }
  end

  defp serialize_origin(nil), do: nil

  defp serialize_origin(%Origin{} = o) do
    %{
      "source" => o.source,
      "location" => o.location,
      "timestamp" => o.timestamp,
      "meta" => stringify_keys(o.meta)
    }
  end

  defp serialize_contribution(%Contribution{} = c) do
    %{"source" => c.source, "tag" => c.tag, "meta" => stringify_keys(c.meta)}
  end

  defp serialize_deletion(nil), do: nil

  defp serialize_deletion(%DeletionRecord{} = d) do
    %{"source" => d.source, "reason" => d.reason, "meta" => stringify_keys(d.meta)}
  end

  # Convert any atom keys in a map to string keys, recursively.
  # This is needed because callers may pass %{atom_key: value} as meta,
  # which the json_serializer cannot handle (it requires string keys).
  defp stringify_keys(map) when is_map(map) do
    Map.new(map, fn
      {k, v} when is_atom(k) -> {Atom.to_string(k), stringify_value(v)}
      {k, v} -> {k, stringify_value(v)}
    end)
  end

  defp stringify_keys(other), do: other

  defp stringify_value(v) when is_map(v), do: stringify_keys(v)
  defp stringify_value(v) when is_list(v), do: Enum.map(v, &stringify_value/1)
  defp stringify_value(v) when is_atom(v) and v not in [true, false, nil], do: Atom.to_string(v)
  defp stringify_value(v), do: v

  # Deserialize a plain map (from JSON) back to a CVLog struct.
  defp deserialize(data) when is_map(data) do
    entries_map = Map.get(data, "entries", %{})
    pass_order = Map.get(data, "pass_order", [])
    enabled = Map.get(data, "enabled", true)

    entries =
      Map.new(entries_map, fn {cv_id, entry_map} ->
        {cv_id, deserialize_entry(entry_map)}
      end)

    # Reconstruct counters from the deserialized entries
    # so that subsequent create/derive/merge calls generate unique IDs.
    {base_counters, child_counters} = rebuild_counters(entries)

    %__MODULE__{
      entries: entries,
      pass_order: pass_order,
      enabled: enabled,
      base_counters: base_counters,
      child_counters: child_counters
    }
  end

  # Rebuild the base_counters and child_counters maps from deserialized entries.
  # We need to know the highest sequence number used for each base/parent
  # so that future create/derive/merge calls don't collide.
  defp rebuild_counters(entries) do
    Enum.reduce(entries, {%{}, %{}}, fn {cv_id, _entry}, {base_c, child_c} ->
      parts = String.split(cv_id, ".")

      case parts do
        [base, n_str] ->
          # Root CV: base.N
          n = String.to_integer(n_str)
          {Map.update(base_c, base, n, &max(&1, n)), child_c}

        [_ | _] when length(parts) >= 3 ->
          # Derived CV: parent.M — the parent is everything except the last segment
          last = List.last(parts)
          parent_id = parts |> Enum.drop(-1) |> Enum.join(".")
          m = String.to_integer(last)
          {base_c, Map.update(child_c, parent_id, m, &max(&1, m))}

        _ ->
          {base_c, child_c}
      end
    end)
  end

  defp deserialize_entry(entry_map) when is_map(entry_map) do
    %Entry{
      id: Map.get(entry_map, "id", ""),
      parent_ids: Map.get(entry_map, "parent_ids", []),
      origin: deserialize_origin(Map.get(entry_map, "origin")),
      contributions:
        entry_map
        |> Map.get("contributions", [])
        |> Enum.map(&deserialize_contribution/1),
      deleted: deserialize_deletion(Map.get(entry_map, "deleted"))
    }
  end

  defp deserialize_origin(nil), do: nil

  defp deserialize_origin(o) when is_map(o) do
    %Origin{
      source: Map.get(o, "source", ""),
      location: Map.get(o, "location", ""),
      timestamp: Map.get(o, "timestamp"),
      meta: Map.get(o, "meta", %{})
    }
  end

  defp deserialize_contribution(c) when is_map(c) do
    %Contribution{
      source: Map.get(c, "source", ""),
      tag: Map.get(c, "tag", ""),
      meta: Map.get(c, "meta", %{})
    }
  end

  defp deserialize_deletion(nil), do: nil

  defp deserialize_deletion(d) when is_map(d) do
    %DeletionRecord{
      source: Map.get(d, "source", ""),
      reason: Map.get(d, "reason", ""),
      meta: Map.get(d, "meta", %{})
    }
  end
end
