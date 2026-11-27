defmodule CodingAdventures.UnixHistoryTest do
  use ExUnit.Case, async: true
  alias CodingAdventures.UnixHistory, as: H

  test "path traversal and order are stable" do
    assert {:ok, %{number: 1}} = H.resolve(H.new().fs, "//etc/../")
    assert %{"names" => ["hello", "empty", "etc"]} = H.list(H.new(), "/")
    assert {:error, "ENOTDIR"} = H.resolve(H.new().fs, "/hello/..")
  end

  test "descriptor offsets, independent opens, lowest free reuse" do
    {k, a} = H.open(H.new(), "/hello")
    {k, b} = H.open(k, "/hello")
    assert a["fd"] == 0 and b["fd"] == 1
    {k, e, "Hello, U"} = H.read(k, 0, 8)
    assert e["nextOffset"] == 8
    {k, close} = H.close(k, 0)
    assert close["call"] == "close"
    {_k, c} = H.open(k, "/empty")
    assert c["fd"] == 0
  end

  test "invalid reads and full tables preserve their existing slots" do
    {full, events} =
      Enum.reduce(0..3, {H.new(), []}, fn _index, {state, events} ->
        {next, event} = H.open(state, "/hello")
        {next, events ++ [event]}
      end)

    assert Enum.map(events, & &1["fd"]) == [0, 1, 2, 3]
    {unchanged, failed_read, <<>>} = H.read(full, 0, -1)
    assert failed_read["error"] == "EINVAL"
    {unchanged, overflow} = H.open(unchanged, "/empty")
    assert overflow["error"] == "EMFILE"
    {freed, _closed} = H.close(unchanged, 2)
    {_freed, reused} = H.open(freed, "/empty")
    assert reused["fd"] == 2
    {_same, bad_close} = H.close(full, 8)
    assert bad_close["error"] == "EBADF"
  end

  test "shell emits fixture-shaped object" do
    assert %{"output" => "Hello, Unix!\n", "events" => events} = H.run_command("cat /hello")
    assert length(events) == 5
    assert %{"output" => "error: ENOENT\n"} = H.run_command("cat /missing")
  end
end
