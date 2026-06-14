defmodule CodingAdventures.ConduitOtp.RouterTest do
  @moduledoc """
  Tests for the pure path-pattern router.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.ConduitOtp.Router

  defp routes(list), do: list

  describe "exact match" do
    test "matches root path" do
      rs = routes([%{method: "GET", pattern: "/", handler_id: 1}])
      assert {:ok, 1, %{}} = Router.match(rs, "GET", "/")
    end

    test "matches a literal path" do
      rs = routes([%{method: "GET", pattern: "/hello", handler_id: 2}])
      assert {:ok, 2, %{}} = Router.match(rs, "GET", "/hello")
    end

    test "returns :not_found for a missing path" do
      rs = routes([%{method: "GET", pattern: "/hello", handler_id: 1}])
      assert :not_found = Router.match(rs, "GET", "/goodbye")
    end
  end

  describe "method matching" do
    test "returns :not_found when method does not match" do
      rs = routes([%{method: "GET", pattern: "/", handler_id: 1}])
      assert :not_found = Router.match(rs, "POST", "/")
    end

    test "matches POST" do
      rs = routes([%{method: "POST", pattern: "/users", handler_id: 3}])
      assert {:ok, 3, %{}} = Router.match(rs, "POST", "/users")
    end

    test "matches DELETE" do
      rs = routes([%{method: "DELETE", pattern: "/items/:id", handler_id: 5}])
      assert {:ok, 5, %{"id" => "42"}} = Router.match(rs, "DELETE", "/items/42")
    end
  end

  describe "named captures" do
    test "captures a single :param segment" do
      rs = routes([%{method: "GET", pattern: "/hello/:name", handler_id: 4}])
      assert {:ok, 4, %{"name" => "Alice"}} = Router.match(rs, "GET", "/hello/Alice")
    end

    test "captures multiple :param segments" do
      rs = routes([%{method: "GET", pattern: "/a/:x/b/:y", handler_id: 6}])
      assert {:ok, 6, %{"x" => "1", "y" => "2"}} = Router.match(rs, "GET", "/a/1/b/2")
    end

    test "captured value is a string even for numeric-looking segments" do
      rs = routes([%{method: "GET", pattern: "/items/:id", handler_id: 7}])
      assert {:ok, 7, %{"id" => "123"}} = Router.match(rs, "GET", "/items/123")
    end
  end

  describe "first-match wins" do
    test "returns the handler_id of the first matching route" do
      rs =
        routes([
          %{method: "GET", pattern: "/items/:id", handler_id: 1},
          %{method: "GET", pattern: "/items/new", handler_id: 2}
        ])

      # The first route (:id) should win before the literal "new" route.
      assert {:ok, 1, %{"id" => "new"}} = Router.match(rs, "GET", "/items/new")
    end
  end

  describe "trailing slash normalisation" do
    test "trailing slash is stripped before matching" do
      rs = routes([%{method: "GET", pattern: "/hello", handler_id: 8}])
      assert {:ok, 8, %{}} = Router.match(rs, "GET", "/hello/")
    end

    test "root / is never stripped" do
      rs = routes([%{method: "GET", pattern: "/", handler_id: 9}])
      assert {:ok, 9, %{}} = Router.match(rs, "GET", "/")
    end
  end

  describe "no match" do
    test "empty routes always returns :not_found" do
      assert :not_found = Router.match([], "GET", "/anything")
    end

    test "wrong segment count returns :not_found" do
      rs = routes([%{method: "GET", pattern: "/a/b", handler_id: 1}])
      assert :not_found = Router.match(rs, "GET", "/a")
    end
  end
end
