defmodule CodingAdventures.Conduit.HaltError do
  @moduledoc """
  The escape hatch for short-circuiting a handler with an immediate response.

  ## Why `throw` instead of `raise`?

  `raise` collects a stacktrace — that work is wasted when the "exception"
  is actually flow-control (`halt(404, "Not found")` is not an error, it's
  a directive to stop processing and return a 404). On BEAM, `throw` is
  cheap and is the idiomatic mechanism for non-local return.

  Conduit reserves `raise` for genuine errors — those route through the
  `error_handler` callback. `halt` and `redirect` use `throw` and short-
  circuit the dispatcher's response machinery.

  ## Wire protocol

  The dispatcher catches:

      catch :throw, {:conduit_halt, status, body, headers} -> ...

  And converts the tuple to a `{status, headers, body}` response sent to
  Rust via `Conduit.Native.respond/2`.

  ## Example

      def maintenance(req) do
        if String.starts_with?(req.path, "/admin"),
          do: halt(503, "Under maintenance")
      end
  """

  defexception message: "halted", status: 200, body: "", headers: %{}

  @doc """
  Throw a halt with a custom status, optional body, and optional headers.

      halt(404)                          # → status: 404, body: "", headers: %{}
      halt(404, "Not found")             # → status: 404, body: "Not found"
      halt(503, "Down", %{"Retry-After" => "60"})
  """
  @spec halt(integer, String.t(), map) :: no_return
  def halt(status, body \\ "", headers \\ %{}) do
    throw({:conduit_halt, status, body, headers})
  end

  @doc """
  Throw a redirect with a `Location:` header. Default 302 Found.

  ## Security

  - **CRLF injection** — defended in depth: any `\\r` or `\\n` in the
    location string is rejected with `ArgumentError` so an attacker
    can't smuggle a second header (e.g. `Set-Cookie:`) into the
    response by exploiting unvalidated input. The Rust side performs
    the same check; this is the first line of defense.
  - **Open-redirect** — NOT defended automatically. Do NOT pass
    unvalidated user input as the `location` argument. Use a static
    path or an explicit allowlist. Same convention as Sinatra,
    Express, Flask.

  Examples:

      redirect("/login")           # 302
      redirect("/login", 301)      # 301
  """
  @spec redirect(String.t(), integer) :: no_return
  def redirect(location, status \\ 302) do
    if String.contains?(location, ["\r", "\n"]) do
      raise ArgumentError, "redirect location must not contain CR or LF"
    end

    throw({:conduit_halt, status, "", %{"location" => location}})
  end
end
