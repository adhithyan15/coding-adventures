defmodule CodingAdventures.DerAsn1Test do
  use ExUnit.Case, async: true

  alias CodingAdventures.DerAsn1
  alias CodingAdventures.DerAsn1.Error
  alias CodingAdventures.DerTlv

  @root Path.expand("../../../../specs/fixtures", __DIR__)
  @fixture @root |> Path.join("der-asn1-v1/cases.json") |> File.read!() |> Jason.decode!()
  @upstream @root |> Path.join("der-tlv-v1/cases.json") |> File.read!() |> Jason.decode!()
  @host_max 0x7FFF_FFFF_FFFF_FFFF

  defp materialize(segments) do
    segments
    |> Enum.map(fn
      %{"hex" => value} ->
        Base.decode16!(value, case: :lower)

      %{"repeat_hex" => value, "count" => count} ->
        :binary.copy(Base.decode16!(value, case: :lower), count)
    end)
    |> IO.iodata_to_binary()
  end

  defp limits(test_case) do
    override = Map.get(test_case, "limits", %{})
    defaults = @fixture["defaults"]

    der =
      defaults["der"]
      |> Map.merge(Map.get(override, "der", %{}))
      |> Map.new(fn
        {"max_value_len", "host-max"} -> {:max_value_len, @host_max}
        {key, value} -> {String.to_existing_atom(key), value}
      end)

    %{
      der: der,
      max_depth: Map.get(override, "max_depth", defaults["max_depth"]),
      max_total_elements: Map.get(override, "max_total_elements", defaults["max_total_elements"]),
      max_oid_arcs: Map.get(override, "max_oid_arcs", defaults["max_oid_arcs"])
    }
  end

  defp tag(element) do
    value = DerAsn1.element_tag(element)
    %{"class" => value.class, "constructed" => value.constructed, "number" => value.number}
  end

  defp failure(%Error{} = error, scope \\ "operation-input") do
    result = %{
      "outcome" => "error",
      "error_id" => error.kind,
      "offset" => error.offset,
      "offset_scope" => scope
    }

    if error.framing_kind,
      do: Map.put(result, "framing_error_id", error.framing_kind),
      else: result
  end

  defp upstream_case(test_case) do
    upstream = Enum.find(@upstream["cases"], &(&1["id"] == test_case["der_tlv_case_id"]))
    input = materialize(upstream["input"])

    configured =
      @upstream["defaults"]
      |> Map.merge(Map.get(upstream, "limits", %{}))
      |> Map.new(fn
        {"max_value_len", "host-max"} -> {:max_value_len, @host_max}
        {key, value} -> {String.to_existing_atom(key), value}
      end)

    actual =
      case DerTlv.decode_exact(input, configured) do
        {:ok, element} ->
          %{
            "outcome" => "element",
            "element_offset" => 0,
            "tag" => %{
              "class" => element.tag.class,
              "constructed" => element.tag.constructed,
              "number" => element.tag.number
            },
            "header_len" => byte_size(DerTlv.header(element)),
            "encoded_len" => byte_size(DerTlv.encoded(element)),
            "remainder_offset" => byte_size(DerTlv.encoded(element))
          }

        {:error, error} ->
          %{"outcome" => "error", "error_id" => error.kind, "offset" => error.offset}
      end

    if actual == upstream["expected"], do: %{"outcome" => "upstream"}, else: actual
  end

  defp primitive_result(operation, element, configured, tag_number) do
    case operation do
      "decode-boolean" ->
        with {:ok, value} <- DerAsn1.decode_boolean(element),
             do: %{"outcome" => "value", "boolean" => value}

      operation when operation in ["decode-integer", "integer-to-u64"] ->
        with {:ok, integer} <- DerAsn1.decode_integer(element) do
          result = %{
            "outcome" => "value",
            "signed_hex" => Base.encode16(integer.signed_bytes, case: :lower),
            "negative" => integer.negative
          }

          if operation == "integer-to-u64" do
            case DerAsn1.integer_to_u64(integer) do
              {:ok, value} -> Map.put(result, "u64_decimal", Integer.to_string(value))
              {:error, error} -> {:error, error}
            end
          else
            result
          end
        end

      "decode-bit-string" ->
        with {:ok, bits} <- DerAsn1.decode_bit_string(element) do
          %{
            "outcome" => "value",
            "bytes_hex" => Base.encode16(bits.bytes, case: :lower),
            "unused_bits" => bits.unused_bits,
            "bit_length" => bits.bit_length
          }
        end

      "decode-octet-string" ->
        with {:ok, value} <- DerAsn1.decode_octet_string(element),
             do: %{"outcome" => "value", "bytes_hex" => Base.encode16(value, case: :lower)}

      "decode-implicit-octet-string" ->
        with {:ok, value} <- DerAsn1.decode_implicit_octet_string(element, tag_number),
             do: %{"outcome" => "value", "bytes_hex" => Base.encode16(value, case: :lower)}

      "decode-ia5-string" ->
        with {:ok, value} <- DerAsn1.decode_ia5_string(element),
             do: %{"outcome" => "value", "text" => value}

      "decode-implicit-ia5-string" ->
        with {:ok, value} <- DerAsn1.decode_implicit_ia5_string(element, tag_number),
             do: %{"outcome" => "value", "text" => value}

      "decode-null" ->
        with {:ok, nil} <- DerAsn1.decode_null(element), do: %{"outcome" => "value"}

      operation
      when operation in ["decode-object-identifier", "decode-implicit-object-identifier"] ->
        decoded =
          if operation == "decode-object-identifier",
            do: DerAsn1.decode_object_identifier(element, configured),
            else: DerAsn1.decode_implicit_object_identifier(element, tag_number, configured)

        with {:ok, oid} <- decoded do
          %{
            "outcome" => "value",
            "bytes_hex" => Base.encode16(oid.encoded, case: :lower),
            "arcs_decimal" => Enum.map(oid.arcs, &Integer.to_string/1),
            "arc_count" => length(oid.arcs)
          }
        end
    end
  end

  defp cursor_result(test_case, decoder, root) do
    {:ok, cursor} = DerAsn1.sequence(decoder, root)
    total = byte_size(DerAsn1.cursor_remaining(cursor))

    {events, cursor, decoder} =
      Enum.reduce(test_case["actions"], {[], cursor, decoder}, fn action,
                                                                  {events, cursor, decoder} ->
        case action do
          "finish" ->
            event =
              case DerAsn1.cursor_finish(cursor) do
                :ok -> %{"outcome" => "finished"}
                {:error, error} -> failure(error, "container-value")
              end

            {events ++ [event], cursor, decoder}

          "read-with-different-limits" ->
            current = DerAsn1.decoder_limits(decoder)

            {:ok, other} =
              DerAsn1.new_decoder(%{current | max_total_elements: current.max_total_elements + 1})

            {:error, error, _same_cursor, _other} = DerAsn1.cursor_read(cursor, other)
            {events ++ [failure(error, "container-value")], cursor, decoder}

          "read-nested-sequence" ->
            case DerAsn1.cursor_read(cursor, decoder) do
              {:ok, child, next_cursor, next_decoder} ->
                case DerAsn1.sequence(next_decoder, child) do
                  {:ok, nested} ->
                    case DerAsn1.cursor_read(nested, next_decoder) do
                      {:ok, grandchild, nested_after, final_decoder} ->
                        :ok = DerAsn1.cursor_finish(nested_after)

                        event = %{
                          "outcome" => "value",
                          "tag" => tag(grandchild),
                          "depth" => DerAsn1.element_depth(grandchild)
                        }

                        {events ++ [event], next_cursor, final_decoder}

                      {:error, error, _same_nested, same_decoder} ->
                        {events ++ [failure(error, "container-value")], next_cursor, same_decoder}
                    end

                  {:error, error} ->
                    {events ++ [failure(error, "container-value")], next_cursor, next_decoder}
                end

              {:error, error, same_cursor, same_decoder} ->
                {events ++ [failure(error, "container-value")], same_cursor, same_decoder}
            end

          "read" ->
            case DerAsn1.cursor_read(cursor, decoder) do
              {:ok, child, next_cursor, next_decoder} ->
                event = %{
                  "outcome" => "value",
                  "tag" => tag(child),
                  "depth" => DerAsn1.element_depth(child)
                }

                {events ++ [event], next_cursor, next_decoder}

              {:end, same_cursor} ->
                {events ++ [%{"outcome" => "end"}], same_cursor, decoder}

              {:error, error, same_cursor, same_decoder} ->
                {events ++ [failure(error, "container-value")], same_cursor, same_decoder}
            end
        end
      end)

    %{
      "outcome" => "value",
      "elements_read" => DerAsn1.elements_read(decoder),
      "remaining_offset" => total - byte_size(DerAsn1.cursor_remaining(cursor)),
      "events" => events
    }
  end

  defp run_case(%{"der_tlv_case_id" => _} = test_case), do: upstream_case(test_case)

  defp run_case(test_case) do
    configured = limits(test_case)
    {:ok, decoder} = DerAsn1.new_decoder(configured)
    operation = test_case["operation"]

    case DerAsn1.decode_exact(decoder, materialize(test_case["input"])) do
      {:error, error} ->
        failure(error)

      {:ok, root, decoder} ->
        result =
          case operation do
            "decode-exact" ->
              %{
                "outcome" => "value",
                "tag" => tag(root),
                "header_hex" => Base.encode16(DerAsn1.element_header(root), case: :lower),
                "value_hex" => Base.encode16(DerAsn1.element_value(root), case: :lower),
                "encoded_hex" => Base.encode16(DerAsn1.element_encoded(root), case: :lower),
                "depth" => DerAsn1.element_depth(root),
                "elements_read" => DerAsn1.elements_read(decoder)
              }

            "cursor-script" ->
              cursor_result(test_case, decoder, root)

            operation when operation in ["sequence", "set"] ->
              opened =
                if operation == "sequence",
                  do: DerAsn1.sequence(decoder, root),
                  else: DerAsn1.set(decoder, root)

              case opened do
                {:ok, cursor} ->
                  %{
                    "outcome" => "value",
                    "elements_read" => DerAsn1.elements_read(decoder),
                    "remaining_offset" =>
                      byte_size(DerAsn1.element_value(root)) -
                        byte_size(DerAsn1.cursor_remaining(cursor))
                  }

                {:error, error} ->
                  {:error, error}
              end

            "explicit" ->
              case DerAsn1.explicit(decoder, root, test_case["tag_number"]) do
                {:ok, child, next_decoder} ->
                  %{
                    "outcome" => "value",
                    "tag" => tag(child),
                    "value_hex" => Base.encode16(DerAsn1.element_value(child), case: :lower),
                    "depth" => DerAsn1.element_depth(child),
                    "elements_read" => DerAsn1.elements_read(next_decoder)
                  }

                {:error, error} ->
                  {:error, error}
              end

            _ ->
              primitive_result(operation, root, configured, test_case["tag_number"])
          end

        case result do
          {:error, error} ->
            scope =
              if operation == "explicit" and error.kind == "framing",
                do: "container-value",
                else: "operation-input"

            failure(error, scope)

          value when is_map(value) ->
            if Map.has_key?(test_case["expected"], "elements_read") and
                 not Map.has_key?(value, "elements_read"),
               do: Map.put(value, "elements_read", DerAsn1.elements_read(decoder)),
               else: value
        end
    end
  end

  test "executes the closed portable fixture" do
    assert length(@fixture["cases"]) == 122
    assert length(@fixture["error_ids"]) == 22

    references =
      @fixture["cases"]
      |> Enum.map(& &1["der_tlv_case_id"])
      |> Enum.reject(&is_nil/1)
      |> Enum.uniq()

    assert length(references) == 46

    Enum.each(@fixture["cases"], fn test_case ->
      actual = run_case(test_case)

      assert actual == test_case["expected"],
             "#{test_case["id"]}\nactual: #{inspect(actual)}\nexpected: #{inspect(test_case["expected"])}"

      if hostile = test_case["redacted_input_hex"],
        do: refute(Jason.encode!(actual) =~ hostile)
    end)
  end

  test "seals wrappers and binds descendants to their decoder" do
    {:ok, decoder} = DerAsn1.new_decoder()
    {:ok, element, decoder} = DerAsn1.decode_exact(decoder, <<4, 1, 42>>)
    assert {:ok, <<42>>} = DerAsn1.decode_octet_string(element)

    assert_raise ArgumentError, fn ->
      DerAsn1.decode_octet_string(fn _ -> %{tag: nil, value: "secret"} end)
    end

    {:ok, other} = DerAsn1.new_decoder()
    assert {:error, %ArgumentError{}} = DerAsn1.sequence(other, element)
    assert DerAsn1.elements_read(decoder) == 1
  end

  test "validates limits, tags, exact OIDs, and redacted errors" do
    assert DerAsn1.default_limits().max_depth == 32
    assert {:error, %ArgumentError{}} = DerAsn1.new_decoder(%{unknown: 1})
    assert {:error, %ArgumentError{}} = DerAsn1.new_decoder(%{max_depth: -1})
    assert {:error, %ArgumentError{}} = DerAsn1.new_decoder(%{max_depth: @host_max + 1})
    assert {:error, %ArgumentError{}} = DerAsn1.new_decoder([])
    assert {:error, %ArgumentError{}} = DerAsn1.new_decoder(%{der: []})

    {:ok, decoder} = DerAsn1.new_decoder()

    assert {:error, %Error{kind: "framing", framing_kind: "empty-input"}} =
             DerAsn1.decode_exact(decoder, <<>>)

    assert {:error, %ArgumentError{}} = DerAsn1.decode_exact(decoder, [])
    assert {:error, %ArgumentError{}} = DerAsn1.integer_to_u64(:forged)
    assert_raise ArgumentError, fn -> DerAsn1.element_value(:forged) end

    {:ok, element, _decoder} = DerAsn1.decode_exact(decoder, <<6, 3, 42, 3, 4>>)
    assert {:ok, oid} = DerAsn1.decode_object_identifier(element)
    assert DerAsn1.oid_equals?(oid, [1, 2, 3, 4])
    refute DerAsn1.oid_equals?(oid, [1, 2, 3, 5])
    assert {:error, %ArgumentError{}} = DerAsn1.decode_implicit_octet_string(element, -1)

    assert {:error, %ArgumentError{}} =
             DerAsn1.decode_implicit_octet_string(element, 4_294_967_296)

    assert {:ok, zero_first, _decoder} = DerAsn1.decode_exact(decoder, <<6, 1, 10>>)
    assert {:ok, %{arcs: [0, 10]}} = DerAsn1.decode_object_identifier(zero_first)

    assert {:ok, implicit, _decoder} = DerAsn1.decode_exact(decoder, <<0x88, 3, 42, 3, 4>>)

    assert {:ok, %{arcs: [1, 2, 3, 4]}} =
             DerAsn1.decode_implicit_object_identifier(implicit, 8)

    {:ok, sequence, decoder} = DerAsn1.decode_exact(decoder, <<0x30, 2, 5, 0>>)
    {:ok, cursor} = DerAsn1.sequence(decoder, sequence)

    assert {:error, %Error{kind: "framing", framing_kind: "trailing-data"}} =
             DerAsn1.cursor_finish(cursor)

    message =
      Exception.message(%Error{kind: "framing", offset: 2, framing_kind: "truncated-value"})

    assert message == "DER ASN.1 error framing (truncated-value) at byte 2"
    refute message =~ "secret"

    plain = Error.exception(kind: "unexpected-tag", offset: 0)
    assert Exception.message(plain) == "DER ASN.1 error unexpected-tag at byte 0"
  end
end
