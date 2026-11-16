defmodule CodingAdventures.CanonicalCbor.CborError do
  @moduledoc """
  Exception representing any RFC 8949 canonical CBOR decoding or encoding violation.
  Always wraps a closed payload-blind diagnostic message from `Messages`.
  """
  defexception [:id, :message]

  alias CodingAdventures.CanonicalCbor.Messages

  @type t :: %__MODULE__{
          id: atom() | String.t(),
          message: String.t()
        }

  @impl true
  def exception(id) when is_atom(id) or is_binary(id) do
    msg = Messages.message_for(id)
    %__MODULE__{id: id, message: msg}
  end

  def exception(opts) when is_list(opts) do
    id = Keyword.get(opts, :id, :unknown)
    msg = Messages.message_for(id)
    %__MODULE__{id: id, message: msg}
  end

  def exception(_other) do
    %__MODULE__{id: :unknown, message: Messages.unknown_error_id_message()}
  end
end

defmodule CodingAdventures.CanonicalCbor.Span do
  @moduledoc """
  Half-open wire byte offset span `[start, finish)`.
  """
  @enforce_keys [:start, :finish]
  defstruct [:start, :finish]

  alias CodingAdventures.CanonicalCbor.{CborError, Constants}

  @type t :: %__MODULE__{
          start: non_neg_integer(),
          finish: non_neg_integer()
        }

  @doc """
  Construct and validate a `Span`. Raises `CborError` on invalid offsets.
  """
  @spec new(term(), term()) :: t()
  def new(start, finish) do
    if is_integer(start) and start >= 0 and is_integer(finish) and finish >= start and
         finish <= Constants.max_uint64() do
      %__MODULE__{start: start, finish: finish}
    else
      raise CborError, :unsupported_simple
    end
  end

  @doc """
  Return the byte length of the span.
  """
  @spec length(t()) :: non_neg_integer()
  def length(%__MODULE__{start: s, finish: f}), do: f - s
end

defimpl Inspect, for: CodingAdventures.CanonicalCbor.Span do
  alias CodingAdventures.CanonicalCbor.Messages

  def inspect(%{start: s, finish: f}, _opts) when is_integer(s) and is_integer(f) do
    "[#{s}, #{f})"
  end

  def inspect(_other, _opts) do
    "Span(#{Messages.invalid_placeholder()})"
  end
end

defmodule CodingAdventures.CanonicalCbor.SpannedMapEntry do
  @moduledoc """
  Map entry with authentic CBOR key and value, plus exact byte offset span
  `[start, finish)` of the raw value wire slice. Redacts key and value on stringification.
  """
  @enforce_keys [:key, :value, :value_span]
  defstruct [:key, :value, :value_span]

  alias CodingAdventures.CanonicalCbor.{CborError, Span, Types}

  @type t :: %__MODULE__{
          key: Types.cbor_value(),
          value: Types.cbor_value(),
          value_span: Span.t()
        }

  @doc """
  Construct and validate a `SpannedMapEntry`. Raises `CborError` on invalid arguments.
  """
  @spec new(term(), term(), term()) :: t()
  def new(key, value, value_span) do
    if Types.valid_cbor_node?(key) and Types.valid_cbor_node?(value) and
         Types.is_span?(value_span) do
      %__MODULE__{key: key, value: value, value_span: value_span}
    else
      raise CborError, :unsupported_simple
    end
  end
end

defimpl Inspect, for: CodingAdventures.CanonicalCbor.SpannedMapEntry do
  alias CodingAdventures.CanonicalCbor.{Messages, Span}

  def inspect(%{value_span: %Span{start: s, finish: f}}, _opts)
      when is_integer(s) and is_integer(f) do
    ph = Messages.redacted_placeholder()
    "SpannedMapEntry(key: #{ph}, value: #{ph}, value_span: [#{s}, #{f}))"
  end

  def inspect(_other, _opts) do
    ph = Messages.redacted_placeholder()
    inv = Messages.invalid_placeholder()
    "SpannedMapEntry(key: #{ph}, value: #{ph}, value_span: #{inv})"
  end
end

