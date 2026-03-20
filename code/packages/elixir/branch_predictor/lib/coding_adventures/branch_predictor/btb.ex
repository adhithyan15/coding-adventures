defmodule CodingAdventures.BranchPredictor.BTB do
  @moduledoc """
  Branch Target Buffer (BTB) — caching where branches go.

  ## The problem

  The branch predictor answers "WILL this branch be taken?"
  The BTB answers "WHERE does it go?"

  Both are needed for high-performance fetch. Without a BTB, even a perfect
  direction predictor would cause a 1-cycle bubble: the predictor says "taken"
  in the fetch stage, but the target address isn't known until decode (when
  the instruction's immediate field is extracted). With a BTB, the target
  is available in the SAME cycle as the prediction, enabling zero-bubble
  fetch redirection.

  ## How it works in the pipeline

      Cycle 1 (Fetch):
          1. Read PC
          2. Direction predictor: "taken" or "not taken"?
          3. BTB lookup: if "taken", where does it go?
          4. Redirect fetch to target (BTB hit) or PC+4 (not taken / BTB miss)

      Cycle 2+ (Decode, Execute, ...):
          Branch is decoded and eventually resolved.
          If BTB was wrong -> flush pipeline and update BTB.

  ## Organization

  This implementation uses a direct-mapped cache indexed by `rem(pc, size)`.
  Each entry stores:

  - `tag` — the full PC (for detecting aliasing conflicts)
  - `target` — the branch target address
  - `branch_type` — metadata ("conditional", "unconditional", "call", "return")

  On lookup: check tag match. Miss if no entry or tag mismatch.
  On update: overwrite the entry at the computed index (direct-mapped eviction).

  ## Eviction policy

  Direct-mapped: new entries always replace the old entry at the same index.
  This means a BTB miss is guaranteed when:
  - First encounter of a branch (compulsory miss)
  - Two frequently-used branches alias to the same index (conflict miss)

  ## Real-world BTB sizes

  - Intel Skylake: 4096 entries (L1 BTB) + 4096 entries (L2 BTB)
  - ARM Cortex-A72: 64 entries (micro BTB) + 4096 entries (main BTB)
  - AMD Zen 2: 512 entries (L1 BTB) + 7168 entries (L2 BTB)

  ## Immutability

  Since Elixir data is immutable, the BTB entries are stored in a map.
  Every lookup and update returns a new BTB struct. The entries map uses
  the index as key and a `%{tag, target, branch_type}` map as value.
  """

  defstruct size: 256, entries: %{}, lookups: 0, hits: 0, misses: 0

  @type entry :: %{tag: non_neg_integer(), target: non_neg_integer(), branch_type: String.t()}

  @type t :: %__MODULE__{
          size: pos_integer(),
          entries: %{non_neg_integer() => entry()},
          lookups: non_neg_integer(),
          hits: non_neg_integer(),
          misses: non_neg_integer()
        }

  @doc """
  Create a new Branch Target Buffer.

  ## Options

  - `:size` — number of entries in the BTB (default: 256). Should be a
    power of 2 for efficient hardware implementation.

  ## Examples

      iex> btb = CodingAdventures.BranchPredictor.BTB.new()
      iex> btb.size
      256

      iex> btb = CodingAdventures.BranchPredictor.BTB.new(size: 64)
      iex> btb.size
      64
  """
  def new(opts \\ []) do
    size = Keyword.get(opts, :size, 256)
    %__MODULE__{size: size}
  end

  @doc """
  Look up the predicted target for a branch at `pc`.

  Returns `{target | nil, btb}` where:
  - `target` is the cached target address on a hit
  - `nil` on a miss (entry not valid or tag mismatch)
  - `btb` is the updated BTB with incremented stats counters

  A miss occurs when:
  - The entry at this index has never been written (compulsory miss)
  - The entry's tag doesn't match the PC (aliasing conflict miss)

  ## Examples

      iex> btb = CodingAdventures.BranchPredictor.BTB.new()
      iex> {target, _btb} = CodingAdventures.BranchPredictor.BTB.lookup(btb, 0x100)
      iex> target
      nil
  """
  def lookup(%__MODULE__{} = btb, pc) do
    index = index(btb, pc)
    btb = %{btb | lookups: btb.lookups + 1}

    case Map.get(btb.entries, index) do
      %{tag: ^pc, target: target} ->
        {target, %{btb | hits: btb.hits + 1}}

      _ ->
        {nil, %{btb | misses: btb.misses + 1}}
    end
  end

  @doc """
  Record a branch target after execution.

  Writes the target and metadata into the BTB. If another branch was
  occupying this index (aliasing), it gets evicted — this is the
  direct-mapped eviction policy.

  ## Parameters

  - `btb` — the current BTB struct
  - `pc` — the program counter of the branch instruction
  - `target` — the actual target address of the branch
  - `branch_type` — the kind of branch (default: "conditional")

  ## Examples

      iex> btb = CodingAdventures.BranchPredictor.BTB.new()
      iex> btb = CodingAdventures.BranchPredictor.BTB.update(btb, 0x100, 0x200)
      iex> {target, _btb} = CodingAdventures.BranchPredictor.BTB.lookup(btb, 0x100)
      iex> target
      0x200
  """
  def update(%__MODULE__{} = btb, pc, target, branch_type \\ "conditional") do
    index = index(btb, pc)

    entry = %{tag: pc, target: target, branch_type: branch_type}
    %{btb | entries: Map.put(btb.entries, index, entry)}
  end

  @doc """
  Inspect the BTB entry for a given PC (for testing/debugging).

  Returns the entry map if it's valid and the tag matches, nil otherwise.
  Does NOT update stats counters (this is a debug/inspection function).

  ## Examples

      iex> btb = CodingAdventures.BranchPredictor.BTB.new()
      iex> btb = CodingAdventures.BranchPredictor.BTB.update(btb, 0x100, 0x200)
      iex> CodingAdventures.BranchPredictor.BTB.get_entry(btb, 0x100)
      %{tag: 0x100, target: 0x200, branch_type: "conditional"}
  """
  def get_entry(%__MODULE__{} = btb, pc) do
    index = index(btb, pc)

    case Map.get(btb.entries, index) do
      %{tag: ^pc} = entry -> entry
      _ -> nil
    end
  end

  @doc """
  BTB hit rate as a percentage (0.0 to 100.0).

  Returns 0.0 if no lookups have been performed.
  """
  def hit_rate(%__MODULE__{lookups: 0}), do: 0.0

  def hit_rate(%__MODULE__{} = btb) do
    btb.hits / btb.lookups * 100.0
  end

  @doc """
  Reset all BTB state — entries and statistics.
  """
  def reset(%__MODULE__{} = btb) do
    %{btb | entries: %{}, lookups: 0, hits: 0, misses: 0}
  end

  # ── Private helpers ──────────────────────────────────────────────────────

  defp index(%__MODULE__{size: size}, pc) do
    rem(pc, size)
  end
end
