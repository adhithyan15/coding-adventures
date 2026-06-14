defmodule CodingAdventures.ConduitOtp.Application do
  @moduledoc """
  Teaching topic: Immutable application struct and functional DSL.

  ## What is this?

  This module defines `%Application{}` — a plain Elixir struct that holds
  everything you have declared about your web application: routes, filters,
  fallback handlers, and settings. It is purely data; it starts no processes
  and holds no resources.

  The struct is **immutable** in the Elixir sense: every builder function
  (`get/3`, `before_filter/2`, etc.) returns a *new* struct. The original is
  unchanged. This mirrors Elixir's functional core and makes the application
  definition trivially serialisable, inspectable, and safe to pass between
  processes.

  ## Why not macros?

  Plug and Phoenix use compile-time macros (`get "/" do ... end`). The
  trade-off is that the DSL becomes a mini language to debug, and the
  framework state is baked into the module bytecode — not inspectable at
  runtime without reflection.

  Conduit favours `IO.inspect(app)` showing you exactly what is registered:
  routes, handler IDs, filter lists. The same approach is used in the
  Python, Ruby, Lua, and TypeScript ports — one clear pattern everywhere.

  ## Handler IDs

  Every anonymous function you register is assigned a sequential integer
  `handler_id`. The struct keeps a `handlers` map:

      %{1 => fn1, 2 => fn2, ...}

  When a Worker receives a connection it takes a snapshot of the Application
  struct and looks up handlers by ID. This means:
  - Functions are never sent over the wire (they can't be; BEAM functions
    are closures tied to a process).
  - `hot_reload/1` on the RouteTable swaps the whole struct atomically.
    In-flight requests finish on the old version; new requests use the new one.

  ## Example

      app =
        Application.new()
        |> Application.before_filter(fn req ->
             if req.path == "/down", do: halt(503, "Maintenance")
           end)
        |> Application.get("/", fn _req -> html("<h1>Hello!</h1>") end)
        |> Application.get("/hello/:name", fn req ->
             json(%{message: "Hello " <> req.params["name"]})
           end)
        |> Application.not_found_handler(fn req ->
             html("<h1>Not Found: \#{req.path}</h1>", 404)
           end)
  """

  alias __MODULE__

  defstruct routes: [],
            before_filters: [],
            after_filters: [],
            not_found_handler: nil,
            error_handler: nil,
            settings: %{},
            handlers: %{},
            next_id: 1

  @type handler_id :: pos_integer
  @type handler :: (CodingAdventures.ConduitOtp.Request.t() -> term)

  @type route_entry :: %{
          method: String.t(),
          pattern: String.t(),
          handler_id: handler_id
        }

  @type t :: %__MODULE__{
          routes: [route_entry],
          before_filters: [handler_id],
          after_filters: [handler_id],
          not_found_handler: handler_id | nil,
          error_handler: handler_id | nil,
          settings: %{optional(String.t() | atom) => term},
          handlers: %{optional(handler_id) => handler},
          next_id: pos_integer
        }

  @doc "Construct an empty application."
  @spec new() :: t
  def new, do: %__MODULE__{}

  # ── HTTP method helpers ─────────────────────────────────────────────────────
  #
  # One function per HTTP method — spelled out explicitly rather than
  # generated with a macro, which keeps the documentation tool-friendly and
  # the intent obvious to a reader new to Elixir.

  @doc "Register a GET handler for `pattern`."
  @spec get(t, String.t(), handler) :: t
  def get(%Application{} = app, pattern, handler)
      when is_binary(pattern) and is_function(handler, 1),
      do: add_route(app, "GET", pattern, handler)

  @doc "Register a POST handler for `pattern`."
  @spec post(t, String.t(), handler) :: t
  def post(%Application{} = app, pattern, handler)
      when is_binary(pattern) and is_function(handler, 1),
      do: add_route(app, "POST", pattern, handler)

  @doc "Register a PUT handler for `pattern`."
  @spec put(t, String.t(), handler) :: t
  def put(%Application{} = app, pattern, handler)
      when is_binary(pattern) and is_function(handler, 1),
      do: add_route(app, "PUT", pattern, handler)

  @doc "Register a DELETE handler for `pattern`."
  @spec delete(t, String.t(), handler) :: t
  def delete(%Application{} = app, pattern, handler)
      when is_binary(pattern) and is_function(handler, 1),
      do: add_route(app, "DELETE", pattern, handler)

  @doc "Register a PATCH handler for `pattern`."
  @spec patch(t, String.t(), handler) :: t
  def patch(%Application{} = app, pattern, handler)
      when is_binary(pattern) and is_function(handler, 1),
      do: add_route(app, "PATCH", pattern, handler)

  @doc """
  Generic route registration when the method is dynamic.

  Useful for HEAD, OPTIONS, or any non-standard methods. Most users will
  call the named helpers (`get/3`, `post/3`, etc.) instead.
  """
  @spec add_route(t, String.t(), String.t(), handler) :: t
  def add_route(%Application{} = app, method, pattern, handler)
      when is_binary(method) and is_binary(pattern) and is_function(handler, 1) do
    {id, app} = assign_id(app, handler)
    %{app | routes: app.routes ++ [%{method: method, pattern: pattern, handler_id: id}]}
  end

  # ── Filters ─────────────────────────────────────────────────────────────────

  @doc """
  Register a `before` filter — runs before route dispatch on every request.

  The filter receives the current `Request`. If it returns `nil`, processing
  continues. If it returns a `{status, headers, body}` tuple OR calls
  `halt/1-3`, the request short-circuits immediately.
  """
  @spec before_filter(t, handler) :: t
  def before_filter(%Application{} = app, handler) when is_function(handler, 1) do
    {id, app} = assign_id(app, handler)
    %{app | before_filters: app.before_filters ++ [id]}
  end

  @doc """
  Register an `after` filter — runs after route dispatch on every response.

  Receives the `Request` (enriched with the route-matched params). The return
  value replaces the pending response tuple if it is a 3-tuple; otherwise the
  original response is kept.
  """
  @spec after_filter(t, handler) :: t
  def after_filter(%Application{} = app, handler) when is_function(handler, 1) do
    {id, app} = assign_id(app, handler)
    %{app | after_filters: app.after_filters ++ [id]}
  end

  # ── Fallback handlers ────────────────────────────────────────────────────────

  @doc "Set the not-found handler (called when no route matches). Overwrites any previous."
  @spec not_found_handler(t, handler) :: t
  def not_found_handler(%Application{} = app, handler) when is_function(handler, 1) do
    {id, app} = assign_id(app, handler)
    %{app | not_found_handler: id}
  end

  @doc "Set the error handler (called when a route handler raises). Overwrites any previous."
  @spec error_handler(t, handler) :: t
  def error_handler(%Application{} = app, handler) when is_function(handler, 1) do
    {id, app} = assign_id(app, handler)
    %{app | error_handler: id}
  end

  # ── Settings ─────────────────────────────────────────────────────────────────

  @doc "Store a setting. Value can be any term."
  @spec put_setting(t, atom | String.t(), term) :: t
  def put_setting(%Application{} = app, key, value) do
    %{app | settings: Map.put(app.settings, key, value)}
  end

  @doc "Read a setting; returns `default` (nil) if absent."
  @spec get_setting(t, atom | String.t(), term) :: term
  def get_setting(%Application{} = app, key, default \\ nil) do
    Map.get(app.settings, key, default)
  end

  # ── Internal: ID assignment ──────────────────────────────────────────────────

  # Each handler gets a monotonically-increasing integer ID.  The integer is
  # lightweight to pass around (fits in a machine word) while the actual closure
  # lives only in the `handlers` map — never serialised or sent cross-node.
  defp assign_id(%Application{next_id: id, handlers: handlers} = app, handler) do
    {id, %{app | next_id: id + 1, handlers: Map.put(handlers, id, handler)}}
  end
end
