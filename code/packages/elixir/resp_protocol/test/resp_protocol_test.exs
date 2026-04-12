defmodule CodingAdventures.RESPProtocolTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.RESPProtocol

  test "encodes and decodes arrays" do
    payload = RESPProtocol.array([RESPProtocol.bulk_string("PING"), RESPProtocol.bulk_string("hello")])
    binary = RESPProtocol.encode(payload)
    assert {:ok, {:array, [ {:bulk_string, "PING"}, {:bulk_string, "hello"} ]}, ""} = RESPProtocol.decode(binary)
  end

  test "decodes simple strings" do
    assert {:ok, {:simple_string, "OK"}, ""} = RESPProtocol.decode("+OK\r\n")
  end

  test "covers integers bulk strings nulls and arrays" do
    assert {:ok, {:integer, 42}, ""} = RESPProtocol.decode(":42\r\n")
    assert {:ok, {:bulk_string, "abc"}, ""} = RESPProtocol.decode("$3\r\nabc\r\n")
    assert {:ok, :null_bulk_string, ""} = RESPProtocol.decode("$-1\r\n")
    assert {:ok, :null_array, ""} = RESPProtocol.decode("*-1\r\n")
    assert RESPProtocol.encode(RESPProtocol.null_array()) == "*-1\r\n"
    assert RESPProtocol.encode(RESPProtocol.error("ERR nope")) == "-ERR nope\r\n"
    assert RESPProtocol.encode(RESPProtocol.integer(7)) == ":7\r\n"
    assert RESPProtocol.encode(RESPProtocol.bulk_string("x")) == "$1\r\nx\r\n"
  end

  test "decode incomplete input is rejected" do
    assert {:error, :incomplete} = RESPProtocol.decode("$3\r\nab")
    assert {:error, :incomplete} = RESPProtocol.decode("*2\r\n$3\r\nfoo\r\n")
  end

  test "simple strings, nested arrays and error replies are covered" do
    assert {:ok, {:error, "ERR boom"}, ""} = RESPProtocol.decode("-ERR boom\r\n")
    assert RESPProtocol.encode(RESPProtocol.simple_string("OK")) == "+OK\r\n"

    nested = RESPProtocol.array([RESPProtocol.array([RESPProtocol.bulk_string("a")])])
    assert {:ok, {:array, [{:array, [{:bulk_string, "a"}]}]}, ""} = RESPProtocol.decode(RESPProtocol.encode(nested))
  end
end
