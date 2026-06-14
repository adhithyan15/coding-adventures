defmodule CodingAdventures.ConduitOtp.HaltError do
  @moduledoc """
  Teaching topic: Non-local control flow with `throw`.

  ## Why `throw` instead of `raise`?

  `raise` is Elixir's signal for *genuine errors* — things that should not
  happen under normal operation. Raising collects a full stacktrace (expensive)
  and goes through the `rescue` machinery.

  `halt` and `redirect` are *not* errors — they are deliberate control-flow
  decisions ("stop processing, send this response now"). On BEAM, `throw/1` is
  the idiomatic mechanism for this: cheap (no stacktrace), caught with
  `catch :throw, value -> ...`.

  The pattern comes from Erlang itself:
  - `throw` — non-local return / early exit
  - `error` (aka `raise`) — genuine errors
  - `exit` — process termination signals

  ## Wire protocol

  The Worker catches:

      catch :throw, {:conduit_halt, status, body, headers} -> ...

  And turns the 4-tuple into the response `{status, headers, body}` it sends
  back over the TCP socket. This is the same pattern used by Plug and Sinatra.

  ## Security: CRLF injection in redirects

  HTTP headers are delimited by `\\r\\n`. If a Location header value contains
  a literal `\\r\\n`, an attacker can inject extra headers into the response
  (e.g., `Set-Cookie:` with a stolen session). We reject such values at the
  Elixir level — the first line of defence. The defence-in-depth principle:
  never trust input even from your own application layer.

  ## Example

      def before_maintenance(req) do
        if req.path == "/down", do: halt(503, "Under maintenance")
      end

      def require_auth(req) do
        unless req.headers["authorization"], do: redirect("/login")
      end
  """

  defexception message: "halted", status: 200, body: "", headers: %{}

  @doc """
  Throw a halt with a custom status, optional body, and optional headers.

      halt(404)                            # status 404, empty body
      halt(404, "Not found")               # status 404, body "Not found"
      halt(503, "Down", %{"Retry-After" => "60"})
  """
  @spec halt(integer, String.t(), map) :: no_return
  def halt(status, body \\ "", headers \\ %{}) do
    throw({:conduit_halt, status, body, headers})
  end

  @doc """
  Throw a redirect with a `Location:` header. Default 302 Found.

  ## Security

  - **CRLF injection** — any `\\r` or `\\n` in the location string is
    rejected with `ArgumentError`. An attacker cannot smuggle extra headers.
  - **Open redirect** — NOT automatically defended. Never pass unvalidated
    user input as `location`. Use a static path or an explicit allowlist.

  Examples:

      redirect("/login")       # 302
      redirect("/login", 301)  # 301 Moved Permanently
  """
  @spec redirect(String.t(), integer) :: no_return
  def redirect(location, status \\ 302) do
    if String.contains?(location, ["\r", "\n"]) do
      raise ArgumentError, "redirect location must not contain CR or LF"
    end

    throw({:conduit_halt, status, "", %{"location" => location}})
  end
end