defmodule CodingAdventures.CanonicalCbor.DecodeMapSpansResult do
  @moduledoc """
  Result of `decode_map_spans/1`. Distinguishes authentic maps from non-map items
  without boolean primitive flags.
  """
  @enforce_keys [:is_map, :entries]
  defstruct [:is_map, :entries]

  alias CodingAdventures.CanonicalCbor.{CborError, SpannedMapEntry, Types}

  @type t :: %__MODULE__{
          is_map: boolean(),
          entries: [SpannedMapEntry.t()] | nil
        }

  @doc """
  Construct a map result containing spanned entries.
  """
  @spec map(term()) :: t()
  def map(entries) do
    if is_list(entries) and
         Enum.all?(entries, fn
           %SpannedMapEntry{key: k, value: v, value_span: s} ->
             Types.valid_cbor_node?(k) and Types.valid_cbor_node?(v) and Types.is_span?(s)

           _ ->
             false
         end) do
      %__MODULE__{is_map: true, entries: entries}
    else
      raise CborError, :unsupported_simple
    end
  end

  @doc """
  Construct a non-map result.
  """
  @spec non_map() :: t()
  def non_map do
    %__MODULE__{is_map: false, entries: nil}
  end
end

defimpl Inspect, for: CodingAdventures.CanonicalCbor.DecodeMapSpansResult do
  alias CodingAdventures.CanonicalCbor.Messages

  def inspect(%{is_map: true, entries: e}, _opts) when is_list(e) do
    "DecodeMapSpansResult.map(##{Kernel.length(e)})"
  end

  def inspect(%{is_map: false}, _opts) do
    "DecodeMapSpansResult.non_map"
  end

  def inspect(_other, _opts) do
    "DecodeMapSpansResult(#{Messages.invalid_placeholder()})"
  end
end

defmodule CodingAdventures.CanonicalCbor.Types.CborUnsigned do
  @moduledoc "Major Type 0: Unsigned 64-bit integer."
  @enforce_keys [:value]
  defstruct [:value]
  @type t :: %__MODULE__{value: non_neg_integer()}
end

defimpl Inspect, for: CodingAdventures.CanonicalCbor.Types.CborUnsigned do
  alias CodingAdventures.CanonicalCbor.Messages
  def inspect(%{value: v}, _opts) when is_integer(v) and v >= 0, do: "CborUnsigned(#{v})"
  def inspect(_other, _opts), do: "CborUnsigned(#{Messages.invalid_placeholder()})"
end

defmodule CodingAdventures.CanonicalCbor.Types.CborNegative do
  @moduledoc "Major Type 1: Negative 64-bit integer representing -1 - value."
  @enforce_keys [:value]
  defstruct [:value]
  @type t :: %__MODULE__{value: non_neg_integer()}
end

defimpl Inspect, for: CodingAdventures.CanonicalCbor.Types.CborNegative do
  alias CodingAdventures.CanonicalCbor.Messages
  def inspect(%{value: v}, _opts) when is_integer(v) and v >= 0, do: "CborNegative(#{v})"
  def inspect(_other, _opts), do: "CborNegative(#{Messages.invalid_placeholder()})"
end

defmodule CodingAdventures.CanonicalCbor.Types.CborBytes do
  @moduledoc "Major Type 2: Byte string."
  @enforce_keys [:bytes]
  defstruct [:bytes]
  @type t :: %__MODULE__{bytes: binary()}
end

defimpl Inspect, for: CodingAdventures.CanonicalCbor.Types.CborBytes do
  alias CodingAdventures.CanonicalCbor.Messages
  def inspect(%{bytes: b}, _opts) when is_binary(b), do: "CborBytes(##{byte_size(b)})"
  def inspect(_other, _opts), do: "CborBytes(#{Messages.invalid_placeholder()})"
end

defmodule CodingAdventures.CanonicalCbor.Types.CborText do
  @moduledoc "Major Type 3: UTF-8 text string."
  @enforce_keys [:text]
  defstruct [:text]
  @type t :: %__MODULE__{text: String.t()}
end

defimpl Inspect, for: CodingAdventures.CanonicalCbor.Types.CborText do
  alias CodingAdventures.CanonicalCbor.Messages

  def inspect(%{text: t}, _opts) when is_binary(t) do
    if String.valid?(t) do
      "CborText(#{inspect(t)})"
    else
      "CborText(#{Messages.invalid_placeholder()})"
    end
  end

  def inspect(_other, _opts), do: "CborText(#{Messages.invalid_placeholder()})"
end

defmodule CodingAdventures.CanonicalCbor.Types.CborArray do
  @moduledoc "Major Type 4: Homogeneous/heterogeneous CBOR array."
  @enforce_keys [:items]
  defstruct [:items]

  alias CodingAdventures.CanonicalCbor.{CborError, Types}

  @type t :: %__MODULE__{items: [CodingAdventures.CanonicalCbor.Types.cbor_value()]}

  @doc "Construct and validate a `CborArray`. Raises `CborError` on invalid arguments."
  @spec new(term()) :: t()
  def new(items) do
    if is_list(items) and Enum.all?(items, &Types.valid_cbor_node?/1) do
      %__MODULE__{items: items}
    else
      raise CborError, :unsupported_simple
    end
  end
