defmodule CodingAdventures.Conduit.DispatcherTest do
  @moduledoc """
  Tests for the in-memory handler-execution path.

  We test `run_handler/3` directly (it's a pure function over the handler
  map) without needing the Rust NIF or a TCP server.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.Conduit.{Dispatcher, HandlerContext}

  test "happy path: handler returns a {status, headers, body} tuple" do
    handlers = %{1 => fn _req -> HandlerContext.html("<h1>OK</h1>") end}
    {200, headers, body} = Dispatcher.run_handler(handlers, 1, %{})
    assert headers["content-type"] == "text/html; charset=utf-8"
    assert body == "<h1>OK</h1>"
  end

  test "handler returns nil → no_override sentinel {0, %{}, \"\"}" do
    handlers = %{1 => fn _req -> nil end}
    assert Dispatcher.run_handler(handlers, 1, %{}) == {0, %{}, ""}
  end

  test "handler returns a bare string → wrapped in text/plain" do
    handlers = %{1 => fn _req -> "plain string" end}
    {200, headers, body} = Dispatcher.run_handler(handlers, 1, %{})
    assert headers["content-type"] == "text/plain; charset=utf-8"
    assert body == "plain string"
  end

  test "halt is converted to a real response tuple" do
    handlers = %{1 => fn _req -> HandlerContext.halt(503, "Maintenance") end}
    assert {503, %{}, "Maintenance"} == Dispatcher.run_handler(handlers, 1, %{})
  end

  test "redirect is converted to 302 with Location header" do
    handlers = %{1 => fn _req -> HandlerContext.redirect("/login") end}

    assert {302, %{"location" => "/login"}, ""} ==
             Dispatcher.run_handler(handlers, 1, %{})
  end

  test "rescue path: handler raises → bare 500, empty headers (Rust sentinel)" do
    handlers = %{1 => fn _req -> raise "boom" end}
    {500, headers, body} = Dispatcher.run_handler(handlers, 1, %{})
    assert headers == %{}
    assert body =~ "boom"
  end

  test "missing handler ID returns a 500 with diagnostic body" do
    {500, %{}, body} = Dispatcher.run_handler(%{}, 999, %{})
    assert body =~ "999"
  end

  test "handler returning a malformed shape gets coerced to 500 (no leak)" do
    handlers = %{1 => fn _req -> {:something_weird, 1, 2, 3} end}
    # The body is now a generic "Internal Server Error" — the offending
    # shape is logged but not echoed back to the client.
    {500, %{}, body} = Dispatcher.run_handler(handlers, 1, %{})
    assert body == "Internal Server Error"
    refute body =~ "something_weird"
  end
end
