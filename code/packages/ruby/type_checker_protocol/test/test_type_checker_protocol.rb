# frozen_string_literal: true

require_relative "test_helper"

class DummyChecker < CodingAdventures::TypeCheckerProtocol::GenericTypeChecker
  def initialize
    super(
      node_kind: ->(node) { node[:kind] },
      locate: ->(node) { [node.fetch(:line, 1), node.fetch(:column, 1)] }
    )
  end

  def run(ast)
    dispatch("enter", ast)
  end
end

class TypeCheckerProtocolTest < Minitest::Test
  def test_exact_hook_dispatches
    checker = DummyChecker.new
    checker.register_hook("enter", "fn decl") { |_node| :exact }

    assert_equal :exact, checker.dispatch("enter", { kind: "fn decl" })
  end

  def test_wildcard_hook_dispatches_when_exact_missing
    checker = DummyChecker.new
    checker.register_hook("enter", "*") { |_node| :wildcard }

    assert_equal :wildcard, checker.dispatch("enter", { kind: "let stmt" })
  end

  def test_not_handled_falls_through_to_wildcard
    checker = DummyChecker.new
    checker.register_hook("enter", "expr:add") { |_node| checker.not_handled }
    checker.register_hook("enter", "*") { |_node| :fallback }

    assert_equal :fallback, checker.dispatch("enter", { kind: "expr:add" })
  end

  def test_check_returns_result_with_errors
    checker = DummyChecker.new
    checker.register_hook("enter", "*") do |node|
      checker.error("bad node", node)
    end

    result = checker.check({ kind: "expr:add", line: 3, column: 7 })

    assert_equal false, result.ok
    assert_equal 1, result.errors.length
    assert_equal "bad node", result.errors.first.message
    assert_equal 3, result.errors.first.line
    assert_equal 7, result.errors.first.column
  end
end
