defmodule CodingAdventures.ConduitOtp.ApplicationTest do
  @moduledoc """
  Tests for the Application struct and functional DSL.
  Mirrors the WEB06 application_test to verify API parity.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.ConduitOtp.Application

  describe "new/0" do
    test "returns an empty struct with no routes or handlers" do
      app = Application.new()
      assert app.routes == []
      assert app.before_filters == []
      assert app.after_filters == []
      assert app.not_found_handler == nil
      assert app.error_handler == nil
      assert app.settings == %{}
      assert app.handlers == %{}
    end

    test "next_id starts at 1" do
      assert Application.new().next_id == 1
    end
  end

  describe "get/3" do
    test "registers a GET route" do
      app = Application.new() |> Application.get("/", fn _ -> :ok end)
      assert [%{method: "GET", pattern: "/", handler_id: 1}] = app.routes
    end

    test "stores the handler function by ID" do
      f = fn _ -> :my_handler end
      app = Application.new() |> Application.get("/", f)
      assert app.handlers[1] == f
    end

    test "increments next_id" do
      app = Application.new() |> Application.get("/", fn _ -> :ok end)
      assert app.next_id == 2
    end
  end

  describe "post/3" do
    test "registers a POST route" do
      app = Application.new() |> Application.post("/users", fn _ -> :ok end)
      assert [%{method: "POST", pattern: "/users"}] = app.routes
    end
  end

  describe "put/3, delete/3, patch/3" do
    test "each registers the correct method" do
      app =
        Application.new()
        |> Application.put("/p", fn _ -> :ok end)
        |> Application.delete("/d", fn _ -> :ok end)
        |> Application.patch("/x", fn _ -> :ok end)

      methods = Enum.map(app.routes, & &1.method)
      assert methods == ["PUT", "DELETE", "PATCH"]
    end
  end

  describe "add_route/4" do
    test "registers a route with any method" do
      app = Application.new() |> Application.add_route("OPTIONS", "/api", fn _ -> :ok end)
      assert [%{method: "OPTIONS", pattern: "/api"}] = app.routes
    end
  end

  describe "registration order" do
    test "routes are appended in order" do
      app =
        Application.new()
        |> Application.get("/a", fn _ -> :ok end)
        |> Application.get("/b", fn _ -> :ok end)
        |> Application.get("/c", fn _ -> :ok end)

      assert Enum.map(app.routes, & &1.pattern) == ["/a", "/b", "/c"]
    end

    test "each route gets a unique handler_id" do
      app =
        Application.new()
        |> Application.get("/a", fn _ -> :ok end)
        |> Application.post("/b", fn _ -> :ok end)

      ids = Enum.map(app.routes, & &1.handler_id)
      assert ids == Enum.uniq(ids)
    end
  end

  describe "before_filter/2" do
    test "appends a handler ID" do
      app = Application.new() |> Application.before_filter(fn _ -> nil end)
      assert [id] = app.before_filters
      assert is_integer(id)
    end

    test "appends multiple in order" do
      app =
        Application.new()
        |> Application.before_filter(fn _ -> nil end)
        |> Application.before_filter(fn _ -> nil end)

      assert length(app.before_filters) == 2
    end
  end

  describe "after_filter/2" do
    test "appends a handler ID" do
      app = Application.new() |> Application.after_filter(fn _ -> nil end)
      assert [_id] = app.after_filters
    end
  end

  describe "not_found_handler/2" do
    test "sets a single handler id" do
      app = Application.new() |> Application.not_found_handler(fn _ -> :ok end)
      assert is_integer(app.not_found_handler)
    end

    test "overwrites the previous not_found handler" do
      app =
        Application.new()
        |> Application.not_found_handler(fn _ -> :first end)
        |> Application.not_found_handler(fn _ -> :second end)

      id = app.not_found_handler
      assert app.handlers[id].(%{}) == :second
    end
  end

  describe "error_handler/2" do
    test "sets a single handler id" do
      app = Application.new() |> Application.error_handler(fn _ -> :ok end)
      assert is_integer(app.error_handler)
    end

    test "overwrites the previous error handler" do
      app =
        Application.new()
        |> Application.error_handler(fn _ -> :first end)
        |> Application.error_handler(fn _ -> :second end)

      id = app.error_handler
      assert app.handlers[id].(%{}) == :second
    end
  end

  describe "settings" do
    test "put_setting + get_setting round-trip with atom key" do
      app = Application.new() |> Application.put_setting(:name, "Test")
      assert Application.get_setting(app, :name) == "Test"
    end

    test "put_setting + get_setting round-trip with string key" do
      app = Application.new() |> Application.put_setting("port", 3001)
      assert Application.get_setting(app, "port") == 3001
    end

    test "get_setting returns default when key is absent" do
      app = Application.new()
      assert Application.get_setting(app, :missing, :default) == :default
    end

    test "get_setting default is nil" do
      app = Application.new()
      assert Application.get_setting(app, :missing) == nil
    end
  end

  describe "chainability" do
    test "all builders return %Application{}" do
      app =
        Application.new()
        |> Application.get("/", fn _ -> :ok end)
        |> Application.post("/u", fn _ -> :ok end)
        |> Application.put("/p", fn _ -> :ok end)
        |> Application.delete("/d", fn _ -> :ok end)
        |> Application.patch("/x", fn _ -> :ok end)
        |> Application.before_filter(fn _ -> nil end)
        |> Application.after_filter(fn _ -> nil end)
        |> Application.not_found_handler(fn _ -> :ok end)
        |> Application.error_handler(fn _ -> :ok end)
        |> Application.put_setting(:k, :v)

      assert %Application{} = app
    end

    test "returns a new struct each time (immutability)" do
      base = Application.new()
      app1 = Application.get(base, "/a", fn _ -> :ok end)
      app2 = Application.get(base, "/b", fn _ -> :ok end)

      # base was not mutated
      assert base.routes == []
      # app1 and app2 are different
      assert Enum.map(app1.routes, & &1.pattern) == ["/a"]
      assert Enum.map(app2.routes, & &1.pattern) == ["/b"]
    end
  end
end
