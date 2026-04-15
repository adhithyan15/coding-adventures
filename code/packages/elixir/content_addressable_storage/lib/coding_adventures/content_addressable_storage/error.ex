defmodule CodingAdventures.ContentAddressableStorage.Error do
  @moduledoc """
  Typed error reasons returned by `CodingAdventures.ContentAddressableStorage.Store`.

  ## Why Typed Errors?

  Rather than returning raw strings or opaque `{:error, term()}` tuples,
  the Store layer uses a small vocabulary of structured reasons. This allows
  callers to pattern-match on specific failure modes and take appropriate action:

  - A `:not_found` might cause a fallback fetch from a remote.
  - A `{:corrupted, key}` should trigger an alert or repair procedure.
  - An `{:ambiguous_prefix, prefix}` means the user typed too few characters.

  ## Error Catalogue

  | Reason                        | Meaning                                              |
  |-------------------------------|------------------------------------------------------|
  | `:not_found`                  | Requested key is not in the store                    |
  | `{:corrupted, key}`           | Stored bytes don't hash to the key (data integrity)  |
  | `{:ambiguous_prefix, prefix}` | Hex prefix matched two or more keys                  |
  | `{:prefix_not_found, prefix}` | Hex prefix matched zero keys                         |
  | `{:invalid_prefix, prefix}`   | Hex string contains non-hex characters or is empty   |
  | `{:store_error, reason}`      | Backend (BlobStore) returned an unexpected error     |

  These are *reason terms*, not structs — they are used directly in
  `{:error, reason}` tuples. For example:

      {:error, :not_found}
      {:error, {:corrupted, <<0xa3, 0xf4, ...>>}}
      {:error, {:ambiguous_prefix, "a3f4"}}
  """

  @typedoc "Key is absent from the store."
  @type not_found :: :not_found

  @typedoc "Stored bytes do not hash to the expected key."
  @type corrupted :: {:corrupted, binary()}

  @typedoc "Hex prefix matched two or more keys."
  @type ambiguous_prefix :: {:ambiguous_prefix, String.t()}

  @typedoc "Hex prefix matched no keys."
  @type prefix_not_found :: {:prefix_not_found, String.t()}

  @typedoc "Hex prefix is not valid hexadecimal, or is empty."
  @type invalid_prefix :: {:invalid_prefix, String.t()}

  @typedoc "The BlobStore backend returned an unexpected error."
  @type store_error :: {:store_error, term()}

  @typedoc "Union of all CAS error reasons."
  @type t ::
          not_found()
          | corrupted()
          | ambiguous_prefix()
          | prefix_not_found()
          | invalid_prefix()
          | store_error()

  @doc """
  Format a CAS error reason as a human-readable string.

  Useful for logging and error messages.

      iex> CodingAdventures.ContentAddressableStorage.Error.format(:not_found)
      "object not found"

      iex> CodingAdventures.ContentAddressableStorage.Error.format({:invalid_prefix, "xyz!"})
      "invalid hex prefix: \\"xyz!\\""
  """
  @spec format(t()) :: String.t()
  def format(:not_found), do: "object not found"

  def format({:corrupted, key}) when is_binary(key) do
    hex = Base.encode16(key, case: :lower)
    "object corrupted: #{hex}"
  end

  def format({:ambiguous_prefix, prefix}), do: "ambiguous prefix: #{prefix}"
  def format({:prefix_not_found, prefix}), do: "object not found for prefix: #{prefix}"
  def format({:invalid_prefix, prefix}), do: "invalid hex prefix: #{inspect(prefix)}"
  def format({:store_error, reason}), do: "store error: #{inspect(reason)}"
  def format(other), do: "cas error: #{inspect(other)}"
end
