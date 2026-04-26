defmodule CodingAdventures.Conduit.ApplicationTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Conduit.Application

  describe "new/0" do
    test "returns an empty struct" do
      app = Application.new()
      assert app.routes == []
      assert app.before_filters == []
      assert app.after_filters == []
      assert app.not_found_handler == nil
      assert app.error_handler == nil
      assert app.settings == %{}
      assert app.handlers == %{}
    end
  end

  describe "method helpers" do
    test "get/3 registers a GET route and returns a new struct" do
      app =
        Application.new()
        |> Application.get("/", fn _ -> :ok end)

      assert [%{method: "GET", pattern: "/", handler_id: 1}] = app.routes
      assert is_function(app.handlers[1], 1)
    end

    test "post/3 registers a POST route" do
      app = Application.new() |> Application.post("/u", fn _ -> :ok end)
      assert [%{method: "POST"}] = app.routes
    end

    test "put/3, delete/3, patch/3 each register their respective method" do
      app =
        Application.new()
        |> Application.put("/p", fn _ -> :ok end)
        |> Application.delete("/d", fn _ -> :ok end)
        |> Application.patch("/x", fn _ -> :ok end)

      methods = Enum.map(app.routes, & &1.method)
      assert methods == ["PUT", "DELETE", "PATCH"]
    end

    test "registration order is preserved" do
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

  describe "filters" do
    test "before_filter/2 appends a handler" do
      app =
        Application.new()
        |> Application.before_filter(fn _ -> nil end)
        |> Application.before_filter(fn _ -> nil end)

      assert length(app.before_filters) == 2
    end

    test "after_filter/2 appends a handler" do
      app = Application.new() |> Application.after_filter(fn _ -> nil end)
      assert [_id] = app.after_filters
    end
  end

  describe "fallback handlers" do
    test "not_found_handler/2 sets a single handler id" do
      app = Application.new() |> Application.not_found_handler(fn _ -> :ok end)
      assert is_integer(app.not_found_handler)
    end

    test "not_found_handler/2 overwrites previous setting" do
      app =
        Application.new()
        |> Application.not_found_handler(fn _ -> :first end)
        |> Application.not_found_handler(fn _ -> :second end)

      first_id = app.not_found_handler
      handler = app.handlers[first_id]
      assert handler.(nil) == :second
    end

    test "error_handler/2 sets a single handler id" do
      app = Application.new() |> Application.error_handler(fn _ -> :ok end)
      assert is_integer(app.error_handler)
    end
  end

  describe "settings" do
    test "put_setting/3 + get_setting/2 round-trip" do
      app =
        Application.new()
        |> Application.put_setting(:app_name, "Test")
        |> Application.put_setting("port", 3001)

      assert Application.get_setting(app, :app_name) == "Test"
      assert Application.get_setting(app, "port") == 3001
    end

    test "get_setting/3 returns the default when key is missing" do
      app = Application.new()
      assert Application.get_setting(app, :nope, :missing) == :missing
    end
  end

  describe "chainability" do
    test "all builders return %Application{}" do
      app =
        Application.new()
        |> Application.get("/", fn _ -> :ok end)
        |> Application.post("/u", fn _ -> :ok end)
        |> Application.before_filter(fn _ -> nil end)
        |> Application.after_filter(fn _ -> nil end)
        |> Application.not_found_handler(fn _ -> :ok end)
        |> Application.error_handler(fn _ -> :ok end)
        |> Application.put_setting(:k, :v)

      assert %Application{} = app
    end
  end
end
