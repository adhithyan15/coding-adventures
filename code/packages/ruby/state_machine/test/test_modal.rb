# frozen_string_literal: true

require "test_helper"
require "set"

# ---------------------------------------------------------------------------
# Tests for the Modal State Machine implementation.
# ---------------------------------------------------------------------------
#
# These tests model a simplified HTML tokenizer with three modes:
#
# - DATA mode: reads characters and detects '<' for tag opening
# - TAG mode: reads tag name characters and detects '>' for tag closing
# - SCRIPT mode: reads raw characters until end-of-script
#
# Mode transitions:
#   data --enter_tag--> tag
#   tag --exit_tag--> data
#   tag --enter_script--> script
#   script --exit_script--> data
#
# This mirrors how real browser engines tokenize HTML. The key insight is
# that different contexts (data, tags, scripts) require completely different
# tokenization rules, and mode switching is how we handle that.
# ---------------------------------------------------------------------------

module CodingAdventures
  module StateMachine
    # === Modal State Machine Factory Methods ===

    def self.make_data_mode
      DFA.new(
        states: Set["text", "tag_detected"],
        alphabet: Set["char", "open_angle"],
        transitions: {
          ["text", "char"] => "text",
          ["text", "open_angle"] => "tag_detected",
          ["tag_detected", "char"] => "text",
          ["tag_detected", "open_angle"] => "tag_detected"
        },
        initial: "text",
        accepting: Set["text"]
      )
    end

    def self.make_tag_mode
      DFA.new(
        states: Set["reading_name", "tag_done"],
        alphabet: Set["char", "close_angle"],
        transitions: {
          ["reading_name", "char"] => "reading_name",
          ["reading_name", "close_angle"] => "tag_done",
          ["tag_done", "char"] => "reading_name",
          ["tag_done", "close_angle"] => "tag_done"
        },
        initial: "reading_name",
        accepting: Set["tag_done"]
      )
    end

    def self.make_script_mode
      DFA.new(
        states: Set["raw"],
        alphabet: Set["char", "end_marker"],
        transitions: {
          ["raw", "char"] => "raw",
          ["raw", "end_marker"] => "raw"
        },
        initial: "raw",
        accepting: Set["raw"]
      )
    end

    def self.make_html_tokenizer
      ModalStateMachine.new(
        modes: {
          "data" => make_data_mode,
          "tag" => make_tag_mode,
          "script" => make_script_mode
        },
        mode_transitions: {
          ["data", "enter_tag"] => "tag",
          ["tag", "exit_tag"] => "data",
          ["tag", "enter_script"] => "script",
          ["script", "exit_script"] => "data"
        },
        initial_mode: "data"
      )
    end

    # ================================================================
    # Construction Tests
    # ================================================================

    class TestModalConstruction < Minitest::Test
      def test_valid_construction
        html = StateMachine.make_html_tokenizer
        assert_equal "data", html.current_mode
        assert_equal 3, html.modes.size
      end

      def test_no_modes_rejected
        error = assert_raises(ArgumentError) do
          ModalStateMachine.new(
            modes: {},
            mode_transitions: {},
            initial_mode: "data"
          )
        end
        assert_match(/one mode/, error.message)
      end

      def test_invalid_initial_mode
        error = assert_raises(ArgumentError) do
          ModalStateMachine.new(
            modes: {"data" => StateMachine.make_data_mode},
            mode_transitions: {},
            initial_mode: "missing"
          )
        end
        assert_match(/Initial mode/, error.message)
      end

      def test_invalid_transition_source
        error = assert_raises(ArgumentError) do
          ModalStateMachine.new(
            modes: {"data" => StateMachine.make_data_mode},
            mode_transitions: {["missing", "trigger"] => "data"},
            initial_mode: "data"
          )
        end
        assert_match(/source/, error.message)
      end

      def test_invalid_transition_target
        error = assert_raises(ArgumentError) do
          ModalStateMachine.new(
            modes: {"data" => StateMachine.make_data_mode},
            mode_transitions: {["data", "trigger"] => "missing"},
            initial_mode: "data"
          )
        end
        assert_match(/target/, error.message)
      end
    end

    # ================================================================
    # Mode Switching Tests
    # ================================================================

    class TestModeSwitching < Minitest::Test
      def test_switch_mode
        html = StateMachine.make_html_tokenizer
        assert_equal "data", html.current_mode
        html.switch_mode("enter_tag")
        assert_equal "tag", html.current_mode
      end

      def test_switch_mode_returns_new_mode
        html = StateMachine.make_html_tokenizer
        result = html.switch_mode("enter_tag")
        assert_equal "tag", result
      end

      def test_switch_resets_target_dfa
        html = StateMachine.make_html_tokenizer
        html.switch_mode("enter_tag")
        html.process("char")
        html.process("close_angle")
        assert_equal "tag_done", html.active_machine.current_state

        html.switch_mode("exit_tag")
        html.switch_mode("enter_tag")
        assert_equal "reading_name", html.active_machine.current_state
      end

      def test_switch_data_to_tag_to_data
        html = StateMachine.make_html_tokenizer
        html.switch_mode("enter_tag")
        assert_equal "tag", html.current_mode
        html.switch_mode("exit_tag")
        assert_equal "data", html.current_mode
      end

      def test_switch_to_script_mode
        html = StateMachine.make_html_tokenizer
        html.switch_mode("enter_tag")
        html.switch_mode("enter_script")
        assert_equal "script", html.current_mode
        html.switch_mode("exit_script")
        assert_equal "data", html.current_mode
      end

      def test_invalid_trigger
        html = StateMachine.make_html_tokenizer
        error = assert_raises(ArgumentError) { html.switch_mode("nonexistent_trigger") }
        assert_match(/No mode transition/, error.message)
      end

      def test_mode_trace
        html = StateMachine.make_html_tokenizer
        html.switch_mode("enter_tag")
        html.switch_mode("exit_tag")

        trace = html.mode_trace
        assert_equal 2, trace.length
        assert_equal "data", trace[0].from_mode
        assert_equal "enter_tag", trace[0].trigger
        assert_equal "tag", trace[0].to_mode
        assert_equal "tag", trace[1].from_mode
        assert_equal "exit_tag", trace[1].trigger
        assert_equal "data", trace[1].to_mode
      end
    end

    # ================================================================
    # Processing Within Modes Tests
    # ================================================================

    class TestProcessingInModes < Minitest::Test
      def test_process_in_data_mode
        html = StateMachine.make_html_tokenizer
        result = html.process("char")
        assert_equal "text", result
      end

      def test_process_in_tag_mode
        html = StateMachine.make_html_tokenizer
        html.switch_mode("enter_tag")
        result = html.process("char")
        assert_equal "reading_name", result
        result = html.process("close_angle")
        assert_equal "tag_done", result
      end

      def test_process_in_script_mode
        html = StateMachine.make_html_tokenizer
        html.switch_mode("enter_tag")
        html.switch_mode("enter_script")
        result = html.process("char")
        assert_equal "raw", result
      end

      def test_process_invalid_event_for_mode
        html = StateMachine.make_html_tokenizer
        # "close_angle" is not in data mode's alphabet
        assert_raises(ArgumentError) { html.process("close_angle") }
      end

      def test_active_machine_property
        html = StateMachine.make_html_tokenizer
        data_dfa = html.active_machine
        assert_equal "text", data_dfa.current_state

        html.switch_mode("enter_tag")
        tag_dfa = html.active_machine
        assert_equal "reading_name", tag_dfa.current_state
      end
    end

    # ================================================================
    # Reset Tests
    # ================================================================

    class TestModalReset < Minitest::Test
      def test_reset_mode
        html = StateMachine.make_html_tokenizer
        html.switch_mode("enter_tag")
        html.reset
        assert_equal "data", html.current_mode
      end

      def test_reset_clears_trace
        html = StateMachine.make_html_tokenizer
        html.switch_mode("enter_tag")
        html.switch_mode("exit_tag")
        assert_equal 2, html.mode_trace.length

        html.reset
        assert_equal [], html.mode_trace
      end

      def test_reset_resets_all_dfas
        html = StateMachine.make_html_tokenizer
        html.switch_mode("enter_tag")
        html.process("char")
        html.process("close_angle")

        html.reset
        html.switch_mode("enter_tag")
        assert_equal "reading_name", html.active_machine.current_state
      end
    end

    # ================================================================
    # Repr Tests
    # ================================================================

    class TestModalRepr < Minitest::Test
      def test_repr
        html = StateMachine.make_html_tokenizer
        r = html.inspect
        assert_includes r, "ModalStateMachine"
        assert_includes r, "data"
      end
    end
  end
end
