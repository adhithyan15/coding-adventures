defmodule CodingAdventures.StateMachine.PDATest do
  use ExUnit.Case, async: true

  alias CodingAdventures.StateMachine.PDA
  alias CodingAdventures.StateMachine.PDA.Transition
  alias CodingAdventures.StateMachine.PDA.TraceEntry

  # === Fixture helpers ===

  # PDA for balanced parentheses.
  #
  # This is the canonical PDA example. It pushes a "(" onto the stack for
  # each "(" in the input, and pops for each ")". The string is accepted
  # if the stack is back to just the bottom marker "$" at end of input.
  #
  # Transitions:
  # - In q0, reading "(", stack has "$": push "(" above "$"
  # - In q0, reading "(", stack has "(": push another "("
  # - In q0, reading ")", stack has "(": pop the "("
  # - In q0, at end of input, stack has "$": move to accept (pop "$")
  defp balanced_parens do
    PDA.new(
      MapSet.new(["q0", "accept"]),
      MapSet.new(["(", ")"]),
      MapSet.new(["(", "$"]),
      [
        %Transition{source: "q0", event: "(", stack_read: "$", target: "q0", stack_push: ["$", "("]},
        %Transition{source: "q0", event: "(", stack_read: "(", target: "q0", stack_push: ["(", "("]},
        %Transition{source: "q0", event: ")", stack_read: "(", target: "q0", stack_push: []},
        %Transition{source: "q0", event: nil, stack_read: "$", target: "accept", stack_push: []}
      ],
      "q0",
      "$",
      MapSet.new(["accept"])
    )
  end

  # PDA for a^n b^n (equal numbers of a's followed by b's).
  #
  # Transitions:
  # - Read "a": push "a" onto stack
  # - Read "b": pop "a" from stack
  # - At end: if stack is just "$", accept
  defp anbn do
    PDA.new(
      MapSet.new(["q0", "q1", "accept"]),
      MapSet.new(["a", "b"]),
      MapSet.new(["a", "$"]),
      [
        %Transition{source: "q0", event: "a", stack_read: "$", target: "q0", stack_push: ["$", "a"]},
        %Transition{source: "q0", event: "a", stack_read: "a", target: "q0", stack_push: ["a", "a"]},
        %Transition{source: "q0", event: "b", stack_read: "a", target: "q1", stack_push: []},
        %Transition{source: "q1", event: "b", stack_read: "a", target: "q1", stack_push: []},
        %Transition{source: "q1", event: nil, stack_read: "$", target: "accept", stack_push: []}
      ],
      "q0",
      "$",
      MapSet.new(["accept"])
    )
  end

  # === Construction tests ===

  describe "new/7 — construction and validation" do
    test "creates a valid PDA" do
      assert {:ok, pda} = balanced_parens()
      assert pda.current == "q0"
      assert pda.stack == ["$"]
    end

    test "rejects empty states" do
      assert {:error, msg} =
               PDA.new(
                 MapSet.new(),
                 MapSet.new(["a"]),
                 MapSet.new(["$"]),
                 [],
                 "q0",
                 "$",
                 MapSet.new()
               )

      assert msg =~ "non-empty"
    end

    test "rejects initial state not in states" do
      assert {:error, msg} =
               PDA.new(
                 MapSet.new(["q0"]),
                 MapSet.new(["a"]),
                 MapSet.new(["$"]),
                 [],
                 "q99",
                 "$",
                 MapSet.new()
               )

      assert msg =~ "q99"
    end

    test "rejects initial stack symbol not in stack alphabet" do
      assert {:error, msg} =
               PDA.new(
                 MapSet.new(["q0"]),
                 MapSet.new(["a"]),
                 MapSet.new(["$"]),
                 [],
                 "q0",
                 "Z",
                 MapSet.new()
               )

      assert msg =~ "Z"
    end

    test "rejects accepting states not in states" do
      assert {:error, msg} =
               PDA.new(
                 MapSet.new(["q0"]),
                 MapSet.new(["a"]),
                 MapSet.new(["$"]),
                 [],
                 "q0",
                 "$",
                 MapSet.new(["q99"])
               )

      assert msg =~ "q99"
    end

    test "rejects duplicate (non-deterministic) transitions" do
      assert {:error, msg} =
               PDA.new(
                 MapSet.new(["q0"]),
                 MapSet.new(["a"]),
                 MapSet.new(["$"]),
                 [
                   %Transition{source: "q0", event: "a", stack_read: "$", target: "q0", stack_push: []},
                   %Transition{source: "q0", event: "a", stack_read: "$", target: "q0", stack_push: ["$"]}
                 ],
                 "q0",
                 "$",
                 MapSet.new()
               )

      assert msg =~ "Duplicate"
    end

    test "starts with initial stack symbol" do
      {:ok, pda} = balanced_parens()
      assert pda.stack == ["$"]
    end

    test "starts with empty trace" do
      {:ok, pda} = balanced_parens()
      assert pda.trace == []
    end
  end

  # === Processing tests ===

  describe "process/2" do
    test "processes opening paren" do
      {:ok, pda} = balanced_parens()
      {:ok, pda} = PDA.process(pda, "(")
      assert pda.current == "q0"
      assert pda.stack == ["$", "("]
    end

    test "processes closing paren after opening" do
      {:ok, pda} = balanced_parens()
      {:ok, pda} = PDA.process(pda, "(")
      {:ok, pda} = PDA.process(pda, ")")
      assert pda.current == "q0"
      assert pda.stack == ["$"]
    end

    test "nested parens" do
      {:ok, pda} = balanced_parens()
      {:ok, pda} = PDA.process(pda, "(")
      {:ok, pda} = PDA.process(pda, "(")
      assert pda.stack == ["$", "(", "("]

      {:ok, pda} = PDA.process(pda, ")")
      assert pda.stack == ["$", "("]

      {:ok, pda} = PDA.process(pda, ")")
      assert pda.stack == ["$"]
    end

    test "records trace entries" do
      {:ok, pda} = balanced_parens()
      {:ok, pda} = PDA.process(pda, "(")
      assert length(pda.trace) == 1

      [entry] = pda.trace
      assert %TraceEntry{} = entry
      assert entry.source == "q0"
      assert entry.event == "("
      assert entry.target == "q0"
    end

    test "fails on unmatched close paren" do
      {:ok, pda} = balanced_parens()
      assert {:error, _msg} = PDA.process(pda, ")")
    end
  end

  # === Sequence processing tests ===

  describe "process_sequence/2" do
    test "balanced parens accepted" do
      {:ok, pda} = balanced_parens()
      {:ok, pda, trace} = PDA.process_sequence(pda, ["(", ")"])
      assert pda.current == "accept"
      assert length(trace) >= 2
    end

    test "nested balanced parens" do
      {:ok, pda} = balanced_parens()
      {:ok, pda, _trace} = PDA.process_sequence(pda, ["(", "(", ")", ")"])
      assert pda.current == "accept"
    end

    test "fails on unbalanced input" do
      {:ok, pda} = balanced_parens()
      assert {:error, _msg} = PDA.process_sequence(pda, [")", "("])
    end
  end

  # === Acceptance tests ===

  describe "accepts?/2" do
    test "balanced parens: ()" do
      {:ok, pda} = balanced_parens()
      assert PDA.accepts?(pda, ["(", ")"])
    end

    test "balanced parens: (())" do
      {:ok, pda} = balanced_parens()
      assert PDA.accepts?(pda, ["(", "(", ")", ")"])
    end

    test "balanced parens: ()()" do
      {:ok, pda} = balanced_parens()
      assert PDA.accepts?(pda, ["(", ")", "(", ")"])
    end

    test "balanced parens: empty is accepted" do
      {:ok, pda} = balanced_parens()
      assert PDA.accepts?(pda, [])
    end

    test "balanced parens: rejects (" do
      {:ok, pda} = balanced_parens()
      refute PDA.accepts?(pda, ["("])
    end

    test "balanced parens: rejects (()" do
      {:ok, pda} = balanced_parens()
      refute PDA.accepts?(pda, ["(", "(", ")"])
    end

    test "balanced parens: rejects )(" do
      {:ok, pda} = balanced_parens()
      refute PDA.accepts?(pda, [")", "("])
    end

    test "balanced parens: rejects )" do
      {:ok, pda} = balanced_parens()
      refute PDA.accepts?(pda, [")"])
    end

    test "a^n b^n: ab" do
      {:ok, pda} = anbn()
      assert PDA.accepts?(pda, ["a", "b"])
    end

    test "a^n b^n: aabb" do
      {:ok, pda} = anbn()
      assert PDA.accepts?(pda, ["a", "a", "b", "b"])
    end

    test "a^n b^n: aaabbb" do
      {:ok, pda} = anbn()
      assert PDA.accepts?(pda, ["a", "a", "a", "b", "b", "b"])
    end

    test "a^n b^n: rejects aab" do
      {:ok, pda} = anbn()
      refute PDA.accepts?(pda, ["a", "a", "b"])
    end

    test "a^n b^n: rejects abb" do
      {:ok, pda} = anbn()
      refute PDA.accepts?(pda, ["a", "b", "b"])
    end

    test "a^n b^n: rejects ba" do
      {:ok, pda} = anbn()
      refute PDA.accepts?(pda, ["b", "a"])
    end

    test "a^n b^n: rejects single a" do
      {:ok, pda} = anbn()
      refute PDA.accepts?(pda, ["a"])
    end

    test "accepts? does not modify PDA" do
      {:ok, pda} = balanced_parens()
      PDA.accepts?(pda, ["(", "(", ")", ")"])
      assert pda.current == "q0"
      assert pda.stack == ["$"]
    end

    test "deeply nested parens" do
      {:ok, pda} = balanced_parens()
      n = 20
      events = List.duplicate("(", n) ++ List.duplicate(")", n)
      assert PDA.accepts?(pda, events)
    end
  end

  # === Reset tests ===

  describe "reset/1" do
    test "resets to initial state" do
      {:ok, pda} = balanced_parens()
      {:ok, pda} = PDA.process(pda, "(")
      pda = PDA.reset(pda)
      assert pda.current == "q0"
      assert pda.stack == ["$"]
      assert pda.trace == []
    end
  end

  # === Stack top tests ===

  describe "stack_top/1" do
    test "initial stack top is initial symbol" do
      {:ok, pda} = balanced_parens()
      assert PDA.stack_top(pda) == "$"
    end

    test "stack top after push" do
      {:ok, pda} = balanced_parens()
      {:ok, pda} = PDA.process(pda, "(")
      assert PDA.stack_top(pda) == "("
    end
  end
end
