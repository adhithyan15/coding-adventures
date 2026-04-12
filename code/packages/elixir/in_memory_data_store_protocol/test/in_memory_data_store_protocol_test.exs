defmodule CodingAdventures.InMemoryDataStoreProtocolTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.InMemoryDataStoreProtocol
  alias CodingAdventures.RESPProtocol

  test "parses command arrays" do
    resp = RESPProtocol.array([RESPProtocol.bulk_string("SET"), RESPProtocol.bulk_string("key"), RESPProtocol.bulk_string("value")])
    assert {:ok, %InMemoryDataStoreProtocol.Command{name: "SET", args: ["key", "value"]}} = InMemoryDataStoreProtocol.from_resp(resp)
  end

  test "encodes commands back to resp" do
    cmd = InMemoryDataStoreProtocol.build("PING", ["hello"])
    assert {:array, [{:bulk_string, "PING"}, {:bulk_string, "hello"}]} = InMemoryDataStoreProtocol.to_resp(cmd)
  end

  test "rejects empty and invalid commands" do
    assert {:error, :empty_command} = InMemoryDataStoreProtocol.from_resp(:null_array)
    assert {:error, :invalid_command} = InMemoryDataStoreProtocol.from_resp({:simple_string, "PING"})
  end

  test "build defaults and command round trip" do
    cmd = InMemoryDataStoreProtocol.build(:echo)
    assert cmd.name == "ECHO"
    assert cmd.args == []
    assert {:ok, parsed} = InMemoryDataStoreProtocol.from_resp(InMemoryDataStoreProtocol.to_resp(cmd))
    assert parsed.name == "ECHO"
  end

  test "simple string tokens and invalid tail tokens are rejected" do
    assert {:ok, %InMemoryDataStoreProtocol.Command{name: "PING", args: ["A"]}} =
             InMemoryDataStoreProtocol.from_resp({:array, [{:simple_string, "PING"}, {:simple_string, "A"}]})

    assert {:error, :invalid_command} =
             InMemoryDataStoreProtocol.from_resp({:array, [{:bulk_string, "PING"}, :null_bulk_string]})
  end
end
