defmodule GrammarToolsCliTest do
  use ExUnit.Case
  doctest GrammarToolsCli

  test "greets the world" do
    assert GrammarToolsCli.hello() == :world
  end
end
