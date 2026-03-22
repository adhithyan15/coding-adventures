defmodule UnixTools.Uname do
  @moduledoc """
  uname -- print system information.

  ## What This Program Does

  This is a reimplementation of the GNU `uname` utility in Elixir. It prints
  various pieces of system information depending on which flags are provided.

  ## How uname Works

  With no flags, `uname` prints just the kernel name:

      uname    =>   Darwin     (on macOS)
      uname    =>   Linux      (on Linux)

  With `-a` (all), it prints everything:

      uname -a  =>  Darwin hostname 23.1.0 Darwin Kernel Version 23.1.0 x86_64

  ## Available Information Fields

  | Flag | Field            | Source in Erlang/OTP                    |
  |------|------------------|-----------------------------------------|
  | -s   | Kernel name      | `:os.type()` -> {family, _}             |
  | -n   | Node name        | `:inet.gethostname()`                   |
  | -r   | Kernel release   | `:os.cmd(~c"uname -r")`                |
  | -v   | Kernel version   | `:os.cmd(~c"uname -v")`                |
  | -m   | Machine          | `:erlang.system_info(:system_architecture)` |
  | -p   | Processor        | Same as machine on most systems         |
  | -i   | Hardware platform| Same as machine on most systems         |
  | -o   | Operating system | Derived from `:os.type()`               |

  ## Implementation Notes

  We use Erlang/OTP's built-in system information functions where possible.
  For fields not directly available (kernel release, kernel version), we
  fall back to calling the system `uname` command. This is pragmatic --
  these values are OS-specific and not abstracted by the BEAM.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Gather all system information into a map.

  Returns a map with keys: `:kernel_name`, `:nodename`, `:kernel_release`,
  `:kernel_version`, `:machine`, `:processor`, `:hardware_platform`,
  `:operating_system`.

  ## How Each Field Is Obtained

  - **kernel_name**: From `:os.type()`. The atom `:darwin` becomes "Darwin",
    `:linux` becomes "Linux", etc.
  - **nodename**: From `:inet.gethostname()`, the local hostname.
  - **kernel_release**: From `uname -r` system command.
  - **kernel_version**: From `uname -v` system command.
  - **machine**: From `:erlang.system_info(:system_architecture)`, taking
    the first segment (e.g., "x86_64" from "x86_64-apple-darwin23.1.0").
  - **processor**: Same as machine (portable approximation).
  - **hardware_platform**: Same as machine (portable approximation).
  - **operating_system**: "Darwin" on macOS, "GNU/Linux" on Linux.
  """
  def get_system_info do
    {os_family, os_name} = :os.type()

    kernel_name = format_kernel_name(os_family, os_name)
    nodename = get_hostname()
    kernel_release = run_cmd(~c"uname -r")
    kernel_version = run_cmd(~c"uname -v")
    arch = get_architecture()

    operating_system =
      case os_family do
        :unix ->
          case os_name do
            :darwin -> "Darwin"
            :linux -> "GNU/Linux"
            other_name -> other_name |> Atom.to_string() |> String.capitalize()
          end

        :win32 ->
          "Windows"

        _ ->
          Atom.to_string(os_family)
      end

    %{
      kernel_name: kernel_name,
      nodename: nodename,
      kernel_release: kernel_release,
      kernel_version: kernel_version,
      machine: arch,
      processor: arch,
      hardware_platform: arch,
      operating_system: operating_system
    }
  end

  @doc """
  Build the output string from system info and selected fields.

  ## How Field Selection Works

  If no specific field flags are set, only `:kernel_name` is shown.
  If `-a` is set, all fields are shown in a fixed order.
  Otherwise, only the explicitly requested fields are shown.

  The fields are printed space-separated, in the canonical order:
  kernel_name, nodename, kernel_release, kernel_version, machine,
  processor, hardware_platform, operating_system.

  ## Examples

      iex> info = %{kernel_name: "Linux", nodename: "box", ...}
      iex> UnixTools.Uname.format_output(info, %{all: true})
      "Linux box 5.4.0 #1 SMP x86_64 x86_64 x86_64 GNU/Linux"
  """
  def format_output(info, flags) do
    # The canonical field order.
    field_order = [
      {:kernel_name, :kernel_name},
      {:nodename, :nodename},
      {:kernel_release, :kernel_release},
      {:kernel_version, :kernel_version},
      {:machine, :machine},
      {:processor, :processor},
      {:hardware_platform, :hardware_platform},
      {:operating_system, :operating_system}
    ]

    show_all = !!flags[:all]

    # If -a is set, show everything.
    # If no specific flags are set, show kernel_name.
    # Otherwise, show only the flagged fields.
    any_specific =
      Enum.any?([:kernel_name, :nodename, :kernel_release, :kernel_version,
                  :machine, :processor, :hardware_platform, :operating_system],
        fn key -> !!flags[key] end)

    selected =
      if show_all do
        Enum.map(field_order, fn {_flag, info_key} -> Map.get(info, info_key) end)
      else
        if any_specific do
          field_order
          |> Enum.filter(fn {flag_key, _info_key} -> !!flags[flag_key] end)
          |> Enum.map(fn {_flag_key, info_key} -> Map.get(info, info_key) end)
        else
          # Default: just kernel name.
          [info[:kernel_name]]
        end
      end

    Enum.join(selected, " ")
  end

  # ---------------------------------------------------------------------------
  # System Info Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp format_kernel_name(:unix, :darwin), do: "Darwin"
  defp format_kernel_name(:unix, :linux), do: "Linux"
  defp format_kernel_name(:unix, name), do: name |> Atom.to_string() |> String.capitalize()
  defp format_kernel_name(:win32, _), do: "Windows_NT"
  defp format_kernel_name(family, _), do: Atom.to_string(family)

  @doc false
  defp get_hostname do
    case :inet.gethostname() do
      {:ok, hostname} -> to_string(hostname)
      _ -> "unknown"
    end
  end

  @doc false
  defp run_cmd(cmd) do
    cmd
    |> :os.cmd()
    |> to_string()
    |> String.trim()
  end

  @doc false
  defp get_architecture do
    :erlang.system_info(:system_architecture)
    |> to_string()
    |> String.split("-")
    |> List.first()
  end

  # ---------------------------------------------------------------------------
  # Entry Point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["uname" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags}} ->
        flag_map = %{
          all: !!flags["all"],
          kernel_name: !!flags["kernel_name"],
          nodename: !!flags["nodename"],
          kernel_release: !!flags["kernel_release"],
          kernel_version: !!flags["kernel_version"],
          machine: !!flags["machine"],
          processor: !!flags["processor"],
          hardware_platform: !!flags["hardware_platform"],
          operating_system: !!flags["operating_system"]
        }

        info = get_system_info()
        output = format_output(info, flag_map)
        IO.puts(output)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "uname: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "uname.json"),
        else: nil
      ),
      "uname.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "uname.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find uname.json spec file"
  end
end
