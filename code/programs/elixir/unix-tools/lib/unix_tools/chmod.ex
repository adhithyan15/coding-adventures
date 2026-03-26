defmodule UnixTools.Chmod do
  @moduledoc """
  chmod -- change file mode bits.

  ## What This Program Does

  This is a reimplementation of the GNU `chmod` utility in Elixir. It changes
  the permission bits of files and directories.

  ## How File Permissions Work

  Unix file permissions are represented as a 12-bit number. The lower 9 bits
  control read/write/execute for three categories:

  | Bits  | Category | Meaning                              |
  |-------|----------|--------------------------------------|
  | 8-6   | Owner(u) | Owner's read, write, execute         |
  | 5-3   | Group(g) | Group's read, write, execute         |
  | 2-0   | Other(o) | Everyone else's read, write, execute |

  Each category has three permission bits:

  | Bit | Letter | Octal | Meaning   |
  |-----|--------|-------|-----------|
  | r   | read   | 4     | Can read  |
  | w   | write  | 2     | Can write |
  | x   | execute| 1     | Can exec  |

  ## Octal Notation

  Permissions can be specified as an octal number:

      chmod 755 file    =>   rwxr-xr-x  (owner: all, group+other: read+exec)
      chmod 644 file    =>   rw-r--r--  (owner: read+write, group+other: read)

  ## Symbolic Notation

  Or as symbolic expressions:

      chmod u+x file        =>   Add execute for owner
      chmod go-w file       =>   Remove write for group and other
      chmod a=rw file       =>   Set read+write for all (remove everything else)
      chmod u+x,g+r file   =>   Multiple changes separated by commas

  ## Symbolic Mode Grammar

  A symbolic mode has the form: `[ugoa...][+-=][rwxXst...]`

  - **Who**: `u` (user/owner), `g` (group), `o` (other), `a` (all)
  - **Operation**: `+` (add), `-` (remove), `=` (set exactly)
  - **Permissions**: `r` (read), `w` (write), `x` (execute),
    `X` (execute only if directory or already has execute),
    `s` (setuid/setgid), `t` (sticky)

  ## Implementation Approach

  1. `parse_mode/1` determines if the mode is octal or symbolic.
  2. `parse_symbolic/1` parses a symbolic mode string into operations.
  3. `apply_mode/2` computes the new permission bits from the old ones.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Entry point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["chmod" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        opts = %{
          recursive: !!flags["recursive"],
          verbose: !!flags["verbose"],
          changes: !!flags["changes"],
          silent: !!flags["silent"],
          reference: flags["reference"]
        }

        mode_str = arguments["mode"]
        file_list = normalize_files(arguments["files"])

        run(mode_str, file_list, opts)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn err ->
          IO.puts(:stderr, "chmod: #{err.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Mode Parsing
  # ---------------------------------------------------------------------------

  @doc """
  Parse a mode string into a mode specification.

  Returns either `{:octal, integer}` for numeric modes, or
  `{:symbolic, operations}` for symbolic modes.

  ## Examples

      iex> UnixTools.Chmod.parse_mode("755")
      {:octal, 0o755}

      iex> UnixTools.Chmod.parse_mode("0644")
      {:octal, 0o644}

      iex> {:symbolic, ops} = UnixTools.Chmod.parse_mode("u+x")
      iex> length(ops)
      1
  """
  def parse_mode(mode_str) do
    if Regex.match?(~r/^[0-7]+$/, mode_str) do
      {:octal, String.to_integer(mode_str, 8)}
    else
      {:symbolic, parse_symbolic(mode_str)}
    end
  end

  @doc """
  Parse a symbolic mode string into a list of operations.

  Each operation is a map with:
  - `:who` — list of `:user`, `:group`, `:other` (or all three for `a`)
  - `:op` — `:add`, `:remove`, or `:set`
  - `:perms` — list of permission atoms (`:read`, `:write`, `:execute`, etc.)

  ## Parsing Rules

  The symbolic mode `u+rwx,go-w` is split by comma into clauses:
  - `u+rwx` → who: [:user], op: :add, perms: [:read, :write, :execute]
  - `go-w`  → who: [:group, :other], op: :remove, perms: [:write]

  ## Examples

      iex> UnixTools.Chmod.parse_symbolic("u+x")
      [%{who: [:user], op: :add, perms: [:execute]}]

      iex> UnixTools.Chmod.parse_symbolic("a=rw")
      [%{who: [:user, :group, :other], op: :set, perms: [:read, :write]}]
  """
  def parse_symbolic(mode_str) do
    mode_str
    |> String.split(",")
    |> Enum.flat_map(&parse_symbolic_clause/1)
  end

  defp parse_symbolic_clause(clause) do
    # Extract who, operation, and permissions using regex
    case Regex.run(~r/^([ugoa]*)([+\-=])([rwxXst]*)$/, clause) do
      [_full, who_str, op_str, perm_str] ->
        who = parse_who(who_str)
        operation = parse_op(op_str)
        perms = parse_perms(perm_str)

        [%{who: who, op: operation, perms: perms}]

      nil ->
        # Try to handle multiple operations in one clause (e.g., "u+rw-x")
        []
    end
  end

  defp parse_who(""), do: [:user, :group, :other]
  defp parse_who("a"), do: [:user, :group, :other]

  defp parse_who(str) do
    str
    |> String.graphemes()
    |> Enum.map(fn
      "u" -> :user
      "g" -> :group
      "o" -> :other
      "a" -> :all
    end)
    |> Enum.flat_map(fn
      :all -> [:user, :group, :other]
      other -> [other]
    end)
    |> Enum.uniq()
  end

  defp parse_op("+"), do: :add
  defp parse_op("-"), do: :remove
  defp parse_op("="), do: :set

  defp parse_perms(str) do
    str
    |> String.graphemes()
    |> Enum.map(fn
      "r" -> :read
      "w" -> :write
      "x" -> :execute
      "X" -> :conditional_execute
      "s" -> :setuid
      "t" -> :sticky
    end)
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Mode Application
  # ---------------------------------------------------------------------------

  @doc """
  Apply a mode specification to an existing permission value.

  ## Octal Mode

  Octal mode replaces the entire permission value:

      apply_mode({:octal, 0o755}, 0o644) => 0o755

  ## Symbolic Mode

  Symbolic mode modifies specific bits:

      apply_mode({:symbolic, [%{who: [:user], op: :add, perms: [:execute]}]}, 0o644)
      => 0o744

  ## How Symbolic Bit Manipulation Works

  Each who+perm combination maps to a specific bit position:

  | Who   | Perm    | Bit Position | Octal |
  |-------|---------|-------------|-------|
  | user  | read    | 8           | 0o400 |
  | user  | write   | 7           | 0o200 |
  | user  | execute | 6           | 0o100 |
  | group | read    | 5           | 0o040 |
  | group | write   | 4           | 0o020 |
  | group | execute | 3           | 0o010 |
  | other | read    | 2           | 0o004 |
  | other | write   | 1           | 0o002 |
  | other | execute | 0           | 0o001 |

  ## Examples

      iex> UnixTools.Chmod.apply_mode({:octal, 0o755}, 0o644)
      0o755

      iex> UnixTools.Chmod.apply_mode({:symbolic, [%{who: [:user], op: :add, perms: [:execute]}]}, 0o644)
      0o744
  """
  def apply_mode({:octal, new_mode}, _current_mode), do: new_mode

  def apply_mode({:symbolic, operations}, current_mode) do
    Enum.reduce(operations, current_mode, fn %{who: who_list, op: operation, perms: perm_list}, mode ->
      # Compute the bitmask for this operation
      mask = compute_mask(who_list, perm_list)

      case operation do
        :add -> Bitwise.bor(mode, mask)
        :remove -> Bitwise.band(mode, Bitwise.bnot(mask))
        :set ->
          # Clear all bits for the specified who categories, then set new bits
          clear_mask = compute_mask(who_list, [:read, :write, :execute])
          mode |> Bitwise.band(Bitwise.bnot(clear_mask)) |> Bitwise.bor(mask)
      end
    end)
  end

  @doc """
  Compute a bitmask for the given who categories and permissions.

  ## Examples

      iex> UnixTools.Chmod.compute_mask([:user], [:read, :write])
      0o600

      iex> UnixTools.Chmod.compute_mask([:other], [:execute])
      0o001
  """
  def compute_mask(who_list, perm_list) do
    Enum.reduce(who_list, 0, fn who, outer_mask ->
      Enum.reduce(perm_list, outer_mask, fn perm, inner_mask ->
        bit = permission_bit(who, perm)
        Bitwise.bor(inner_mask, bit)
      end)
    end)
  end

  @doc """
  Get the octal bit value for a who+permission combination.

  ## Truth Table (all 9 standard combinations)

  | Who   | Perm    | Octal |
  |-------|---------|-------|
  | user  | read    | 0o400 |
  | user  | write   | 0o200 |
  | user  | execute | 0o100 |
  | group | read    | 0o040 |
  | group | write   | 0o020 |
  | group | execute | 0o010 |
  | other | read    | 0o004 |
  | other | write   | 0o002 |
  | other | execute | 0o001 |
  """
  def permission_bit(:user, :read), do: 0o400
  def permission_bit(:user, :write), do: 0o200
  def permission_bit(:user, :execute), do: 0o100
  def permission_bit(:user, :conditional_execute), do: 0o100
  def permission_bit(:group, :read), do: 0o040
  def permission_bit(:group, :write), do: 0o020
  def permission_bit(:group, :execute), do: 0o010
  def permission_bit(:group, :conditional_execute), do: 0o010
  def permission_bit(:other, :read), do: 0o004
  def permission_bit(:other, :write), do: 0o002
  def permission_bit(:other, :execute), do: 0o001
  def permission_bit(:other, :conditional_execute), do: 0o001
  def permission_bit(:user, :setuid), do: 0o4000
  def permission_bit(:group, :setuid), do: 0o2000
  def permission_bit(_, :sticky), do: 0o1000
  def permission_bit(_, _), do: 0

  @doc """
  Format a permission value as an octal string.

  ## Examples

      iex> UnixTools.Chmod.format_octal_mode(0o755)
      "0755"
  """
  def format_octal_mode(mode) do
    mode
    |> Integer.to_string(8)
    |> String.pad_leading(4, "0")
  end

  # ---------------------------------------------------------------------------
  # Run
  # ---------------------------------------------------------------------------

  defp run(mode_str, file_list, opts) do
    mode_spec =
      if opts[:reference] do
        case File.stat(opts[:reference]) do
          {:ok, stat} -> {:octal, stat.mode |> Bitwise.band(0o7777)}
          {:error, reason} ->
            IO.puts(:stderr, "chmod: cannot stat '#{opts[:reference]}': #{:file.format_error(reason)}")
            System.halt(1)
        end
      else
        parse_mode(mode_str)
      end

    Enum.each(file_list, fn file_path ->
      chmod_file(file_path, mode_spec, opts)
    end)
  end

  defp chmod_file(file_path, mode_spec, opts) do
    case File.stat(file_path) do
      {:ok, stat} ->
        current_mode = stat.mode |> Bitwise.band(0o7777)
        new_mode = apply_mode(mode_spec, current_mode)

        case File.chmod(file_path, new_mode) do
          :ok ->
            changed = current_mode != new_mode

            cond do
              opts[:verbose] ->
                IO.puts("mode of '#{file_path}' changed from #{format_octal_mode(current_mode)} to #{format_octal_mode(new_mode)}")

              opts[:changes] and changed ->
                IO.puts("mode of '#{file_path}' changed from #{format_octal_mode(current_mode)} to #{format_octal_mode(new_mode)}")

              true ->
                :ok
            end

          {:error, reason} ->
            unless opts[:silent] do
              IO.puts(:stderr, "chmod: changing permissions of '#{file_path}': #{:file.format_error(reason)}")
            end
        end

        # Recurse into directories if -R is set
        if opts[:recursive] and stat.type == :directory do
          case File.ls(file_path) do
            {:ok, entries} ->
              Enum.each(entries, fn entry ->
                chmod_file(Path.join(file_path, entry), mode_spec, opts)
              end)

            {:error, reason} ->
              unless opts[:silent] do
                IO.puts(:stderr, "chmod: cannot open directory '#{file_path}': #{:file.format_error(reason)}")
              end
          end
        end

      {:error, reason} ->
        unless opts[:silent] do
          IO.puts(:stderr, "chmod: cannot access '#{file_path}': #{:file.format_error(reason)}")
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]
  defp normalize_files(nil), do: []

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "chmod.json"),
        else: nil
      ),
      "chmod.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "chmod.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      path -> File.exists?(path)
    end) ||
      raise "Could not find chmod.json spec file"
  end
end
