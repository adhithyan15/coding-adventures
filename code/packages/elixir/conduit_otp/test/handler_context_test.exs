defmodule CodingAdventures.ConduitOtp.HandlerContextTest do
  @moduledoc """
  Tests for the response-helper and halt/redirect functions.
  """

  use ExUnit.Case, async: true

  import CodingAdventures.ConduitOtp.HandlerContext

  describe "html/1" do
    test "returns {200, text/html, body}" do
      assert {200, %{"content-type" => ct}, "<h1>Hi</h1>"} = html("<h1>Hi</h1>")
      assert ct =~ "text/html"
    end
  end

  describe "html/2" do
    test "accepts a custom status" do
      {404, %{"content-type" => ct}, _body} = html("Not Found", 404)
      assert ct =~ "text/html"
    end
  end

  describe "json/1" do
    test "encodes a map as JSON" do
      {200, %{"content-type" => ct}, body} = json(%{ok: true})
      assert ct == "application/json"
      assert body =~ "true"
    end

    test "encodes a list" do
      {200, _, body} = json([1, 2, 3])
      assert body =~ "1"
    end

    test "encodes nil as null" do
      {200, _, body} = json(nil)
      assert body == "null"
    end
  end

  describe "json/2" do
    test "accepts a custom status" do
      {500, _, _} = json(%{error: "boom"}, 500)
    end
  end

  describe "text/1" do
    test "returns {200, text/plain, body}" do
      {200, %{"content-type" => ct}, "pong"} = text("pong")
      assert ct =~ "text/plain"
    end
  end

  describe "text/2" do
    test "accepts a custom status" do
      {201, _, "created"} = text("created", 201)
    end
  end

  describe "respond/2" do
    test "returns status and body with empty headers" do
      {204, %{}, ""} = respond(204, "")
    end
  end

  describe "respond/3" do
    test "includes custom headers" do
      {200, %{"x-custom" => "val"}, "body"} = respond(200, "body", %{"x-custom" => "val"})
    end
  end

  describe "halt delegation" do
    test "halt/1 throws via HaltError" do
      assert catch_throw(halt(404)) == {:conduit_halt, 404, "", %{}}
    end

    test "halt/2 throws with body" do
      assert catch_throw(halt(503, "maintenance")) == {:conduit_halt, 503, "maintenance", %{}}
    end

    test "halt/3 throws with headers" do
      assert catch_throw(halt(503, "down", %{"retry-after" => "60"})) ==
               {:conduit_halt, 503, "down", %{"retry-after" => "60"}}
    end
  end

  describe "redirect delegation" do
    test "redirect/1 throws a 302" do
      assert catch_throw(redirect("/new")) ==
               {:conduit_halt, 302, "", %{"location" => "/new"}}
    end

    test "redirect/2 allows a custom status" do
      assert catch_throw(redirect("/new", 301)) ==
               {:conduit_halt, 301, "", %{"location" => "/new"}}
    end
  end
end
