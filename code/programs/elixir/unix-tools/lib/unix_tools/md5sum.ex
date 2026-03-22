defmodule UnixTools.Md5sum do
  @moduledoc """
  md5sum -- compute and check MD5 message digest.

  ## What This Program Does

  This is a reimplementation of the GNU `md5sum` utility in Elixir. It
  computes MD5 message digests for files or stdin, and can verify previously
  computed checksums.

  ## How MD5 Works (Simplified)

  MD5 (Message Digest Algorithm 5) takes an arbitrary-length input and
  produces a fixed 128-bit (16-byte) hash value, typically displayed as
  a 32-character hexadecimal string.

  The algorithm works in rounds:

  1. **Padding**: The message is padded to a multiple of 512 bits.
  2. **Processing**: The padded message is processed in 512-bit blocks.
  3. **Output**: Four 32-bit state variables are concatenated into the
     128-bit digest.

  ## MD5 Security Warning

  MD5 is **cryptographically broken** and should NOT be used for security
  purposes. Collisions (two different inputs producing the same hash) can
  be generated in seconds on modern hardware. Use SHA-256 or better for
  any security-sensitive application.

  MD5 is still useful for:
  - Verifying file integrity (accidental corruption, not malicious tampering).
  - Quick content comparison.
  - Legacy systems compatibility.

  ## Output Format

      md5sum file.txt     =>   d41d8cd98f00b204e9800998ecf8427e  file.txt

  The hash is followed by two spaces and the filename. In binary mode (-b),
  the separator is ` *` instead of `  `.

  ## Check Mode (-c)

  In check mode, md5sum reads a file containing hash/filename pairs and
  verifies each file against its recorded hash:

      md5sum -c checksums.md5    =>   file1.txt: OK
                                       file2.txt: FAILED

  ## Implementation

  We use Erlang/OTP's `:crypto.hash(:md5, data)` which provides a
  battle-tested, C-implemented MD5 function via OpenSSL.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Compute the MD5 hash of binary data.

  Returns the hash as a lowercase hexadecimal string (32 characters).

  ## How It Works

  We delegate to `:crypto.hash(:md5, data)` which returns a 16-byte
  binary. We then convert each byte to its 2-character hex representation.

  ## Examples

      iex> UnixTools.Md5sum.compute_md5("")
      "d41d8cd98f00b204e9800998ecf8427e"

      iex> UnixTools.Md5sum.compute_md5("hello\\n")
      "b1946ac92492d2347c6235b4d2611184"
  """
  def compute_md5(data) do
    :crypto.hash(:md5, data)
    |> Base.encode16(case: :lower)
  end

  @doc """
  Compute the MD5 hash of a file.

  Reads the file and computes its MD5 hash. Returns `{:ok, hash}` or
  `{:error, reason}`.

  ## Streaming vs Loading

  For simplicity, we read the entire file into memory. For very large
  files, a streaming approach using `:crypto.hash_init/1` and
  `:crypto.hash_update/2` would be more memory-efficient.

  ## Examples

      iex> {:ok, hash} = UnixTools.Md5sum.compute_md5_file("/dev/null")
      iex> hash
      "d41d8cd98f00b204e9800998ecf8427e"
  """
  def compute_md5_file(file_path) do
    case File.read(file_path) do
      {:ok, content} -> {:ok, compute_md5(content)}
      {:error, reason} -> {:error, reason}
    end
  end

  @doc """
  Format an MD5 hash result for output.

  ## Output Format

  - Text mode (default): `hash  filename` (two spaces)
  - Binary mode: `hash *filename` (space + asterisk)

  ## Examples

      iex> UnixTools.Md5sum.format_hash("abc123", "file.txt", false)
      "abc123  file.txt"

      iex> UnixTools.Md5sum.format_hash("abc123", "file.txt", true)
      "abc123 *file.txt"
  """
  def format_hash(hash, filename, binary_mode \\ false) do
    if binary_mode do
      "#{hash} *#{filename}"
    else
      "#{hash}  #{filename}"
    end
  end

  @doc """
  Parse a checksum line for verification (check mode).

  Parses lines in the format `hash  filename` or `hash *filename`.

  Returns `{:ok, {hash, filename}}` or `{:error, :invalid_format}`.

  ## Examples

      iex> UnixTools.Md5sum.parse_checksum_line("abc123  file.txt")
      {:ok, {"abc123", "file.txt"}}

      iex> UnixTools.Md5sum.parse_checksum_line("abc123 *file.txt")
      {:ok, {"abc123", "file.txt"}}
  """
  def parse_checksum_line(line) do
    # Try text mode format: "hash  filename"
    case Regex.run(~r/^([0-9a-fA-F]+)\s+\*?(.+)$/, String.trim(line)) do
      [_full, hash, filename] -> {:ok, {String.downcase(hash), filename}}
      nil -> {:error, :invalid_format}
    end
  end

  @doc """
  Verify a file against an expected hash.

  Returns `:ok` if the hash matches, `{:error, :mismatch}` if it doesn't,
  or `{:error, reason}` if the file can't be read.
  """
  def verify_file(file_path, expected_hash) do
    case compute_md5_file(file_path) do
      {:ok, actual_hash} ->
        if actual_hash == String.downcase(expected_hash) do
          :ok
        else
          {:error, :mismatch}
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  # ---------------------------------------------------------------------------
  # Entry Point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["md5sum" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        check_mode = !!flags["check"]
        binary_mode = !!flags["binary"]

        file_list = normalize_files(arguments["files"])

        if check_mode do
          # Verification mode: read checksum files and verify.
          Enum.each(file_list, fn checksum_file ->
            lines = read_lines(checksum_file)

            Enum.each(lines, fn line ->
              case parse_checksum_line(line) do
                {:ok, {expected_hash, filename}} ->
                  case verify_file(filename, expected_hash) do
                    :ok ->
                      IO.puts("#{filename}: OK")

                    {:error, :mismatch} ->
                      IO.puts("#{filename}: FAILED")

                    {:error, reason} ->
                      IO.puts(:stderr, "md5sum: #{filename}: #{:file.format_error(reason)}")
                  end

                {:error, :invalid_format} ->
                  IO.puts(:stderr, "md5sum: invalid checksum line")
              end
            end)
          end)
        else
          # Compute mode: hash each file and output.
          Enum.each(file_list, fn file_path ->
            case file_path do
              "-" ->
                data =
                  case IO.read(:stdio, :eof) do
                    {:error, _} -> ""
                    :eof -> ""
                    content -> content
                  end

                hash = compute_md5(data)
                IO.puts(format_hash(hash, "-", binary_mode))

              _ ->
                case compute_md5_file(file_path) do
                  {:ok, hash} ->
                    IO.puts(format_hash(hash, file_path, binary_mode))

                  {:error, reason} ->
                    IO.puts(:stderr, "md5sum: #{file_path}: #{:file.format_error(reason)}")
                end
            end
          end)
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "md5sum: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp read_lines("-") do
    case IO.read(:stdio, :eof) do
      {:error, _} -> []
      :eof -> []
      data -> String.split(data, "\n", trim: true)
    end
  end

  defp read_lines(file_path) do
    case File.read(file_path) do
      {:ok, content} -> String.split(content, "\n", trim: true)
      {:error, reason} ->
        IO.puts(:stderr, "md5sum: #{file_path}: #{:file.format_error(reason)}")
        []
    end
  end

  @doc false
  defp normalize_files(nil), do: ["-"]
  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "md5sum.json"),
        else: nil
      ),
      "md5sum.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "md5sum.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find md5sum.json spec file"
  end
end
