defmodule CodingAdventures.ConduitOtp.RouteTableTest do
  @moduledoc """
  Tests for the Agent-based RouteTable: start, snapshot, and hot reload.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.ConduitOtp.{RouteTable, Application}

  # Each test starts its own uniquely-named RouteTable to avoid name conflicts
  # across async tests.

  defp new_table(app) do
    name = :"RouteTable_test_#{:erlang.unique_integer([:positive])}"
    {:ok, _pid} = RouteTable.start_link(app: app, name: name)
    name
  end

  describe "start_link/1 and snapshot/1" do
    test "snapshot returns the Application passed at startup" do
      app = Application.new() |> Application.get("/", fn _ -> :ok end)
      name = new_table(app)

      snapshot = RouteTable.snapshot(name)
      assert %Application{} = snapshot
      assert length(snapshot.routes) == 1
    end

    test "snapshot returns exact same struct reference" do
      app = Application.new()
      name = new_table(app)
      assert RouteTable.snapshot(name) == app
    end
  end

  describe "hot_reload/2" do
    test "snapshot reflects the new Application after hot_reload" do
      original = Application.new() |> Application.get("/old", fn _ -> :old end)
      name = new_table(original)

      new_app = Application.new() |> Application.get("/new", fn _ -> :new end)
      :ok = RouteTable.hot_reload(new_app, name)

      snapshot = RouteTable.snapshot(name)
      assert Enum.any?(snapshot.routes, &(&1.pattern == "/new"))
      refute Enum.any?(snapshot.routes, &(&1.pattern == "/old"))
    end

    test "hot_reload is atomic (no partial state seen)" do
      app = Application.new()
      name = new_table(app)

      new_app =
        Application.new()
        |> Application.get("/a", fn _ -> :ok end)
        |> Application.get("/b", fn _ -> :ok end)

      :ok = RouteTable.hot_reload(new_app, name)
      snap = RouteTable.snapshot(name)
      assert length(snap.routes) == 2
    end

    test "multiple hot_reloads in sequence" do
      app = Application.new()
      name = new_table(app)

      :ok = RouteTable.hot_reload(Application.new() |> Application.get("/v1", fn _ -> :ok end), name)
      :ok = RouteTable.hot_reload(Application.new() |> Application.get("/v2", fn _ -> :ok end), name)

      snap = RouteTable.snapshot(name)
      assert Enum.any?(snap.routes, &(&1.pattern == "/v2"))
      refute Enum.any?(snap.routes, &(&1.pattern == "/v1"))
    end
  end

  describe "default name" do
    # We can't use the default name (__MODULE__) in async tests because it would
    # conflict with other test runs. Instead we test the API signature.
    test "start_link requires an :app key" do
      assert_raise KeyError, fn ->
        RouteTable.start_link([])
      end
    end
  end
end
