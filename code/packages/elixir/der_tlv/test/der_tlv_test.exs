defmodule CodingAdventures.DerTlvTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.DerTlv
  alias CodingAdventures.DerTlv.Error

  @fixture Path.expand("../../../../specs/fixtures/der-tlv-v1/cases.json", __DIR__)
           |> File.read!()
           |> Jason.decode!()

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
    values = Map.merge(@fixture["defaults"], Map.get(test_case, "limits", %{}))

    max_value =
      if values["max_value_len"] == "host-max",
        do: 0x7FFF_FFFF_FFFF_FFFF,
        else: values["max_value_len"]

    %{
      max_input_len: values["max_input_len"],
      max_value_len: max_value,
      max_elements: values["max_elements"],
      max_tag_number: values["max_tag_number"]
    }
  end

  defp element_result(element, offset) do
    %{
      "outcome" => "element",
      "element_offset" => offset,
      "tag" => %{
        "class" => element.tag.class,
        "constructed" => element.tag.constructed,
        "number" => element.tag.number
      },
      "header_len" => byte_size(DerTlv.header(element)),
      "encoded_len" => byte_size(DerTlv.encoded(element)),
      "remainder_offset" => offset + byte_size(DerTlv.encoded(element))
    }
  end

  defp error_result(%Error{} = error) do
    %{"outcome" => "error", "error_id" => error.kind, "offset" => error.offset}
  end

  defp run_decode(test_case, input, configured) do
    case test_case["operation"] do
      "decode-one" ->
        case DerTlv.decode_one(input, configured) do
          {:ok, element, remainder} ->
            result = element_result(element, 0)
            assert result["remainder_offset"] == byte_size(input) - byte_size(remainder)
            result

          {:error, failure} ->
            error_result(failure)
        end

      "decode-exact" ->
        case DerTlv.decode_exact(input, configured) do
          {:ok, element} -> element_result(element, 0)
          {:error, failure} -> error_result(failure)
        end
    end
  end

  defp run_cursor(test_case, input, configured) do
    {:ok, cursor} = DerTlv.new_cursor(input, configured)

    {events, final_cursor} =
      Enum.map_reduce(test_case["actions"], cursor, fn
        "finish", current ->
          event =
            case DerTlv.finish(current) do
              :ok -> %{"outcome" => "finished"}
              {:error, failure} -> error_result(failure)
            end

          {event, current}

        "read", current ->
          offset = byte_size(input) - byte_size(DerTlv.remaining(current))

          case DerTlv.read(current) do
            {:ok, element, next} -> {element_result(element, offset), next}
            {:end, same} -> {%{"outcome" => "end"}, same}
            {:error, failure, same} -> {error_result(failure), same}
          end
      end)

    %{
      "events" => events,
      "elements_read" => final_cursor.elements_read,
      "remaining_offset" => byte_size(input) - byte_size(DerTlv.remaining(final_cursor))
    }
  end

  test "pins the closed profile" do
    assert length(@fixture["cases"]) == 54
    assert length(@fixture["error_ids"]) == 17
  end

  for test_case <- @fixture["cases"] do
    @test_case test_case
    test test_case["id"] do
      input = materialize(@test_case["input"])
      configured = limits(@test_case)

      actual =
        if @test_case["operation"] == "cursor" do
          run_cursor(@test_case, input, configured)
        else
          run_decode(@test_case, input, configured)
        end

      assert actual == @test_case["expected"]

      if hostile = @test_case["redacted_input_hex"] do
        refute Jason.encode!(actual) =~ hostile
      end
    end
  end

  test "sub-binaries preserve the caller bytes" do
    {:ok, element} = DerTlv.decode_exact(<<4, 1, 42>>)
    assert DerTlv.value(element) == <<42>>
  end

  test "default entry points use the documented limits" do
    assert DerTlv.default_limits().max_elements == 4_096
    assert {:ok, _element, <<>>} = DerTlv.decode_one(<<5, 0>>)
    assert {:ok, cursor} = DerTlv.new_cursor(<<5, 0>>)
    assert byte_size(DerTlv.remaining(cursor)) == 2
  end

  test "invalid limits fail before decoding" do
    assert {:error, %ArgumentError{}} = DerTlv.decode_exact(<<>>, %{max_elements: -1})

    assert {:error, %ArgumentError{}} =
             DerTlv.decode_exact(<<>>, %{max_tag_number: 0x1_0000_0000})
  end

  test "errors have a stable non-payload-bearing message" do
    error = Error.exception(kind: "truncated-value", offset: 2)
    assert Exception.message(error) == "DER framing error truncated-value at byte 2"
  end
end