end

defimpl Inspect, for: CodingAdventures.CanonicalCbor.Types.CborArray do
  alias CodingAdventures.CanonicalCbor.Messages
  def inspect(%{items: it}, _opts) when is_list(it), do: "CborArray(##{Kernel.length(it)})"
  def inspect(_other, _opts), do: "CborArray(#{Messages.invalid_placeholder()})"
end

defmodule CodingAdventures.CanonicalCbor.Types.CborMapEntry do
  @moduledoc "Key-value pair descriptor for CborMap. NOT a CborValue."
  @enforce_keys [:key, :value]
  defstruct [:key, :value]

  alias CodingAdventures.CanonicalCbor.{CborError, Types}

  @type t :: %__MODULE__{
          key: CodingAdventures.CanonicalCbor.Types.cbor_value(),
          value: CodingAdventures.CanonicalCbor.Types.cbor_value()
        }

  @doc "Construct and validate a `CborMapEntry`. Raises `CborError` on invalid arguments."
  @spec new(term(), term()) :: t()
  def new(key, value) do
    if Types.valid_cbor_node?(key) and Types.valid_cbor_node?(value) do
      %__MODULE__{key: key, value: value}
    else
      raise CborError, :unsupported_simple
    end
  end
end

defimpl Inspect, for: CodingAdventures.CanonicalCbor.Types.CborMapEntry do
  import Inspect.Algebra
  alias CodingAdventures.CanonicalCbor.Messages

  def inspect(%{key: k, value: v}, opts) do
    concat(["CborMapEntry(", to_doc(k, opts), " => ", to_doc(v, opts), ")"])
  rescue
    _ -> "CborMapEntry(#{Messages.invalid_placeholder()})"
  end

  def inspect(_other, _opts), do: "CborMapEntry(#{Messages.invalid_placeholder()})"
end

defmodule CodingAdventures.CanonicalCbor.Types.CborMap do
  @moduledoc "Major Type 5: Map of canonical entries."
  @enforce_keys [:entries]
  defstruct [:entries]

  alias CodingAdventures.CanonicalCbor.{CborError, Types, Types.CborMapEntry}

  @type t :: %__MODULE__{entries: [CborMapEntry.t()]}

  @doc "Construct and validate a `CborMap`. Raises `CborError` on invalid arguments."
  @spec new(term()) :: t()
  def new(entries) do
    if is_list(entries) and
         Enum.all?(entries, fn
           %CborMapEntry{key: k, value: v} ->
             Types.valid_cbor_node?(k) and Types.valid_cbor_node?(v)

           _ ->
             false
         end) do
      %__MODULE__{entries: entries}
    else
      raise CborError, :unsupported_simple
    end
  end
end

defimpl Inspect, for: CodingAdventures.CanonicalCbor.Types.CborMap do
  alias CodingAdventures.CanonicalCbor.Messages
  def inspect(%{entries: e}, _opts) when is_list(e), do: "CborMap(##{Kernel.length(e)})"
  def inspect(_other, _opts), do: "CborMap(#{Messages.invalid_placeholder()})"
end

defmodule CodingAdventures.CanonicalCbor.Types.CborTag do
  @moduledoc "Major Type 6: Semantic tag (tag number + inner CBOR item)."
  @enforce_keys [:tag, :value]
  defstruct [:tag, :value]

  alias CodingAdventures.CanonicalCbor.{CborError, Constants, Types}

  @type t :: %__MODULE__{
          tag: non_neg_integer(),
          value: CodingAdventures.CanonicalCbor.Types.cbor_value()
        }

  @doc "Construct and validate a `CborTag`. Raises `CborError` on invalid arguments."
  @spec new(term(), term()) :: t()
  def new(tag, value) do
    if is_integer(tag) and tag >= 0 and tag <= Constants.max_uint64() and
         Types.valid_cbor_node?(value) do
      %__MODULE__{tag: tag, value: value}
    else
      raise CborError, :unsupported_simple
    end
  end
end

defimpl Inspect, for: CodingAdventures.CanonicalCbor.Types.CborTag do
  import Inspect.Algebra
  alias CodingAdventures.CanonicalCbor.Messages

  def inspect(%{tag: t, value: v}, opts) when is_integer(t) and t >= 0 do
    concat(["CborTag(#{t}, ", to_doc(v, opts), ")"])
  rescue
    _ -> "CborTag(#{Messages.invalid_placeholder()})"
  end

  def inspect(_other, _opts), do: "CborTag(#{Messages.invalid_placeholder()})"
