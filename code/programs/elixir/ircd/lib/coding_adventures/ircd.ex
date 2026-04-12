defmodule CodingAdventures.Ircd do
  @moduledoc """
  IRC server entry point.

  This module is the wiring layer -- the topmost layer of the IRC stack.
  It connects the pure IRC logic (`irc_server`) to the TCP transport layer
  (`irc_net_stdlib`) via `DriverHandler`.

  ## Usage

  As a Mix escript:

      mix escript.build
      ./coding_adventures_ircd --port 6667

  As a module:

      CodingAdventures.Ircd.main(["--port", "6667"])
  """

  alias CodingAdventures.IrcServer
  alias CodingAdventures.Ircd.DriverHandler
  alias CodingAdventures.IrcNetStdlib.{EventLoop, Listener}

  @doc """
  Parse command-line arguments and start the IRC server.

  This is the escript entry point. It blocks until the process receives
  SIGINT or SIGTERM (or until `stop/0` is called in tests).
  """
  def main(argv \\ []) do
    config = parse_args(argv)

    {:ok, loop} = EventLoop.start_link()

    server_state = IrcServer.new(
      config.server_name,
      "0.1.0",
      config.motd,
      config.oper_password
    )

    {:ok, _handler} = DriverHandler.start_link(server_state, loop, name: DriverHandler)

    {:ok, sock} = Listener.listen(config.host, config.port)
    actual_port = Listener.port!(sock)

    {:ok, _accept_task} = EventLoop.run(loop, sock, DriverHandler)

    IO.puts("ircd listening on #{config.host}:#{actual_port}")

    # Block until the process is killed or stop() is called.
    receive do
      :stop -> :ok
    end

    EventLoop.stop(loop)
    Listener.close(sock)
  end

  @doc """
  Request the running ircd to stop.

  Sends a `:stop` message to the main process. Only useful in tests.
  """
  def stop do
    send(self(), :stop)
  end

  # ---------------------------------------------------------------------------
  # Config and argument parsing
  # ---------------------------------------------------------------------------

  defmodule Config do
    @moduledoc """
    Runtime configuration for `ircd`.

    Fields:

    - `:host`          -- bind address (default `"0.0.0.0"`).
    - `:port`          -- TCP port (default `6667`).
    - `:server_name`   -- hostname shown in messages (default `"irc.local"`).
    - `:motd`          -- list of MOTD lines.
    - `:oper_password` -- password for OPER command; empty disables.
    """

    defstruct host: "0.0.0.0",
              port: 6667,
              server_name: "irc.local",
              motd: ["Welcome."],
              oper_password: ""

    @type t :: %__MODULE__{
            host: String.t(),
            port: non_neg_integer(),
            server_name: String.t(),
            motd: [String.t()],
            oper_password: String.t()
          }
  end

  @doc """
  Parse command-line arguments into a `Config` struct.

  Supported flags:

  - `--host HOST`            -- bind address (default: 0.0.0.0)
  - `--port PORT`            -- TCP port (default: 6667)
  - `--server-name NAME`     -- server hostname (default: irc.local)
  - `--motd LINE`            -- MOTD line (may be repeated)
  - `--oper-password PASS`   -- OPER password (default: empty)

  ## Example

      parse_args(["--port", "6668", "--server-name", "irc.example.com"])
  """
  @spec parse_args([String.t()]) :: Config.t()
  def parse_args(argv) do
    {opts, _remaining, _errors} =
      OptionParser.parse(argv,
        strict: [
          host: :string,
          port: :integer,
          server_name: :string,
          motd: [:string, :keep],
          oper_password: :string
        ],
        aliases: [h: :host, p: :port]
      )

    motd =
      case Keyword.get_values(opts, :motd) do
        [] -> ["Welcome."]
        lines -> lines
      end

    %Config{
      host: Keyword.get(opts, :host, "0.0.0.0"),
      port: Keyword.get(opts, :port, 6667),
      server_name: Keyword.get(opts, :server_name, "irc.local"),
      motd: motd,
      oper_password: Keyword.get(opts, :oper_password, "")
    }
  end
end
