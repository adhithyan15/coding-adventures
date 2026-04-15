defmodule CodingAdventures.TypeCheckerProtocolTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.TypeCheckerProtocol
  alias CodingAdventures.TypeCheckerProtocol.GenericTypeChecker

  test "dispatch normalizes kinds" do
    checker =
      GenericTypeChecker.new(node_kind: & &1.kind)
      |> GenericTypeChecker.register_hook("enter", "fn decl", fn _node, _args -> :exact end)

    assert GenericTypeChecker.dispatch(checker, "enter", %{kind: "fn decl"}) == :exact
  end

  test "dispatch falls through not_handled" do
    checker =
      GenericTypeChecker.new(node_kind: & &1.kind)
      |> GenericTypeChecker.register_hook("enter", "expr:add", fn _node, _args ->
        GenericTypeChecker.not_handled()
      end)
      |> GenericTypeChecker.register_hook("enter", "*", fn _node, _args -> :fallback end)

    assert GenericTypeChecker.dispatch(checker, "enter", %{kind: "expr:add"}) == :fallback
  end

  test "check accumulates diagnostics" do
    checker =
      GenericTypeChecker.new(
        node_kind: & &1.kind,
        locate: fn node -> {node.line, node.column} end
      )

    result =
      GenericTypeChecker.check(checker, %{kind: "expr", line: 5, column: 9}, fn checker, ast ->
        GenericTypeChecker.error(checker, "bad node", ast)
      end)

    assert %TypeCheckerProtocol.TypeCheckResult{} = result
    assert result.ok == false
    assert [%TypeCheckerProtocol.TypeErrorDiagnostic{message: "bad node", line: 5, column: 9}] =
             result.errors
  end
end
