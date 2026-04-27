defmodule CodingAdventures.ConduitTest do
  @moduledoc """
  Tests for the umbrella module's delegated helpers — re-exports of
  HandlerContext, HaltError, and Application/Server entry points.

  These exist mainly so the umbrella module reaches coverage; they also
  document the public API surface in one place.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.Conduit
  alias CodingAdventures.Conduit.Application

  test "application/0 returns an empty Application struct" do
    assert %Application{} = Conduit.application()
  end

  test "html/1 delegates to HandlerContext.html/1" do
    assert {200, %{"content-type" => "text/html; charset=utf-8"}, "<p>hi</p>"} =
             Conduit.html("<p>hi</p>")
  end

  test "html/2 delegates to HandlerContext.html/2 with custom status" do
    assert {201, _, "x"} = Conduit.html("x", 201)
  end

  test "json/1 delegates to HandlerContext.json/1" do
    assert {200, %{"content-type" => "application/json"}, body} = Conduit.json(%{a: 1})
    assert is_binary(body)
  end

  test "json/2 delegates to HandlerContext.json/2 with custom status" do
    {500, _, _} = Conduit.json(%{e: "boom"}, 500)
  end

  test "text/1 delegates to HandlerContext.text/1" do
    assert {200, %{"content-type" => "text/plain; charset=utf-8"}, "hi"} = Conduit.text("hi")
  end

  test "text/2 delegates to HandlerContext.text/2" do
    {204, _, ""} = Conduit.text("", 204)
  end

  test "respond/2 delegates to HandlerContext.respond/2" do
    assert {200, %{}, "x"} = Conduit.respond(200, "x")
  end

  test "respond/3 delegates to HandlerContext.respond/3" do
    assert {204, %{"x" => "y"}, ""} = Conduit.respond(204, "", %{"x" => "y"})
  end

  test "halt/1 throws via HaltError" do
    assert catch_throw(Conduit.halt(404)) == {:conduit_halt, 404, "", %{}}
  end

  test "halt/2 throws with body" do
    assert catch_throw(Conduit.halt(403, "no")) == {:conduit_halt, 403, "no", %{}}
  end

  test "halt/3 throws with headers" do
    assert catch_throw(Conduit.halt(503, "down", %{"retry-after" => "60"})) ==
             {:conduit_halt, 503, "down", %{"retry-after" => "60"}}
  end

  test "redirect/1 throws 302 with location" do
    assert catch_throw(Conduit.redirect("/login")) ==
             {:conduit_halt, 302, "", %{"location" => "/login"}}
  end

  test "redirect/2 throws with custom status" do
    assert catch_throw(Conduit.redirect("/old", 301)) ==
             {:conduit_halt, 301, "", %{"location" => "/old"}}
  end
end
