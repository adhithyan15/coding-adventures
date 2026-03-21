defmodule CodingAdventures.StateMachine.ModalTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.StateMachine.DFA
  alias CodingAdventures.StateMachine.Modal
  alias CodingAdventures.StateMachine.Modal.ModeTransitionRecord

  # === Fixture helpers ===

  # Build a simplified HTML tokenizer with two modes: data and tag.
  #
  # Data mode: processes "char" (stays in text) and "open_angle" (enters tag_start)
  # Tag mode: processes "char" (stays in name) and "close_angle" (enters done)
  #
  # Mode transitions: "enter_tag" switches data -> tag, "exit_tag" switches tag -> data
  defp html_tokenizer do
    {:ok, data_mode} =
      DFA.new(
        MapSet.new(["text", "tag_start"]),
        MapSet.new(["char", "open_angle"]),
        %{
          {"text", "char"} => "text",
          {"text", "open_angle"} => "tag_start",
          {"tag_start", "char"} => "text",
          {"tag_start", "open_angle"} => "tag_start"
        },
        "text",
        MapSet.new(["text"])
      )

    {:ok, tag_mode} =
      DFA.new(
        MapSet.new(["name", "done"]),
        MapSet.new(["char", "close_angle"]),
        %{
          {"name", "char"} => "name",
          {"name", "close_angle"} => "done",
          {"done", "char"} => "name",
          {"done", "close_angle"} => "done"
        },
        "name",
        MapSet.new(["done"])
      )

    Modal.new(
      %{"data" => data_mode, "tag" => tag_mode},
      %{
        {"data", "enter_tag"} => "tag",
        {"tag", "exit_tag"} => "data"
      },
      "data"
    )
  end

  # Build a simple two-mode machine for basic testing.
  defp two_mode do
    {:ok, mode_a} =
      DFA.new(
        MapSet.new(["s0", "s1"]),
        MapSet.new(["x"]),
        %{{"s0", "x"} => "s1", {"s1", "x"} => "s0"},
        "s0",
        MapSet.new(["s1"])
      )

    {:ok, mode_b} =
      DFA.new(
        MapSet.new(["t0", "t1"]),
        MapSet.new(["y"]),
        %{{"t0", "y"} => "t1", {"t1", "y"} => "t0"},
        "t0",
        MapSet.new(["t1"])
      )

    Modal.new(
      %{"alpha" => mode_a, "beta" => mode_b},
      %{
        {"alpha", "go_beta"} => "beta",
        {"beta", "go_alpha"} => "alpha"
      },
      "alpha"
    )
  end

  # === Construction tests ===

  describe "new/3 — construction and validation" do
    test "creates a valid modal machine" do
      assert {:ok, modal} = html_tokenizer()
      assert modal.current_mode == "data"
      assert modal.mode_trace == []
    end

    test "rejects empty modes" do
      assert {:error, msg} = Modal.new(%{}, %{}, "data")
      assert msg =~ "At least one mode"
    end

    test "rejects initial mode not in modes" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0"]),
          MapSet.new(["a"]),
          %{{"q0", "a"} => "q0"},
          "q0",
          MapSet.new()
        )

      assert {:error, msg} = Modal.new(%{"m1" => dfa}, %{}, "m99")
      assert msg =~ "m99"
    end

    test "rejects transition from unknown mode" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0"]),
          MapSet.new(["a"]),
          %{{"q0", "a"} => "q0"},
          "q0",
          MapSet.new()
        )

      assert {:error, msg} =
               Modal.new(
                 %{"m1" => dfa},
                 %{{"m99", "trigger"} => "m1"},
                 "m1"
               )

      assert msg =~ "m99"
    end

    test "rejects transition to unknown mode" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0"]),
          MapSet.new(["a"]),
          %{{"q0", "a"} => "q0"},
          "q0",
          MapSet.new()
        )

      assert {:error, msg} =
               Modal.new(
                 %{"m1" => dfa},
                 %{{"m1", "trigger"} => "m99"},
                 "m1"
               )

      assert msg =~ "m99"
    end
  end

  # === Active machine tests ===

  describe "active_machine/1" do
    test "returns the DFA for the current mode" do
      {:ok, modal} = html_tokenizer()
      dfa = Modal.active_machine(modal)
      assert dfa.initial == "text"
    end

    test "changes after mode switch" do
      {:ok, modal} = html_tokenizer()
      {:ok, modal} = Modal.switch_mode(modal, "enter_tag")
      dfa = Modal.active_machine(modal)
      assert dfa.initial == "name"
    end
  end

  # === Processing tests ===

  describe "process/2" do
    test "processes event in current mode" do
      {:ok, modal} = html_tokenizer()
      {:ok, modal} = Modal.process(modal, "char")
      dfa = Modal.active_machine(modal)
      assert dfa.current == "text"
    end

    test "transitions within current mode" do
      {:ok, modal} = html_tokenizer()
      {:ok, modal} = Modal.process(modal, "open_angle")
      dfa = Modal.active_machine(modal)
      assert dfa.current == "tag_start"
    end

    test "rejects event not in current mode's alphabet" do
      {:ok, modal} = html_tokenizer()
      # "close_angle" is in tag mode, not data mode
      assert {:error, _msg} = Modal.process(modal, "close_angle")
    end

    test "processes multiple events in same mode" do
      {:ok, modal} = two_mode()
      {:ok, modal} = Modal.process(modal, "x")
      dfa = Modal.active_machine(modal)
      assert dfa.current == "s1"

      {:ok, modal} = Modal.process(modal, "x")
      dfa = Modal.active_machine(modal)
      assert dfa.current == "s0"
    end
  end

  # === Mode switching tests ===

  describe "switch_mode/2" do
    test "switches to target mode" do
      {:ok, modal} = html_tokenizer()
      {:ok, modal} = Modal.switch_mode(modal, "enter_tag")
      assert modal.current_mode == "tag"
    end

    test "resets target mode DFA" do
      {:ok, modal} = two_mode()
      # Process in alpha mode
      {:ok, modal} = Modal.process(modal, "x")
      assert Modal.active_machine(modal).current == "s1"

      # Switch to beta and back to alpha
      {:ok, modal} = Modal.switch_mode(modal, "go_beta")
      {:ok, modal} = Modal.switch_mode(modal, "go_alpha")

      # Alpha mode should be reset
      assert Modal.active_machine(modal).current == "s0"
    end

    test "records mode transition" do
      {:ok, modal} = html_tokenizer()
      {:ok, modal} = Modal.switch_mode(modal, "enter_tag")
      assert length(modal.mode_trace) == 1

      [record] = modal.mode_trace
      assert %ModeTransitionRecord{} = record
      assert record.from_mode == "data"
      assert record.trigger == "enter_tag"
      assert record.to_mode == "tag"
    end

    test "accumulates mode trace" do
      {:ok, modal} = html_tokenizer()
      {:ok, modal} = Modal.switch_mode(modal, "enter_tag")
      {:ok, modal} = Modal.switch_mode(modal, "exit_tag")
      {:ok, modal} = Modal.switch_mode(modal, "enter_tag")
      assert length(modal.mode_trace) == 3
    end

    test "rejects unknown trigger" do
      {:ok, modal} = html_tokenizer()
      assert {:error, msg} = Modal.switch_mode(modal, "unknown_trigger")
      assert msg =~ "No mode transition"
    end

    test "rejects trigger valid for wrong mode" do
      {:ok, modal} = html_tokenizer()
      # "exit_tag" is only valid from "tag" mode
      assert {:error, _msg} = Modal.switch_mode(modal, "exit_tag")
    end

    test "can process events after switching" do
      {:ok, modal} = html_tokenizer()
      {:ok, modal} = Modal.switch_mode(modal, "enter_tag")
      {:ok, modal} = Modal.process(modal, "char")
      dfa = Modal.active_machine(modal)
      assert dfa.current == "name"
    end

    test "round trip mode switching" do
      {:ok, modal} = two_mode()
      {:ok, modal} = Modal.process(modal, "x")
      {:ok, modal} = Modal.switch_mode(modal, "go_beta")
      {:ok, modal} = Modal.process(modal, "y")
      {:ok, modal} = Modal.switch_mode(modal, "go_alpha")
      {:ok, modal} = Modal.process(modal, "x")

      assert modal.current_mode == "alpha"
      # Alpha was reset on switch, so after one "x" it should be in s1
      assert Modal.active_machine(modal).current == "s1"
    end
  end

  # === Reset tests ===

  describe "reset/1" do
    test "resets to initial mode" do
      {:ok, modal} = html_tokenizer()
      {:ok, modal} = Modal.switch_mode(modal, "enter_tag")
      modal = Modal.reset(modal)
      assert modal.current_mode == "data"
    end

    test "clears mode trace" do
      {:ok, modal} = html_tokenizer()
      {:ok, modal} = Modal.switch_mode(modal, "enter_tag")
      modal = Modal.reset(modal)
      assert modal.mode_trace == []
    end

    test "resets all sub-machines" do
      {:ok, modal} = two_mode()
      {:ok, modal} = Modal.process(modal, "x")
      {:ok, modal} = Modal.switch_mode(modal, "go_beta")
      {:ok, modal} = Modal.process(modal, "y")

      modal = Modal.reset(modal)

      # Both modes should be in their initial state
      assert modal.modes["alpha"].current == "s0"
      assert modal.modes["beta"].current == "t0"
    end
  end

  # === Edge case tests ===

  describe "edge cases" do
    test "single mode machine" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0"]),
          MapSet.new(["a"]),
          %{{"q0", "a"} => "q0"},
          "q0",
          MapSet.new(["q0"])
        )

      {:ok, modal} = Modal.new(%{"only" => dfa}, %{}, "only")
      assert modal.current_mode == "only"

      {:ok, modal} = Modal.process(modal, "a")
      assert modal.current_mode == "only"
    end

    test "self-transition mode" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0"]),
          MapSet.new(["a"]),
          %{{"q0", "a"} => "q0"},
          "q0",
          MapSet.new(["q0"])
        )

      {:ok, modal} =
        Modal.new(
          %{"m1" => dfa},
          %{{"m1", "restart"} => "m1"},
          "m1"
        )

      {:ok, modal} = Modal.switch_mode(modal, "restart")
      assert modal.current_mode == "m1"
      assert length(modal.mode_trace) == 1
    end

    test "three modes with cyclic transitions" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0"]),
          MapSet.new(["a"]),
          %{{"q0", "a"} => "q0"},
          "q0",
          MapSet.new(["q0"])
        )

      {:ok, modal} =
        Modal.new(
          %{"m1" => dfa, "m2" => dfa, "m3" => dfa},
          %{
            {"m1", "next"} => "m2",
            {"m2", "next"} => "m3",
            {"m3", "next"} => "m1"
          },
          "m1"
        )

      {:ok, modal} = Modal.switch_mode(modal, "next")
      assert modal.current_mode == "m2"
      {:ok, modal} = Modal.switch_mode(modal, "next")
      assert modal.current_mode == "m3"
      {:ok, modal} = Modal.switch_mode(modal, "next")
      assert modal.current_mode == "m1"
    end
  end
end
