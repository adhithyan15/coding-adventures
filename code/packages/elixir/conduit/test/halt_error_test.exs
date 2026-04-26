defmodule CodingAdventures.Conduit.HaltErrorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Conduit.HaltError

  describe "halt/1" do
    test "throws conduit_halt with given status, empty body, empty headers" do
      caught =
        try do
          HaltError.halt(404)
        catch
          :throw, value -> value
        end

      assert caught == {:conduit_halt, 404, "", %{}}
    end
  end

  describe "halt/2" do
    test "includes the body" do
      caught =
        try do
          HaltError.halt(403, "Forbidden")
        catch
          :throw, value -> value
        end

      assert caught == {:conduit_halt, 403, "Forbidden", %{}}
    end
  end

  describe "halt/3" do
    test "includes headers" do
      caught =
        try do
          HaltError.halt(503, "Down", %{"retry-after" => "60"})
        catch
          :throw, value -> value
        end

      assert caught == {:conduit_halt, 503, "Down", %{"retry-after" => "60"}}
    end
  end

  describe "redirect/1" do
    test "defaults to 302 with location header" do
      caught =
        try do
          HaltError.redirect("/login")
        catch
          :throw, value -> value
        end

      assert caught == {:conduit_halt, 302, "", %{"location" => "/login"}}
    end
  end

  describe "redirect/2" do
    test "respects an explicit status (e.g. 301 permanent)" do
      caught =
        try do
          HaltError.redirect("/new", 301)
        catch
          :throw, value -> value
        end

      assert caught == {:conduit_halt, 301, "", %{"location" => "/new"}}
    end
  end

  describe "exception fields" do
    test "the struct exists and stores status/body/headers" do
      e = %HaltError{status: 200, body: "ok", headers: %{}, message: "halt(200)"}
      assert e.status == 200
      assert e.body == "ok"
      assert e.headers == %{}
    end
  end
end
