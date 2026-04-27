defmodule CodingAdventures.ConduitOtp.HaltErrorTest do
  @moduledoc """
  Tests for the throw-based halt/redirect helpers.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.ConduitOtp.HaltError

  describe "halt/1" do
    test "throws {:conduit_halt, status, empty_body, empty_headers}" do
      assert catch_throw(HaltError.halt(404)) == {:conduit_halt, 404, "", %{}}
    end

    test "throws for any status code" do
      assert catch_throw(HaltError.halt(503)) == {:conduit_halt, 503, "", %{}}
    end
  end

  describe "halt/2" do
    test "throws with a body" do
      assert catch_throw(HaltError.halt(404, "Not found")) ==
               {:conduit_halt, 404, "Not found", %{}}
    end

    test "throws 503 with maintenance body" do
      assert catch_throw(HaltError.halt(503, "Maintenance mode")) ==
               {:conduit_halt, 503, "Maintenance mode", %{}}
    end
  end

  describe "halt/3" do
    test "throws with body and headers" do
      headers = %{"retry-after" => "60"}

      assert catch_throw(HaltError.halt(503, "Down", headers)) ==
               {:conduit_halt, 503, "Down", headers}
    end
  end

  describe "redirect/1" do
    test "throws a 302 redirect with location header" do
      assert catch_throw(HaltError.redirect("/login")) ==
               {:conduit_halt, 302, "", %{"location" => "/login"}}
    end
  end

  describe "redirect/2" do
    test "allows a custom status" do
      assert catch_throw(HaltError.redirect("/new", 301)) ==
               {:conduit_halt, 301, "", %{"location" => "/new"}}
    end
  end

  describe "redirect CRLF injection guard" do
    test "raises ArgumentError when location contains CR" do
      assert_raise ArgumentError, fn -> HaltError.redirect("/foo\rbar") end
    end

    test "raises ArgumentError when location contains LF" do
      assert_raise ArgumentError, fn -> HaltError.redirect("/foo\nbar") end
    end

    test "raises ArgumentError when location contains CRLF sequence" do
      assert_raise ArgumentError, fn ->
        HaltError.redirect("/login\r\nSet-Cookie: evil=1")
      end
    end
  end
end
