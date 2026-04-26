defmodule CodingAdventures.Conduit.HandlerContextTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Conduit.HandlerContext

  describe "html/1, html/2" do
    test "default 200 status" do
      assert HandlerContext.html("<h1>OK</h1>") ==
               {200, %{"content-type" => "text/html; charset=utf-8"}, "<h1>OK</h1>"}
    end

    test "custom status" do
      assert HandlerContext.html("<h1>Created</h1>", 201) ==
               {201, %{"content-type" => "text/html; charset=utf-8"}, "<h1>Created</h1>"}
    end
  end

  describe "json/1, json/2" do
    test "encodes a map and sets content-type" do
      {status, headers, body} = HandlerContext.json(%{ok: true, n: 42})
      assert status == 200
      assert headers == %{"content-type" => "application/json"}
      # Don't depend on key ordering — just check parse round-trip ish
      assert is_binary(body)
      assert body =~ ~r/\"ok\"\s*:\s*true/
      assert body =~ ~r/\"n\"\s*:\s*42/
    end

    test "custom status code is propagated" do
      {status, _, _} = HandlerContext.json(%{error: "boom"}, 500)
      assert status == 500
    end

    test "encodes lists" do
      {_, _, body} = HandlerContext.json([1, 2, 3])
      assert body == "[1,2,3]"
    end

    test "encodes nil and booleans" do
      assert {_, _, "null"} = HandlerContext.json(nil)
      assert {_, _, "true"} = HandlerContext.json(true)
      assert {_, _, "false"} = HandlerContext.json(false)
    end
  end

  describe "text/1, text/2" do
    test "default 200 status" do
      assert HandlerContext.text("hello") ==
               {200, %{"content-type" => "text/plain; charset=utf-8"}, "hello"}
    end

    test "custom status" do
      {404, _, _} = HandlerContext.text("nope", 404)
    end
  end

  describe "respond/3" do
    test "passes status, body, headers through unchanged" do
      assert HandlerContext.respond(204, "", %{"x-custom" => "abc"}) ==
               {204, %{"x-custom" => "abc"}, ""}
    end

    test "default empty headers" do
      assert HandlerContext.respond(200, "ok") == {200, %{}, "ok"}
    end
  end

  describe "halt re-export" do
    test "halt/1 throws via HaltError" do
      assert catch_throw(HandlerContext.halt(404)) == {:conduit_halt, 404, "", %{}}
    end

    test "halt/2 throws with body" do
      assert catch_throw(HandlerContext.halt(403, "no")) == {:conduit_halt, 403, "no", %{}}
    end

    test "halt/3 throws with headers" do
      assert catch_throw(HandlerContext.halt(503, "down", %{"retry-after" => "60"})) ==
               {:conduit_halt, 503, "down", %{"retry-after" => "60"}}
    end

    test "redirect/1 throws 302 with Location" do
      assert catch_throw(HandlerContext.redirect("/login")) ==
               {:conduit_halt, 302, "", %{"location" => "/login"}}
    end

    test "redirect/2 with custom status" do
      assert catch_throw(HandlerContext.redirect("/old", 301)) ==
               {:conduit_halt, 301, "", %{"location" => "/old"}}
    end
  end
end
