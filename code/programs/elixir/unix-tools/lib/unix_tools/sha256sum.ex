defmodule UnixTools.Sha256sum do
  @moduledoc """
  sha256sum -- compute and check SHA-256 message digest.

  ## What This Program Does

  This is a reimplementation of the GNU `sha256sum` utility in Elixir. It
  computes SHA-256 message digests for files or stdin, and can verify
  previously computed checksums.

  ## How SHA-256 Works (Simplified)

  SHA-256 (Secure Hash Algorithm 256-bit) is part of the SHA-2 family
  designed by the NSA. It takes an arbitrary-length input and produces
  a fixed 256-bit (32-byte) hash value, displayed as a 64-character
  hexadecimal string.

  The algorithm:

  1. **Padding**: Message is padded to a multiple of 512 bits.
  2. **Parsing**: Padded message is split into 512-bit blocks.
  3. **Compression**: Each block is processed using 64 rounds of
     bitwise operations, additions, and rotations.
  4. **Output**: Eight 32-bit state variables are concatenated.

  ## SHA-256 vs MD5

  | Property      | MD5         | SHA-256      |
  |---------------|-------------|--------------|
  | Output size   | 128 bits    | 256 bits     |
  | Hex length    | 32 chars    | 64 chars     |
  | Security      | Broken      | Secure       |
  | Speed         | Faster      | Slower       |
  | Collision     | Easy        | Infeasible   |

  SHA-256 should be preferred over MD5 for any application where
  security matters.

  ## Output Format

      sha256sum file.txt  =>  e3b0c44298fc...  file.txt

  Same format as md5sum: hash, two spaces, filename.

  ## Implementation

  We use Erlang/OTP's `:crypto.hash(:sha256, data)` which provides a
  hardware-accelerated SHA-256 implementation via OpenSSL.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Compute the SHA-256 hash of binary data.

  Returns the hash as a lowercase hexadecimal string (64 characters).

  ## How It Works

  We delegate to `:crypto.hash(:sha256, data)` which returns a 32-byte
  binary. We then convert each byte to its 2-character hex representation.

  ## The Empty String Hash

  The SHA-256 hash of the empty string is a well-known constant:
  `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`

  This is useful as a test vector to verify correct implementation.

  ## Examples

      iex> UnixTools.Sha256sum.compute_sha256("")
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

      iex> UnixTools.Sha256sum.compute_sha256("hello\\n")
      "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
  """
  def compute_sha256(data) do
    :crypto.hash(:sha256, data)
    |> Base.encode16(case: :lower)
  end

  @doc """
  Compute the SHA-256 hash of a file.

  Reads the file and computes its SHA-256 hash. Returns `{:ok, hash}` or
  `{:error, reason}`.
  """
  def compute_sha256_file(file_path) do
    case File.read(file_path) do
      {:ok, content} -> {:ok, compute_sha256(content)}
      {:error, reason} -> {:error, reason}
    end
  end

  @doc """
  Format a SHA-256 hash result for output.

  ## Output Format

  - Text mode (default): `hash  filename` (two spaces)
  - Binary mode: `hash *filename` (space + asterisk)

  ## Examples

      iex> UnixTools.Sha256sum.format_hash("abc123", "file.txt", false)
      "abc123  file.txt"

      iex> UnixTools.Sha256sum.format_hash("abc123", "file.txt", true)
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

      iex> UnixTools.Sha256sum.parse_checksum_line("abc123  file.txt")
      {:ok, {"abc123", "file.txt"}}
  """
  def parse_checksum_line(line) do
    case Regex.run(~r/^([0-9a-fA-F]+)\s+\*?(.+)$/, String.trim(line)) do
      [_full, hash, filename] -> {:ok, {String.downcase(hash), filename}}
      nil -> {:error, :invalid_format}
    end
  end

  @doc """
  Verify a file against an expected SHA-256 hash.

  Returns `:ok` if the hash matches, `{:error, :mismatch}` if it doesn't,
  or `{:error, reason}` if the file can't be read.
  """
  def verify_file(file_path, expected_hash) do
    case compute_sha256_file(file_path) do
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

    case Parser.parse(spec_path, ["sha256sum" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        check_mode = !!flags["check"]
        binary_mode = !!flags["binary"]

        file_list = normalize_files(arguments["files"])

        if check_mode do
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
                      IO.puts(:stderr, "sha256sum: #{filename}: #{:file.format_error(reason)}")
                  end

                {:error, :invalid_format} ->
                  IO.puts(:stderr, "sha256sum: invalid checksum line")
              end
            end)
          end)
        else
          Enum.each(file_list, fn file_path ->
            case file_path do
              "-" ->
                data =
                  case IO.read(:stdio, :eof) do
                    {:error, _} -> ""
                    :eof -> ""
                    content -> content
                  end

                hash = compute_sha256(data)
                IO.puts(format_hash(hash, "-", binary_mode))

              _ ->
                case compute_sha256_file(file_path) do
                  {:ok, hash} ->
                    IO.puts(format_hash(hash, file_path, binary_mode))

                  {:error, reason} ->
                    IO.puts(:stderr, "sha256sum: #{file_path}: #{:file.format_error(reason)}")
                end
            end
          end)
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "sha256sum: #{e.message}")
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
        IO.puts(:stderr, "sha256sum: #{file_path}: #{:file.format_error(reason)}")
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
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "sha256sum.json"),
        else: nil
      ),
      "sha256sum.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "sha256sum.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find sha256sum.json spec file"
  end
end
