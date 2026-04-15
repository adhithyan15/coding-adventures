defmodule CodingAdventures.InMemoryDataStoreTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.InMemoryDataStore
  alias CodingAdventures.RESPProtocol

  test "execute_command and execute_resp both reach the engine" do
    store = InMemoryDataStore.new()

    {store, reply} = InMemoryDataStore.execute_command(store, "SET", ["foo", "bar"])
    assert reply == {:simple_string, "OK"}

    {store, reply} = InMemoryDataStore.execute_command(store, "GET", ["foo"])
    assert reply == {:bulk_string, "bar"}

    frame = RESPProtocol.encode(RESPProtocol.array([RESPProtocol.bulk_string("PING")]))
    {store, reply} = InMemoryDataStore.execute_resp(store, frame)
    assert {:ok, {:simple_string, "PONG"}, ""} = RESPProtocol.decode(reply)
    assert store.engine != nil
  end

  test "execute_resp handles invalid and trailing input" do
    store = InMemoryDataStore.new()

    {_, reply} = InMemoryDataStore.execute_resp(store, RESPProtocol.encode({:simple_string, "PING"}))
    assert {:ok, {:error, "ERR :invalid_command"}, ""} = RESPProtocol.decode(reply)

    {_, reply} = InMemoryDataStore.execute_resp(store, "*1\r\n$4\r\nPING\r\njunk")
    assert {:ok, {:error, "ERR :trailing_input"}, ""} = RESPProtocol.decode(reply)

    {_, reply} = InMemoryDataStore.execute_resp(store, "$3\r\nab")
    assert {:ok, {:error, "ERR :incomplete"}, ""} = RESPProtocol.decode(reply)
  end
end
