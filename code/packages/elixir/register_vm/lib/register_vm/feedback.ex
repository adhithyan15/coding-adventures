defmodule CodingAdventures.RegisterVM.Feedback do
  @moduledoc """
  Feedback vector management — per-function type observation recording.

  ## What is a feedback vector?

  When a JIT compiler like V8's TurboFan wants to specialize code for
  integers, it needs to know: "has this + instruction EVER seen a non-integer?"
  The feedback vector answers that question.

  Every function has a feedback vector: a flat array of slots, one per
  "dynamic dispatch site" (arithmetic op, property load, call site). Each
  slot tracks what types appeared there at runtime.

  ## The Four-State Machine per Slot

  Each slot progresses through exactly four states, never going backwards:

      :uninitialized
          |
          | (first type pair observed)
          v
      {:monomorphic, [type_pair]}
          |
          | (different type pair observed)
          v
      {:polymorphic, [type_pair1, type_pair2, ...]}   (2–4 distinct pairs)
          |
          | (5th distinct pair observed)
          v
      :megamorphic   ← terminal state, never changes

  ## Why this matters

  - **Monomorphic**: 1 type pair seen → JIT can emit a type check + fast path
  - **Polymorphic**: 2–4 type pairs → JIT emits a small inline cache dispatch
  - **Megamorphic**: too many types → JIT falls back to a generic slow path

  This state machine is a direct simplification of V8's IC (inline cache)
  system. Real V8 uses a richer representation (hidden class maps, smi ranges,
  etc.) but the core state machine is the same.

  ## State transition diagram

      State           + New type pair     → Next state
      --------------- + ---------------   → --------------------
      :uninitialized  + any pair          → {:monomorphic, [pair]}
      {:mono, [p]}    + same p            → {:monomorphic, [p]}   (no-op)
      {:mono, [p]}    + different pair q  → {:polymorphic, [q, p]}
      {:poly, ps}     + pair in ps        → {:polymorphic, ps}    (no-op)
      {:poly, ps}     + new pair, len < 4 → {:polymorphic, [new | ps]}
      {:poly, ps}     + new pair, len >= 4→ :megamorphic
      :megamorphic    + any               → :megamorphic          (terminal)
  """

  # ---------------------------------------------------------------------------
  # Vector construction
  # ---------------------------------------------------------------------------

  @doc """
  Creates a new feedback vector of the given size.

  All slots start as `:uninitialized` — no types have been observed yet.

  ## Examples

      iex> Feedback.new_vector(3)
      [:uninitialized, :uninitialized, :uninitialized]
  """
  def new_vector(size) when is_integer(size) and size >= 0 do
    List.duplicate(:uninitialized, size)
  end

  # ---------------------------------------------------------------------------
  # Type classification
  # ---------------------------------------------------------------------------

  @doc """
  Classifies a runtime value into one of the feedback type atoms.

  This coarse-grained classification is sufficient for the JIT's purposes:
  it tells us whether a slot is "always integer", "always string", etc.

  ## Type atoms

  - `:integer`  — Elixir integer (any precision)
  - `:float`    — Elixir float
  - `:string`   — Elixir binary (UTF-8 string)
  - `:boolean`  — `true` or `false`
  - `:null`     — `nil` (JavaScript null)
  - `:undefined`— `:undefined` atom (JavaScript undefined)
  - `:object`   — Elixir map (used to represent JS objects)
  - `:array`    — Elixir list (used to represent JS arrays)
  - `:function` — a `{:function, code, context}` tuple

  ## Examples

      iex> Feedback.value_type(42)
      :integer
      iex> Feedback.value_type("hello")
      :string
      iex> Feedback.value_type(%{"x" => 1})
      :object
  """
  def value_type(value) do
    cond do
      is_integer(value) -> :integer
      is_float(value) -> :float
      is_binary(value) -> :string
      is_boolean(value) -> :boolean
      is_nil(value) -> :null
      value == :undefined -> :undefined
      is_map(value) -> :object
      is_list(value) -> :array
      is_tuple(value) and tuple_size(value) == 3 and elem(value, 0) == :function -> :function
      true -> :unknown
    end
  end

  # ---------------------------------------------------------------------------
  # Recording type observations
  # ---------------------------------------------------------------------------

  @doc """
  Records the types of both operands for a binary arithmetic operation.

  Returns the updated feedback vector. The slot at `slot_idx` is updated
  with the type pair {left_type, right_type}.

  ## Examples

      iex> v = Feedback.new_vector(2)
      iex> v2 = Feedback.record_binary_op(v, 0, 10, 20)
      iex> Enum.at(v2, 0)
      {:monomorphic, [{:integer, :integer}]}
  """
  def record_binary_op(vector, slot_idx, left, right) do
    left_type = value_type(left)
    right_type = value_type(right)
    type_pair = {left_type, right_type}
    update_vector(vector, slot_idx, type_pair)
  end

  @doc """
  Records the "hidden class" of an object being accessed for a property load.

  The hidden class ID is a hash of the object's sorted key set. Two objects
  with the same set of keys get the same hidden class ID. This mirrors how
  V8 assigns "shapes" (hidden classes / maps) to objects.

  A JIT uses this to detect monomorphic property accesses: if every time
  LdaNamedProperty executes, the object has the same hidden class, the JIT
  can hard-code the property offset rather than doing a hash lookup.

  ## Examples

      iex> v = Feedback.new_vector(1)
      iex> obj = %{"x" => 1, "y" => 2}
      iex> v2 = Feedback.record_property_load(v, 0, obj)
      iex> match?({:monomorphic, _}, Enum.at(v2, 0))
      true
  """
  def record_property_load(vector, slot_idx, object) when is_map(object) do
    hidden_class_id = hidden_class_id(object)
    update_vector(vector, slot_idx, hidden_class_id)
  end

  def record_property_load(vector, _slot_idx, _non_map), do: vector

  @doc """
  Records what type of value was called at a call site.

  Monomorphic call sites (always the same function) can be inlined by the JIT.

  ## Examples

      iex> v = Feedback.new_vector(1)
      iex> callee = {:function, %CodeObject{}, nil}
      iex> v2 = Feedback.record_call_site(v, 0, callee)
      iex> match?({:monomorphic, _}, Enum.at(v2, 0))
      true
  """
  def record_call_site(vector, slot_idx, callee) do
    callee_type = value_type(callee)
    update_vector(vector, slot_idx, callee_type)
  end

  # ---------------------------------------------------------------------------
  # Slot state machine
  # ---------------------------------------------------------------------------

  @doc """
  Advances one feedback slot through the state machine given a new observation.

  This is the core state machine function. Each call either:
  - Stays in the same state (already seen this type)
  - Advances to the next state (new type observed)
  - Stays in :megamorphic forever (terminal)

  ## State transitions

      :uninitialized            → {:monomorphic, [new_pair]}
      {:monomorphic, [p]}       → {:monomorphic, [p]}         if p == new_pair
      {:monomorphic, [p]}       → {:polymorphic, [new_pair, p]} if p != new_pair
      {:polymorphic, ps}        → {:polymorphic, ps}          if new_pair in ps
      {:polymorphic, ps} (len<4)→ {:polymorphic, [new_pair|ps]}
      {:polymorphic, ps} (len≥4)→ :megamorphic
      :megamorphic              → :megamorphic
  """
  def update_slot(:uninitialized, new_type) do
    # First observation: transition to monomorphic
    {:monomorphic, [new_type]}
  end

  def update_slot({:monomorphic, [existing]}, new_type) when existing == new_type do
    # Same type again: stay monomorphic (deduplication)
    {:monomorphic, [existing]}
  end

  def update_slot({:monomorphic, [existing]}, new_type) do
    # New type on previously monomorphic site: go polymorphic
    {:polymorphic, [new_type, existing]}
  end

  def update_slot({:polymorphic, types} = slot, new_type) do
    if new_type in types do
      # Already seen this type: no change
      slot
    else
      # New type: check if we exceed the polymorphic limit (4 types)
      if length(types) >= 4 do
        :megamorphic
      else
        {:polymorphic, [new_type | types]}
      end
    end
  end

  def update_slot(:megamorphic, _new_type) do
    # Terminal state: once megamorphic, always megamorphic
    :megamorphic
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # Updates a single slot in the vector at position `slot_idx`.
  # Returns the updated vector list.
  defp update_vector(vector, slot_idx, type_pair) do
    if slot_idx >= 0 and slot_idx < length(vector) do
      old_slot = Enum.at(vector, slot_idx)
      new_slot = update_slot(old_slot, type_pair)
      List.update_at(vector, slot_idx, fn _ -> new_slot end)
    else
      # Out-of-bounds slot: ignore silently (can happen with nil feedback_slot)
      vector
    end
  end

  # Computes a stable integer "hidden class ID" for an object.
  # Two objects with exactly the same keys will get the same ID.
  # This is a simple hash — not cryptographically secure, just enough
  # for the IC state machine.
  defp hidden_class_id(map) when is_map(map) do
    sorted_keys = map |> Map.keys() |> Enum.sort()
    :erlang.phash2(sorted_keys)
  end
end
