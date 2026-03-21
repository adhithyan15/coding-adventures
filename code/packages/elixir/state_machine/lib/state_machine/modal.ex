defmodule CodingAdventures.StateMachine.Modal do
  @moduledoc """
  Modal State Machine — multiple sub-machines with mode switching.

  ## What is a Modal State Machine?

  A modal state machine is a collection of named sub-machines (modes), each
  a DFA, with transitions that switch between them. When a mode switch
  occurs, the active sub-machine changes.

  Think of it like a text editor with Normal, Insert, and Visual modes. Each
  mode handles keystrokes differently, and certain keys switch between modes.

  ## Why modal machines matter

  The most important use case is **context-sensitive tokenization**. Consider
  HTML: the characters `p > .foo { color: red; }` mean completely different
  things depending on whether they appear inside a `<style>` tag (CSS) or
  in normal text. A single set of token rules cannot handle both contexts.

  A modal state machine solves this: the HTML tokenizer has modes like
  DATA, TAG_OPEN, SCRIPT_DATA, and STYLE_DATA. Each mode has its own DFA
  with its own token rules. Certain tokens (like seeing `<style>`) trigger
  a mode switch.

  This is how real browser engines tokenize HTML, and it is the key
  abstraction that the grammar-tools lexer needs to support HTML, Markdown,
  and other context-sensitive languages.

  ## Connection to the Chomsky Hierarchy

  A single DFA recognizes regular languages (Type 3). A modal state machine
  is more powerful: it can track context (which mode am I in?) and switch
  rules accordingly. This moves us toward context-sensitive languages
  (Type 1), though a modal machine is still not as powerful as a full
  linear-bounded automaton.

  In practice, modal machines + pushdown automata cover the vast majority
  of real-world parsing needs.

  ## Elixir Design

  Since Elixir is immutable, the Modal struct carries the full configuration
  (current mode, all sub-machine states, mode trace). Every operation returns
  a new struct. Mode switches reset the target DFA to its initial state.
  """

  alias CodingAdventures.StateMachine.DFA

  defmodule ModeTransitionRecord do
    @moduledoc """
    Record of a mode switch event.

    Captures which mode we switched from and to, and what triggered it.
    """
    defstruct [:from_mode, :trigger, :to_mode]

    @type t :: %__MODULE__{
            from_mode: String.t(),
            trigger: String.t(),
            to_mode: String.t()
          }
  end

  defstruct [
    :modes,
    :mode_transitions,
    :initial_mode,
    :current_mode,
    :mode_trace
  ]

  @type t :: %__MODULE__{
          modes: %{String.t() => DFA.t()},
          mode_transitions: %{{String.t(), String.t()} => String.t()},
          initial_mode: String.t(),
          current_mode: String.t(),
          mode_trace: [ModeTransitionRecord.t()]
        }

  @doc """
  Create a new Modal State Machine.

  ## Parameters

  - `modes` — a map from mode names to DFA sub-machines
  - `mode_transitions` — a map from `{current_mode, trigger}` to the
    name of the mode to switch to
  - `initial_mode` — the name of the starting mode

  ## Returns

  `{:ok, modal}` on success, `{:error, reason}` on validation failure.

  ## Examples

      iex> alias CodingAdventures.StateMachine.{DFA, Modal}
      iex> {:ok, dfa1} = DFA.new(
      ...>   MapSet.new(["a"]), MapSet.new(["x"]),
      ...>   %{{"a", "x"} => "a"}, "a", MapSet.new(["a"])
      ...> )
      iex> {:ok, dfa2} = DFA.new(
      ...>   MapSet.new(["b"]), MapSet.new(["y"]),
      ...>   %{{"b", "y"} => "b"}, "b", MapSet.new(["b"])
      ...> )
      iex> {:ok, modal} = Modal.new(
      ...>   %{"mode1" => dfa1, "mode2" => dfa2},
      ...>   %{{"mode1", "switch"} => "mode2", {"mode2", "switch"} => "mode1"},
      ...>   "mode1"
      ...> )
      iex> modal.current_mode
      "mode1"
  """
  def new(modes, mode_transitions, initial_mode) do
    with :ok <- validate_modes_non_empty(modes),
         :ok <- validate_initial_mode(initial_mode, modes),
         :ok <- validate_mode_transitions(mode_transitions, modes) do
      {:ok,
       %__MODULE__{
         modes: modes,
         mode_transitions: mode_transitions,
         initial_mode: initial_mode,
         current_mode: initial_mode,
         mode_trace: []
       }}
    end
  end

  @doc """
  Get the DFA for the current mode.
  """
  def active_machine(%__MODULE__{} = modal) do
    Map.fetch!(modal.modes, modal.current_mode)
  end

  @doc """
  Process an input event in the current mode's DFA.

  Delegates to the active DFA's `process/2` function. The updated DFA
  is stored back in the modes map.

  ## Parameters

  - `modal` — the current modal state machine
  - `event` — an input symbol for the current mode's DFA

  ## Returns

  `{:ok, new_modal}` on success, `{:error, reason}` on failure.
  """
  def process(%__MODULE__{} = modal, event) do
    current_dfa = Map.fetch!(modal.modes, modal.current_mode)

    case DFA.process(current_dfa, event) do
      {:ok, new_dfa} ->
        new_modes = Map.put(modal.modes, modal.current_mode, new_dfa)
        {:ok, %{modal | modes: new_modes}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Switch to a different mode based on a trigger event.

  Looks up `{current_mode, trigger}` in the mode transitions. If found,
  switches to the target mode and resets its DFA to the initial state.

  ## Parameters

  - `modal` — the current modal state machine
  - `trigger` — the event that triggers the mode switch

  ## Returns

  `{:ok, new_modal}` on success, `{:error, reason}` if no transition exists.
  """
  def switch_mode(%__MODULE__{} = modal, trigger) do
    key = {modal.current_mode, trigger}

    case Map.get(modal.mode_transitions, key) do
      nil ->
        {:error,
         "No mode transition for (mode='#{modal.current_mode}', trigger='#{trigger}')"}

      new_mode ->
        old_mode = modal.current_mode

        # Reset the target mode's DFA to its initial state
        target_dfa = Map.fetch!(modal.modes, new_mode)
        reset_dfa = DFA.reset(target_dfa)
        new_modes = Map.put(modal.modes, new_mode, reset_dfa)

        record = %ModeTransitionRecord{
          from_mode: old_mode,
          trigger: trigger,
          to_mode: new_mode
        }

        {:ok,
         %{modal |
           modes: new_modes,
           current_mode: new_mode,
           mode_trace: modal.mode_trace ++ [record]
         }}
    end
  end

  @doc """
  Reset to initial mode and reset all sub-machines.

  Returns a new Modal struct with all DFAs reset to their initial states
  and the mode trace cleared.
  """
  def reset(%__MODULE__{} = modal) do
    new_modes =
      modal.modes
      |> Enum.map(fn {name, dfa} -> {name, DFA.reset(dfa)} end)
      |> Map.new()

    %{modal |
      modes: new_modes,
      current_mode: modal.initial_mode,
      mode_trace: []
    }
  end

  # === Private validation helpers ===

  defp validate_modes_non_empty(modes) when map_size(modes) == 0 do
    {:error, "At least one mode must be provided"}
  end

  defp validate_modes_non_empty(_modes), do: :ok

  defp validate_initial_mode(initial_mode, modes) do
    if Map.has_key?(modes, initial_mode) do
      :ok
    else
      {:error, "Initial mode '#{initial_mode}' is not in the modes dict"}
    end
  end

  defp validate_mode_transitions(mode_transitions, modes) do
    Enum.reduce_while(mode_transitions, :ok, fn {{from_mode, _trigger}, to_mode}, :ok ->
      cond do
        not Map.has_key?(modes, from_mode) ->
          {:halt, {:error, "Mode transition source '#{from_mode}' is not a valid mode"}}

        not Map.has_key?(modes, to_mode) ->
          {:halt, {:error, "Mode transition target '#{to_mode}' is not a valid mode"}}

        true ->
          {:cont, :ok}
      end
    end)
  end
end