end

defmodule CodingAdventures.CanonicalCbor.Types.CborBool do
  @moduledoc "Major Type 7: Boolean simple value (true or false)."
  @enforce_keys [:value]
  defstruct [:value]
  @type t :: %__MODULE__{value: boolean()}
end

defimpl Inspect, for: CodingAdventures.CanonicalCbor.Types.CborBool do
  alias CodingAdventures.CanonicalCbor.Messages
  def inspect(%{value: true}, _opts), do: "CborBool(true)"
  def inspect(%{value: false}, _opts), do: "CborBool(false)"
  def inspect(_other, _opts), do: "CborBool(#{Messages.invalid_placeholder()})"
end

defmodule CodingAdventures.CanonicalCbor.Types.CborNull do
  @moduledoc "Major Type 7: Null simple value (0xF6)."
  defstruct []
  @type t :: %__MODULE__{}
end

defimpl Inspect, for: CodingAdventures.CanonicalCbor.Types.CborNull do
  def inspect(_struct, _opts), do: "CborNull"
end

defmodule CodingAdventures.CanonicalCbor.Types do
  @moduledoc """
  Distinct algebraic model, pattern predicates, and defensive constructors
  for deterministic canonical CBOR (RFC 8949 CBR01 profile).
  """

  alias CodingAdventures.CanonicalCbor.{CborError, Constants}

  alias CodingAdventures.CanonicalCbor.Types.{
    CborArray,
    CborBool,
    CborBytes,
    CborMap,
    CborMapEntry,
    CborNegative,
    CborNull,
    CborTag,
    CborText,
    CborUnsigned
  }

  @max_nesting_depth Constants.max_nesting_depth()

  @type cbor_value ::
          CborUnsigned.t()
          | CborNegative.t()
          | CborBytes.t()
          | CborText.t()
          | CborArray.t()
          | CborMap.t()
          | CborTag.t()
          | CborBool.t()
          | CborNull.t()

  @doc """
  Shallow node validation for constructors, keeping nested construction O(N) rather than O(N * depth).
  """
  @spec valid_cbor_node?(term()) :: boolean()
  def valid_cbor_node?(node) do
    case node do
      %CborUnsigned{value: v} ->
        is_integer(v) and v >= 0 and v <= Constants.max_uint64()

      %CborNegative{value: v} ->
        is_integer(v) and v >= 0 and v <= Constants.max_uint64()

      %CborBytes{bytes: b} ->
        is_binary(b)

      %CborText{text: t} ->
        is_binary(t) and String.valid?(t)

      %CborArray{items: it} ->
        is_list(it)

      %CborMap{entries: e} ->
        is_list(e)

      %CborTag{tag: t, value: inner} ->
        is_integer(t) and t >= 0 and t <= Constants.max_uint64() and
          match?(
            %{__struct__: struct}
            when struct in [
                   CborUnsigned,
                   CborNegative,
                   CborBytes,
                   CborText,
                   CborArray,
                   CborMap,
                   CborTag,
                   CborBool,
                   CborNull
                 ],
            inner
          )

      %CborBool{value: v} ->
        is_boolean(v)

      %CborNull{} ->
        true

      _ ->
        false
    end
  end

  @doc """
  Predicate verifying that a term is an authentic CBR01 CBOR value instance.
  Validates full recursive invariants bounded by `Constants.max_nesting_depth()`.
  """
  @spec is_cbor_value?(term()) :: boolean()
  def is_cbor_value?(v), do: validate_cbor_value(v, Constants.root_depth())

  defp validate_cbor_value(_v, depth) when depth > @max_nesting_depth, do: false

  defp validate_cbor_value(%CborUnsigned{value: v}, _depth) do
    is_integer(v) and v >= 0 and v <= Constants.max_uint64()
  end

  defp validate_cbor_value(%CborNegative{value: v}, _depth) do
    is_integer(v) and v >= 0 and v <= Constants.max_uint64()
  end

  defp validate_cbor_value(%CborBytes{bytes: b}, _depth) do
    is_binary(b)
  end

  defp validate_cbor_value(%CborText{text: t}, _depth) do
    is_binary(t) and String.valid?(t)
  end

  defp validate_cbor_value(%CborArray{items: it}, depth) when is_list(it) do
    Enum.all?(it, &validate_cbor_value(&1, depth + 1))
  end

  defp validate_cbor_value(%CborMap{entries: e}, depth) when is_list(e) do
    Enum.all?(e, fn
      %CborMapEntry{key: k, value: v} ->
        validate_cbor_value(k, depth + 1) and validate_cbor_value(v, depth + 1)

      _ ->
        false
    end)
  end

  defp validate_cbor_value(%CborTag{tag: t, value: inner}, depth) do
    is_integer(t) and t >= 0 and t <= Constants.max_uint64() and
      validate_cbor_value(inner, depth + 1)
  end

  defp validate_cbor_value(%CborBool{value: v}, _depth) when is_boolean(v), do: true
  defp validate_cbor_value(%CborNull{}, _depth), do: true
  defp validate_cbor_value(_other, _depth), do: false

  @doc """
  Predicate verifying that a term is an authentic CborMapEntry instance.
  """
  @spec is_cbor_map_entry?(term()) :: boolean()
  def is_cbor_map_entry?(v) do
    case v do
      %CborMapEntry{key: k, value: val} ->
        is_cbor_value?(k) and is_cbor_value?(val)

      _ ->
        false
    end
  end

  @doc """
  Predicate verifying that a term is an authentic Span instance.
  """
  @spec is_span?(term()) :: boolean()
  def is_span?(v) do
    case v do
      %CodingAdventures.CanonicalCbor.Span{start: s, finish: f} ->
        is_integer(s) and s >= 0 and is_integer(f) and f >= s and f <= Constants.max_uint64()

      _ ->
        false
    end
  end

  @doc """
  Predicate verifying that a term is an authentic SpannedMapEntry instance.
  """
  @spec is_spanned_map_entry?(term()) :: boolean()
  def is_spanned_map_entry?(v) do
    case v do
      %CodingAdventures.CanonicalCbor.SpannedMapEntry{key: k, value: val, value_span: span} ->
        is_cbor_value?(k) and is_cbor_value?(val) and is_span?(span)

      _ ->
        false
    end
  end

  @doc """
  Predicate verifying that a term is an authentic DecodeMapSpansResult instance.
  """
  @spec is_decode_map_spans_result?(term()) :: boolean()
  def is_decode_map_spans_result?(v) do
    case v do
      %CodingAdventures.CanonicalCbor.DecodeMapSpansResult{is_map: true, entries: entries}
      when is_list(entries) ->
        Enum.all?(entries, &is_spanned_map_entry?/1)

      %CodingAdventures.CanonicalCbor.DecodeMapSpansResult{is_map: false, entries: nil} ->
        true

      _ ->
        false
    end
  end

  @doc """
  Predicate verifying that a term is an authentic CborError instance.
  """
  @spec is_cbor_error?(term()) :: boolean()
  def is_cbor_error?(v), do: match?(%CborError{}, v)

  # ---------------------------------------------------------------------------
  # Defensive Constructors (Raise CborError on invalid input)
  # ---------------------------------------------------------------------------

  @spec cbor_unsigned(term()) :: CborUnsigned.t()
  def cbor_unsigned(val) do
    if is_integer(val) and val >= 0 and val <= Constants.max_uint64() do
      %CborUnsigned{value: val}
    else
      raise CborError, :unsupported_simple
    end
  end

  @spec cbor_negative(term()) :: CborNegative.t()
  def cbor_negative(val) do
    if is_integer(val) and val >= 0 and val <= Constants.max_uint64() do
      %CborNegative{value: val}
    else
      raise CborError, :unsupported_simple
    end
  end

  @spec cbor_bytes(term()) :: CborBytes.t()
  def cbor_bytes(bytes) do
    if is_binary(bytes) do
      %CborBytes{bytes: bytes}
    else
      raise CborError, :unsupported_simple
    end
  end

  @spec cbor_text(term()) :: CborText.t()
  def cbor_text(text) do
    cond do
      is_binary(text) and String.valid?(text) ->
        %CborText{text: text}

      is_binary(text) ->
        raise CborError, :invalid_utf8

      true ->
        raise CborError, :unsupported_simple
    end
  end

  @spec cbor_array(term()) :: CborArray.t()
  def cbor_array(items), do: CborArray.new(items)

  @spec cbor_map_entry(term(), term()) :: CborMapEntry.t()
  def cbor_map_entry(key, value), do: CborMapEntry.new(key, value)

  @spec cbor_map(term()) :: CborMap.t()
  def cbor_map(entries), do: CborMap.new(entries)

  @spec cbor_tag(term(), term()) :: CborTag.t()
  def cbor_tag(tag, value), do: CborTag.new(tag, value)

  @spec cbor_bool(term()) :: CborBool.t()
  def cbor_bool(val) do
    if is_boolean(val) do
      %CborBool{value: val}
    else
      raise CborError, :unsupported_simple
    end
  end

  @spec cbor_null() :: CborNull.t()
  def cbor_null, do: %CborNull{}
end
