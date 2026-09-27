defmodule CodingAdventures.X509Extension do
  @moduledoc """
  Bounded, payload-blind decoding of the generic RFC 5280 Extension shape.

  Extension and error values are authenticated opaque handles. Their private
  state processes keep payloads outside closure environments and reject forged
  closures before access.
  """

  alias CodingAdventures.DerAsn1
  alias CodingAdventures.DerAsn1.Decoder

  defmodule Extension do
    @moduledoc "Opaque immutable extension value returned by `decode_extension/2`."
    @opaque t :: (term() -> term())
  end

  defmodule Error do
    @moduledoc "Opaque stable payload-blind extension error."
    @opaque t :: (term() -> term())
  end

  @spec decode_extension(Decoder.t(), term()) ::
          {:ok, Extension.t(), Decoder.t()} | {:error, Error.t(), Decoder.t()}
  def decode_extension(decoder, root) do
    value_offset = byte_size(DerAsn1.element_header(root))
    value_length = byte_size(DerAsn1.element_value(root))

    case DerAsn1.sequence(decoder, root) do
      {:ok, cursor} ->
        decode_id(decoder, root, cursor, value_offset, value_length)

      {:error, error} ->
        {:error, make_error("structure", error.offset, error), decoder}
    end
  end

  def extension_id(extension), do: sealed_data!(extension, :extension, "Extension").extension_id

  def extension_critical?(extension),
    do: sealed_data!(extension, :extension, "Extension").critical

  def extension_value(extension) do
    extension
    |> sealed_data!(:extension, "Extension")
    |> Map.fetch!(:extension_value)
    |> :binary.copy()
  end

  def error_kind(error), do: sealed_data!(error, :error, "Error").kind
  def error_offset(error), do: sealed_data!(error, :error, "Error").offset
  def error_asn1_kind(error), do: sealed_data!(error, :error, "Error").asn1_kind
  def error_framing_kind(error), do: sealed_data!(error, :error, "Error").framing_kind

  def error_message(error) do
    data = sealed_data!(error, :error, "Error")
    "X.509 extension error #{data.kind} at byte #{data.offset}"
  end

  def error?(value) do
    _data = sealed_data!(value, :error, "Error")
    true
  rescue
    ArgumentError -> false
  end

  defp decode_id(decoder, root, cursor, value_offset, value_length) do
    offset = child_offset(cursor, value_offset, value_length)

    case read_child(decoder, cursor, value_offset, offset) do
      {:ok, :end, _cursor, decoder} ->
        {:error, make_error("missing-extension-id", offset), decoder}

      {:ok, element, cursor, decoder} ->
        case DerAsn1.decode_object_identifier(element, DerAsn1.decoder_limits(decoder)) do
          {:ok, extension_id} ->
            decode_second(decoder, root, cursor, value_offset, value_length, extension_id)

          {:error, error} ->
            {:error, make_error("invalid-extension-id", offset + error.offset, error), decoder}
        end

      {:error, error, _cursor, decoder} ->
        {:error, error, decoder}
    end
  end

  defp decode_second(decoder, root, cursor, value_offset, value_length, extension_id) do
    offset = child_offset(cursor, value_offset, value_length)

    case read_child(decoder, cursor, value_offset, offset) do
      {:ok, :end, _cursor, decoder} ->
        {:error, make_error("missing-extension-value", offset), decoder}

      {:ok, second, cursor, decoder} ->
        if DerAsn1.element_tag(second).number == 1 do
          decode_critical(
            decoder,
            root,
            cursor,
            value_offset,
            value_length,
            extension_id,
            second,
            offset
          )
        else
          decode_value(
            decoder,
            root,
            cursor,
            value_offset,
            value_length,
            extension_id,
            false,
            second,
            offset
          )
        end

      {:error, error, _cursor, decoder} ->
        {:error, error, decoder}
    end
  end

  defp decode_critical(
         decoder,
         root,
         cursor,
         value_offset,
         value_length,
         extension_id,
         element,
         offset
       ) do
    case DerAsn1.decode_boolean(element) do
      {:ok, false} ->
        {:error, make_error("encoded-default-critical", offset), decoder}

      {:ok, true} ->
        value_start = child_offset(cursor, value_offset, value_length)

        case read_child(decoder, cursor, value_offset, value_start) do
          {:ok, :end, _cursor, decoder} ->
            {:error, make_error("missing-extension-value", value_start), decoder}

          {:ok, value, cursor, decoder} ->
            decode_value(
              decoder,
              root,
              cursor,
              value_offset,
              value_length,
              extension_id,
              true,
              value,
              value_start
            )

          {:error, error, _cursor, decoder} ->
            {:error, error, decoder}
        end

      {:error, error} ->
        {:error, make_error("invalid-critical", offset + error.offset, error), decoder}
    end
  end

  defp decode_value(
         decoder,
         _root,
         cursor,
         value_offset,
         value_length,
         extension_id,
         critical,
         element,
         offset
       ) do
    case DerAsn1.decode_octet_string(element) do
      {:ok, extension_value} ->
        trailing_offset = child_offset(cursor, value_offset, value_length)

        case read_child(decoder, cursor, value_offset, trailing_offset) do
          {:ok, :end, _cursor, decoder} ->
            extension =
              sealed_fun(:extension, %{
                extension_id: extension_id,
                critical: critical,
                extension_value: :binary.copy(extension_value)
              })

            {:ok, extension, decoder}

          {:ok, _trailing, _cursor, decoder} ->
            {:error, make_error("trailing-element", trailing_offset), decoder}

          {:error, error, _cursor, decoder} ->
            {:error, error, decoder}
        end

      {:error, error} ->
        {:error, make_error("invalid-extension-value", offset + error.offset, error), decoder}
    end
  end

  defp child_offset(cursor, value_offset, value_length) do
    value_offset + value_length - byte_size(DerAsn1.cursor_remaining(cursor))
  end

  defp read_child(decoder, cursor, value_offset, current_offset) do
    case DerAsn1.cursor_read(cursor, decoder) do
      {:ok, child, cursor, decoder} ->
        {:ok, child, cursor, decoder}

      {:end, cursor} ->
        {:ok, :end, cursor, decoder}

      {:error, error, cursor, decoder} ->
        offset =
          if error.kind == "framing",
            do: value_offset + error.offset,
            else: current_offset + error.offset

        {:error, make_error("structure", offset, error), cursor, decoder}
    end
  end

  defp make_error(kind, offset, asn1_error \\ nil) do
    sealed_fun(:error, %{
      kind: kind,
      offset: offset,
      asn1_kind: asn1_error && asn1_error.kind,
      framing_kind: asn1_error && asn1_error.framing_kind
    })
  end

  defp sealed_fun(kind, data) do
    owner = self()
    process = spawn(fn -> value_loop(owner, kind, data) end)
    sealed_closure(kind, process)
  end

  defp sealed_closure(kind, process) do
    fn :__x509_extension_sealed__ -> {kind, process} end
  end

  defp sealed_data!(value, kind, name) when is_function(value, 1) do
    sample = sealed_closure(:sample, self())
    fields = [:module, :new_uniq, :new_index]

    if Enum.all?(fields, &(:erlang.fun_info(value, &1) == :erlang.fun_info(sample, &1))) do
      case value.(:__x509_extension_sealed__) do
        {^kind, process} when is_pid(process) ->
          reply = value_call(process)

          case {trusted_value_process?(process, 10), reply} do
            {true, {^kind, data}} -> data
            _ -> raise ArgumentError, "expected validated #{name}"
          end

        _ ->
          raise ArgumentError, "expected validated #{name}"
      end
    else
      raise ArgumentError, "expected validated #{name}"
    end
  end

  defp sealed_data!(_value, _kind, name), do: raise(ArgumentError, "expected validated #{name}")

  defp value_loop(owner, kind, data) do
    monitor = Process.monitor(owner)
    value_loop_messages(monitor, kind, data)
  end

  defp value_loop_messages(monitor, kind, data) do
    receive do
      {:value_call, from, reference} ->
        send(from, {:value_reply, reference, {kind, data}})
        value_loop_messages(monitor, kind, data)

      {:DOWN, ^monitor, :process, _pid, _reason} ->
        :ok

      _other ->
        value_loop_messages(monitor, kind, data)
    end
  end

  defp value_call(process) do
    reference = make_ref()
    send(process, {:value_call, self(), reference})

    receive do
      {:value_reply, ^reference, reply} -> reply
    after
      1_000 -> raise ArgumentError, "validated value is no longer available"
    end
  end

  defp trusted_value_process?(_process, 0), do: false

  defp trusted_value_process?(process, attempts) do
    case Process.info(process, :current_function) do
      {:current_function, {__MODULE__, function, 3}}
      when function in [:value_loop, :value_loop_messages] ->
        true

      _ ->
        :erlang.yield()
        trusted_value_process?(process, attempts - 1)
    end
  end
end
