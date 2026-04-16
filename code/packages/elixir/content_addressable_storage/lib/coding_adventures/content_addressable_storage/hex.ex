defmodule CodingAdventures.ContentAddressableStorage.Hex do
  @moduledoc """
  Hex encoding/decoding utilities for 20-byte SHA-1 keys.

  ## Why a Separate Module?

  Both `Store` and `LocalDiskStore` need to convert between binary keys and hex
  strings. Centralising these small utilities avoids repetition and makes the
  logic testable in isolation.

  ## Key ↔ Hex

  A 20-byte binary key is represented as a 40-character lowercase hex string,
  exactly as Git does:

      <<0xa3, 0xf4, ...>>  ←→  "a3f4..."

  ## Prefix Decoding

  `decode_hex_prefix/1` handles *abbreviated* hex strings of 1–40 characters.
  This is used by `Store.find_by_prefix/2` to resolve short hashes like `git
  show a3f4`.

  Odd-length strings are right-padded with `"0"` before byte conversion, because
  a single hex nibble represents the *high* nibble of a byte:

      "a3f"  →  pad  →  "a3f0"  →  bytes  →  <<0xa3, 0xf0>>

  So `"a3f"` matches any key beginning with `0xa3, 0xf_` — the low nibble is
  don't-care. The prefix byte `0xf0` acts as a mask on the second byte.
  """

  @doc """
  Convert a 20-byte binary key to a 40-character lowercase hex string.

  ## Examples

      iex> CodingAdventures.ContentAddressableStorage.Hex.key_to_hex(<<0xda, 0x39, 0xa3, 0xee, 0x5e, 0x6b, 0x4b, 0x0d, 0x32, 0x55, 0xbf, 0xef, 0x95, 0x60, 0x18, 0x90, 0xaf, 0xd8, 0x07, 0x09>>)
      "da39a3ee5e6b4b0d3255bfef95601890afd80709"
  """
  @spec key_to_hex(binary()) :: String.t()
  def key_to_hex(key) when byte_size(key) == 20 do
    Base.encode16(key, case: :lower)
  end

  @doc """
  Parse a 40-character lowercase or uppercase hex string into a 20-byte binary.

  Returns `{:ok, key}` on success, `{:error, reason}` if the string is not
  exactly 40 valid hex characters.

  ## Examples

      iex> CodingAdventures.ContentAddressableStorage.Hex.hex_to_key("da39a3ee5e6b4b0d3255bfef95601890afd80709")
      {:ok, <<0xda, 0x39, 0xa3, 0xee, 0x5e, 0x6b, 0x4b, 0x0d, 0x32, 0x55, 0xbf, 0xef, 0x95, 0x60, 0x18, 0x90, 0xaf, 0xd8, 0x07, 0x09>>}

      iex> CodingAdventures.ContentAddressableStorage.Hex.hex_to_key("not-valid")
      {:error, :invalid_hex}
  """
  @spec hex_to_key(String.t()) :: {:ok, binary()} | {:error, :invalid_hex}
  def hex_to_key(hex) when is_binary(hex) and byte_size(hex) == 40 do
    case Base.decode16(hex, case: :mixed) do
      {:ok, key} -> {:ok, key}
      :error -> {:error, :invalid_hex}
    end
  end

  def hex_to_key(_), do: {:error, :invalid_hex}

  @doc """
  Decode an abbreviated hex string (1–40 characters) to a byte-prefix binary.

  Odd-length strings are right-padded with `"0"` before decoding, so that a
  nibble prefix like `"a3f"` becomes `<<0xa3, 0xf0>>` — matching any key that
  starts with `0xa3` in byte 0 and has `0xf` as the high nibble of byte 1.

  Returns `{:ok, prefix_bytes}` or `{:error, :invalid_hex}` if any character
  is not a hex digit, or the string is empty.

  ## Examples

      iex> CodingAdventures.ContentAddressableStorage.Hex.decode_hex_prefix("a3f4")
      {:ok, <<0xa3, 0xf4>>}

      iex> CodingAdventures.ContentAddressableStorage.Hex.decode_hex_prefix("a3f")
      {:ok, <<0xa3, 0xf0>>}

      iex> CodingAdventures.ContentAddressableStorage.Hex.decode_hex_prefix("")
      {:error, :invalid_hex}

      iex> CodingAdventures.ContentAddressableStorage.Hex.decode_hex_prefix("zz")
      {:error, :invalid_hex}
  """
  @spec decode_hex_prefix(String.t()) :: {:ok, binary()} | {:error, :invalid_hex}
  def decode_hex_prefix(""), do: {:error, :invalid_hex}

  def decode_hex_prefix(hex) when is_binary(hex) do
    # Validate that every character is a hex digit before doing anything else.
    # This prevents Base.decode16 from silently accepting non-hex characters.
    if hex_string?(hex) do
      # Pad to even length. An odd-length string means the last character is a
      # high-nibble prefix — pad with "0" to form a complete byte.
      padded = if rem(byte_size(hex), 2) == 1, do: hex <> "0", else: hex

      case Base.decode16(padded, case: :mixed) do
        {:ok, bytes} -> {:ok, bytes}
        :error -> {:error, :invalid_hex}
      end
    else
      {:error, :invalid_hex}
    end
  end

  # Returns true if every byte in `str` is a valid hex digit [0-9a-fA-F].
  # We use a bitstring comprehension to check each byte.
  @spec hex_string?(String.t()) :: boolean()
  defp hex_string?(str) do
    str
    |> :binary.bin_to_list()
    |> Enum.all?(&hex_nibble?/1)
  end

  # A single ASCII byte is a valid hex nibble if it is 0-9, a-f, or A-F.
  @spec hex_nibble?(byte()) :: boolean()
  defp hex_nibble?(b) when b >= ?0 and b <= ?9, do: true
  defp hex_nibble?(b) when b >= ?a and b <= ?f, do: true
  defp hex_nibble?(b) when b >= ?A and b <= ?F, do: true
  defp hex_nibble?(_), do: false
end
