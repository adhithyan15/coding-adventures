defmodule CodingAdventures.X509ExtensionTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.DerAsn1
  alias CodingAdventures.X509Extension

  @root Path.expand("../../../../specs/fixtures", __DIR__)
  @fixture @root |> Path.join("x509-extension-v1/cases.json") |> File.read!() |> Jason.decode!()
  @upstream @root |> Path.join("der-asn1-v1/cases.json") |> File.read!() |> Jason.decode!()
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
    defaults = @upstream["defaults"]

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

  defp project_error(error, decoder) do
    result = %{
      "outcome" => "error",
      "error_id" => X509Extension.error_kind(error),
      "offset" => X509Extension.error_offset(error),
      "offset_scope" => "extension-element",
      "elements_read" => DerAsn1.elements_read(decoder)
    }

    result =
      if kind = X509Extension.error_asn1_kind(error),
        do: Map.put(result, "asn1_error_id", kind),
        else: result

    if kind = X509Extension.error_framing_kind(error),
      do: Map.put(result, "framing_error_id", kind),
      else: result
  end

  defp attempt(decoder, root) do
    case X509Extension.decode_extension(decoder, root) do
      {:ok, extension, decoder} ->
        {%{
           "outcome" => "value",
           "extension_id_arcs_decimal" =>
             extension
             |> X509Extension.extension_id()
             |> DerAsn1.oid_arcs()
             |> Enum.map(&Integer.to_string/1),
           "critical" => X509Extension.extension_critical?(extension),
           "extension_value_hex" =>
             extension |> X509Extension.extension_value() |> Base.encode16(case: :lower),
           "elements_read" => DerAsn1.elements_read(decoder)
         }, nil, decoder}

      {:error, error, decoder} ->
        {project_error(error, decoder), error, decoder}
    end
  end

  defp run_case(test_case) do
    {:ok, decoder} = DerAsn1.new_decoder(limits(test_case))
    {:ok, root, decoder} = DerAsn1.decode_exact(decoder, materialize(test_case["input"]))

    if test_case["operation"] == "extension-script" do
      {events, errors, _decoder} =
        Enum.reduce(test_case["actions"], {[], [], decoder}, fn _action,
                                                                {events, errors, decoder} ->
          {event, error, decoder} = attempt(decoder, root)
          {events ++ [event], errors ++ [error], decoder}
        end)

      {%{"outcome" => "script", "events" => events}, errors}
    else
      {actual, error, _decoder} = attempt(decoder, root)
      {actual, [error]}
    end
  end

  test "executes the complete language-neutral fixture" do
    assert length(@fixture["cases"]) == 48
    assert length(@fixture["error_ids"]) == 8

    Enum.each(@fixture["cases"], fn test_case ->
      {actual, errors} = run_case(test_case)
      assert actual == test_case["expected"], test_case["id"]

      if hostile = test_case["redacted_input_hex"] do
        refute Jason.encode!(actual) =~ hostile

        Enum.each(Enum.reject(errors, &is_nil/1), fn error ->
          refute String.contains?(String.downcase(X509Extension.error_message(error)), hostile)
        end)
      end
    end)
  end

  test "values are opaque, defensive, and payload-blind" do
    input = Base.decode16!("30090603551d1104023000", case: :lower)
    {:ok, decoder} = DerAsn1.new_decoder()
    {:ok, root, decoder} = DerAsn1.decode_exact(decoder, input)
    {:ok, extension, _decoder} = X509Extension.decode_extension(decoder, root)

    assert X509Extension.extension_critical?(extension) == false
    assert X509Extension.extension_value(extension) == <<0x30, 0x00>>
    assert DerAsn1.oid_arcs(X509Extension.extension_id(extension)) == [2, 5, 29, 17]
    refute inspect(extension) =~ "3000"
    assert {:extension, extension_process} = extension.(:__x509_extension_sealed__)
    assert is_pid(extension_process)
    send(extension_process, :ignored)
    assert X509Extension.extension_critical?(extension) == false

    assert_raise ArgumentError, fn ->
      X509Extension.extension_value(fn _ -> {:extension, self()} end)
    end

    assert_raise ArgumentError, fn -> X509Extension.error_kind(extension) end
    assert_raise ArgumentError, fn -> X509Extension.extension_value(:forged) end
    refute X509Extension.error?(fn _ -> {:error, self()} end)

    hostile = Base.decode16!("30080601800403deadbe", case: :lower)
    {:ok, hostile_decoder} = DerAsn1.new_decoder()
    {:ok, hostile_root, hostile_decoder} = DerAsn1.decode_exact(hostile_decoder, hostile)
    {:error, error, _decoder} = X509Extension.decode_extension(hostile_decoder, hostile_root)
    assert X509Extension.error?(error)
    assert X509Extension.error_kind(error) == "invalid-extension-id"
    assert X509Extension.error_asn1_kind(error) == "non-minimal-object-identifier"
    assert X509Extension.error_offset(error) == 4
    refute String.contains?(String.downcase(X509Extension.error_message(error)), "deadbe")
    refute inspect(error) =~ "deadbe"

    assert {:error, error_process} = error.(:__x509_extension_sealed__)
    monitor = Process.monitor(error_process)
    Process.exit(error_process, :kill)
    assert_receive {:DOWN, ^monitor, :process, ^error_process, :killed}

    assert_raise ArgumentError, "validated value is no longer available", fn ->
      X509Extension.error_kind(error)
    end
  end

  test "opaque malformed inner DER remains uninterpreted" do
    input = Base.decode16!("300a0603551d110403ff0001", case: :lower)
    {:ok, decoder} = DerAsn1.new_decoder()
    {:ok, root, decoder} = DerAsn1.decode_exact(decoder, input)
    {:ok, extension, _decoder} = X509Extension.decode_extension(decoder, root)
    assert X509Extension.extension_value(extension) == <<0xFF, 0x00, 0x01>>
  end
end
