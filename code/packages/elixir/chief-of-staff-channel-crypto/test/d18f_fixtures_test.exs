defmodule CodingAdventures.ChiefOfStaffChannelCrypto.D18FFixturesTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.ChiefOfStaffChannelCrypto, as: Crypto

  alias Crypto.{
    D18Message,
    MessageFields,
    MessageProfileError,
    MonotonicUuidV7Generator,
    SourcedMessageFields
  }

  @fixture_path Path.expand(
                  "../../../../fixtures/chief-of-staff-message/v1/manifest.json",
                  __DIR__
                )
  @fixture @fixture_path |> File.read!() |> Jason.decode!()
  @signing_seed Base.decode16!(@fixture["keys"]["originator_signing_seed_hex"], case: :lower)
  @signing_keypair CodingAdventures.Ed25519.generate_keypair(@signing_seed)
  @public_key elem(@signing_keypair, 0)
  @signing_secret_key elem(@signing_keypair, 1)
  @expected_public_key Base.decode16!(@fixture["keys"]["originator_public_key_hex"], case: :lower)
  @epoch_keys Map.new(@fixture["keys"]["channel_master_keys"], fn item ->
                {String.to_integer(item["key_epoch"]),
                 Base.decode16!(item["key_hex"], case: :lower)}
              end)

  defp decode(value), do: Base.decode64!(value)

  defp assert_profile_error(code, operation) do
    error = assert_raise MessageProfileError, operation
    assert error.code == code
  end

  test "fixture provenance and public material are locked" do
    assert @fixture["fixture_format"] == "D18F-message-fixtures-v1"
    assert String.length(@fixture["generator_blob_sha1"]) == 40
    assert @fixture["warning"] =~ "test-only"
    assert length(@fixture["positive_cases"]) == 8
    assert length(@fixture["binary_negative_cases"]) == 20
    assert length(@fixture["json_negative_cases"]) == 11
    assert @public_key == @expected_public_key
  end

  test "positive fixtures are reproduced byte identically" do
    for test_case <- @fixture["positive_cases"] do
      binary = decode(test_case["d18m_b64"])
      plaintext = decode(test_case["plaintext_b64"])
      message = Crypto.message_deserialize(binary)
      key = Map.fetch!(@epoch_keys, message.key_epoch)

      assert Crypto.message_serialize(message) == binary, test_case["name"]

      assert Crypto.message_authenticated_header(message) ==
               decode(test_case["authenticated_header_b64"]),
             test_case["name"]

      assert Crypto.message_verify_with_key_resolver(
               message,
               @public_key,
               &Map.get(@epoch_keys, &1)
             ) == plaintext,
             test_case["name"]

      assert Crypto.message_verify(message, @public_key, key) == plaintext, test_case["name"]

      canonical = decode(test_case["canonical_json_b64"])
      assert Crypto.message_to_json(message) == canonical, test_case["name"]

      assert message
             |> Crypto.message_to_json()
             |> Crypto.message_from_json()
             |> Crypto.message_serialize() == binary

      recreated =
        Crypto.message_create(message_fields(message), plaintext, @signing_secret_key, key)

      assert Crypto.message_serialize(recreated) == binary, test_case["name"]
    end
  end

  test "binary mutations map to stable errors" do
    for test_case <- @fixture["binary_negative_cases"] do
      assert_profile_error(test_case["expected_error"], fn ->
        message = test_case["d18m_b64"] |> decode() |> Crypto.message_deserialize()

        if test_case["phase"] == "verify" do
          Crypto.message_verify_with_key_resolver(message, @public_key, &Map.get(@epoch_keys, &1))
        end
      end)
    end
  end

  test "JSON mutations map to stable errors" do
    for test_case <- @fixture["json_negative_cases"] do
      assert_profile_error(test_case["expected_error"], fn ->
        test_case["json_b64"] |> decode() |> Crypto.message_from_json()
      end)
    end
  end

  test "JSON field order is irrelevant and output is canonical" do
    canonical = decode(Enum.at(@fixture["positive_cases"], 2)["canonical_json_b64"])

    reversed =
      canonical
      |> Jason.decode!(objects: :ordered_objects)
      |> then(&%Jason.OrderedObject{values: Enum.reverse(&1.values)})
      |> Jason.encode!()

    assert reversed |> Crypto.message_from_json() |> Crypto.message_to_json() == canonical
  end

  test "JSON rejects unpaired surrogates" do
    canonical = decode(hd(@fixture["positive_cases"])["canonical_json_b64"])

    malformed =
      String.replace(
        canonical,
        ~s("content_type":"application/octet-stream"),
        ~s("content_type":"\\ud800")
      )

    assert_profile_error("invalid_field", fn -> Crypto.message_from_json(malformed) end)
  end

  test "JSON field types fail before magic semantics" do
    canonical = decode(hd(@fixture["positive_cases"])["canonical_json_b64"])
    malformed = String.replace(canonical, ~s("record_type":"D18M"), ~s("record_type":18))
    assert_profile_error("invalid_json", fn -> Crypto.message_from_json(malformed) end)
  end

  test "canonical JSON uses literal UTF-8 instead of optional escapes" do
    canonical = decode(hd(@fixture["positive_cases"])["canonical_json_b64"])

    escaped =
      String.replace(
        canonical,
        ~s("content_type":"application/octet-stream"),
        ~s("content_type":"application/\\u2028")
      )

    encoded = escaped |> Crypto.message_from_json() |> Crypto.message_to_json()
    assert encoded =~ "application/\u2028"
    refute encoded =~ "\\u2028"
  end

  test "compact oversize recipes are enforced" do
    baseline = decode(hd(@fixture["positive_cases"])["d18m_b64"])

    for recipe <- @fixture["oversize_recipes"] do
      if recipe["field"] == "json-input" do
        assert_profile_error(recipe["expected_error"], fn ->
          Crypto.message_from_json(:binary.copy(<<0>>, Crypto.max_message_json_bytes() + 1))
        end)
      else
        length = String.to_integer(recipe["declared_length"])

        changed =
          case recipe["field"] do
            "originator-id" -> replace_bytes(baseline, 29, 4, <<length::unsigned-big-32>>)
            "content-type" -> replace_bytes(baseline, 83, 4, <<length::unsigned-big-32>>)
            "ciphertext" -> replace_bytes(baseline, 143, 8, <<length::unsigned-big-64>>)
          end

        assert_profile_error(recipe["expected_error"], fn ->
          Crypto.message_deserialize(changed)
        end)
      end
    end
  end

  test "constructors validate boundaries and updates cannot mutate a message" do
    source =
      @fixture["positive_cases"]
      |> Enum.at(1)
      |> Map.fetch!("d18m_b64")
      |> decode()
      |> Crypto.message_deserialize()

    original = Crypto.message_serialize(source)
    changed = %{source | sequence: 999, ciphertext: <<0>>}
    assert Crypto.message_serialize(source) == original
    refute changed == source

    assert_profile_error("invalid_field", fn ->
      MessageFields.new!(
        <<0::120>>,
        source.timestamp_ns,
        source.originator_id,
        source.channel_id,
        source.sequence,
        source.key_epoch,
        source.content_type
      )
    end)

    assert_profile_error("invalid_field", fn ->
      D18Message.new!(source |> Map.from_struct() |> Map.put(:authentication_tag, <<0::120>>))
    end)
  end

  test "creation uses injected UUID and monotonic clock sources" do
    source =
      @fixture["positive_cases"]
      |> hd()
      |> Map.fetch!("d18m_b64")
      |> decode()
      |> Crypto.message_deserialize()

    key = Map.fetch!(@epoch_keys, source.key_epoch)

    fields =
      SourcedMessageFields.new!(
        source.originator_id,
        source.channel_id,
        123,
        source.key_epoch,
        source.content_type
      )

    message =
      Crypto.message_create_with_sources(
        fields,
        <<1, 2, 3>>,
        @signing_secret_key,
        key,
        fn -> source.message_id end,
        fn -> 456 end
      )

    assert message.message_id == source.message_id
    assert message.timestamp_ns == 456
    assert Crypto.message_verify(message, @public_key, key) == <<1, 2, 3>>
  end

  test "UUID-v7 generator orders 1000 values in one millisecond" do
    {_generator, _previous} =
      Enum.reduce(1..1000, {MonotonicUuidV7Generator.new(), nil}, fn _, {generator, previous} ->
        {generator, current} =
          MonotonicUuidV7Generator.next(generator, 1_725_000_000_000, :binary.copy(<<0x55>>, 10))

        <<_::binary-size(6), version::4, _::12, variant::2, _::6, _::binary>> = current
        assert version == 7
        assert variant == 2
        if previous != nil, do: assert(previous < current)
        {generator, current}
      end)
  end

  defp message_fields(message) do
    MessageFields.new!(
      message.message_id,
      message.timestamp_ns,
      message.originator_id,
      message.channel_id,
      message.sequence,
      message.key_epoch,
      message.content_type
    )
  end

  defp replace_bytes(value, offset, length, replacement) do
    <<prefix::binary-size(offset), _::binary-size(length), suffix::binary>> = value
    prefix <> replacement <> suffix
  end
end
