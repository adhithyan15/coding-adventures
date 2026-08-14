defmodule CodingAdventures.ChiefOfStaffChannelCrypto do
  @moduledoc """
  Portable, immutable D18F encrypted messages for Chief of Staff channels.
  """

  import Bitwise

  alias CodingAdventures.ChaCha20Poly1305
  alias CodingAdventures.Ed25519
  alias CodingAdventures.Sha256
  alias __MODULE__.{D18Message, MessageFields, MessageProfileError, SourcedMessageFields}

  @message_context "chief-channel-message-v1"
  @message_magic "D18M"
  @wire_version 1
  @max_identity_bytes 4 * 1024
  @max_content_type_bytes 1024
  @max_ciphertext_bytes 64 * 1024 * 1024
  @max_u64 (1 <<< 64) - 1
  @json_fields ~w(record_type wire_version message_id timestamp_ns originator_id_b64
    channel_id sequence key_epoch content_type plaintext_hash_hex ciphertext_b64
    authentication_tag_b64 originator_signature_b64)

  @max_message_json_bytes 90 * 1024 * 1024
  def max_message_json_bytes, do: @max_message_json_bytes

  defmodule MessageProfileError do
    @moduledoc "One fail-closed D18F error with a stable portable code."
    defexception [:code]

    @impl true
    def message(%{code: code}), do: code
  end

  defmodule MessageFields do
    @moduledoc "Immutable fields supplied before hashing, signing, and encryption."
    @enforce_keys [
      :message_id,
      :timestamp_ns,
      :originator_id,
      :channel_id,
      :sequence,
      :key_epoch,
      :content_type
    ]
    defstruct @enforce_keys

    @type t :: %__MODULE__{
            message_id: binary(),
            timestamp_ns: non_neg_integer(),
            originator_id: binary(),
            channel_id: binary(),
            sequence: non_neg_integer(),
            key_epoch: non_neg_integer(),
            content_type: String.t()
          }

    def new!(
          message_id,
          timestamp_ns,
          originator_id,
          channel_id,
          sequence,
          key_epoch,
          content_type
        ) do
      CodingAdventures.ChiefOfStaffChannelCrypto.__new_fields!(
        message_id,
        timestamp_ns,
        originator_id,
        channel_id,
        sequence,
        key_epoch,
        content_type
      )
    end
  end

  defmodule SourcedMessageFields do
    @moduledoc "Creation fields whose UUID-v7 identifier and clock are injected."
    @enforce_keys [:originator_id, :channel_id, :sequence, :key_epoch, :content_type]
    defstruct @enforce_keys

    def new!(originator_id, channel_id, sequence, key_epoch, content_type) do
      fields =
        MessageFields.new!(
          <<0::128>>,
          0,
          originator_id,
          channel_id,
          sequence,
          key_epoch,
          content_type
        )

      %__MODULE__{
        originator_id: fields.originator_id,
        channel_id: fields.channel_id,
        sequence: fields.sequence,
        key_epoch: fields.key_epoch,
        content_type: fields.content_type
      }
    end
  end

  defmodule D18Message do
    @moduledoc "Complete immutable D18F encrypted-message value."
    @enforce_keys [
      :message_id,
      :timestamp_ns,
      :originator_id,
      :channel_id,
      :sequence,
      :key_epoch,
      :content_type,
      :plaintext_hash,
      :ciphertext,
      :authentication_tag,
      :originator_signature
    ]
    defstruct @enforce_keys

    def new!(attributes) when is_map(attributes) do
      CodingAdventures.ChiefOfStaffChannelCrypto.__new_message!(attributes)
    end
  end

  defmodule MonotonicUuidV7Generator do
    @moduledoc "Pure RFC 9562 UUID-v7 state with same-millisecond ordering."
    import Bitwise

    @random_mask (1 <<< 74) - 1
    @max_uuid_timestamp (1 <<< 48) - 1
    defstruct last_timestamp_ms: nil, last_random: 0

    def new, do: %__MODULE__{}

    def next(%__MODULE__{} = generator, timestamp_ms, entropy)
        when is_integer(timestamp_ms) and is_binary(entropy) do
      if timestamp_ms < 0 or timestamp_ms > @max_uuid_timestamp or byte_size(entropy) != 10 do
        raise MessageProfileError, code: "invalid_field"
      end

      supplied_random = :binary.decode_unsigned(entropy) &&& @random_mask

      {effective_timestamp, random} =
        if generator.last_timestamp_ms != nil and timestamp_ms <= generator.last_timestamp_ms do
          cond do
            generator.last_random < @random_mask ->
              {generator.last_timestamp_ms, generator.last_random + 1}

            generator.last_timestamp_ms < @max_uuid_timestamp ->
              {generator.last_timestamp_ms + 1, 0}

            true ->
              raise MessageProfileError, code: "invalid_field"
          end
        else
          {timestamp_ms, supplied_random}
        end

      random_a = random >>> 62 &&& 0xFFF
      random_b = random &&& (1 <<< 62) - 1

      uuid =
        <<effective_timestamp::unsigned-big-48, 7::4, random_a::12, 2::2,
          random_b::unsigned-big-62>>

      {%__MODULE__{last_timestamp_ms: effective_timestamp, last_random: random}, uuid}
    end

    def next(_, _, _), do: raise(MessageProfileError, code: "invalid_field")
  end

  def __new_fields!(
        message_id,
        timestamp_ns,
        originator_id,
        channel_id,
        sequence,
        key_epoch,
        content_type
      ) do
    require_binary!(message_id)
    require_binary!(originator_id)
    require_binary!(channel_id)
    require_binary!(content_type)
    require_length!(message_id, 16)
    require_u64!(timestamp_ns)
    require_length!(channel_id, 16)
    require_u64!(sequence)
    require_u64!(key_epoch)
    if byte_size(originator_id) > @max_identity_bytes, do: fail!("length_limit_exceeded")
    if byte_size(content_type) > @max_content_type_bytes, do: fail!("length_limit_exceeded")
    unless String.valid?(content_type), do: fail!("invalid_field")

    %MessageFields{
      message_id: message_id,
      timestamp_ns: timestamp_ns,
      originator_id: originator_id,
      channel_id: channel_id,
      sequence: sequence,
      key_epoch: key_epoch,
      content_type: content_type
    }
  end

  def __new_message!(attributes) do
    fields =
      __new_fields!(
        Map.fetch!(attributes, :message_id),
        Map.fetch!(attributes, :timestamp_ns),
        Map.fetch!(attributes, :originator_id),
        Map.fetch!(attributes, :channel_id),
        Map.fetch!(attributes, :sequence),
        Map.fetch!(attributes, :key_epoch),
        Map.fetch!(attributes, :content_type)
      )

    plaintext_hash = Map.fetch!(attributes, :plaintext_hash)
    ciphertext = Map.fetch!(attributes, :ciphertext)
    authentication_tag = Map.fetch!(attributes, :authentication_tag)
    originator_signature = Map.fetch!(attributes, :originator_signature)

    Enum.each(
      [plaintext_hash, ciphertext, authentication_tag, originator_signature],
      &require_binary!/1
    )

    require_length!(plaintext_hash, 32)
    if byte_size(ciphertext) > @max_ciphertext_bytes, do: fail!("length_limit_exceeded")
    require_length!(authentication_tag, 16)
    require_length!(originator_signature, 64)

    struct!(
      D18Message,
      Map.merge(Map.from_struct(fields), %{
        plaintext_hash: plaintext_hash,
        ciphertext: ciphertext,
        authentication_tag: authentication_tag,
        originator_signature: originator_signature
      })
    )
  rescue
    error in MessageProfileError -> raise error
    _ -> fail!("invalid_field")
  end

  def validate_message_fields(%MessageFields{} = fields) do
    validate_uuid_v7!(fields.message_id)
    validate_uuid_v7!(fields.channel_id)
    require_u64!(fields.timestamp_ns)
    require_u64!(fields.sequence)
    require_u64!(fields.key_epoch)
    if fields.originator_id == <<>>, do: fail!("invalid_field")
    if byte_size(fields.originator_id) > @max_identity_bytes, do: fail!("length_limit_exceeded")

    if byte_size(fields.content_type) > @max_content_type_bytes,
      do: fail!("length_limit_exceeded")

    validate_mime!(fields.content_type)
    :ok
  end

  def validate_message_fields(_), do: fail!("invalid_field")

  def message_create(%MessageFields{} = fields, plaintext, signing_secret_key, channel_master_key) do
    validate_message_fields(fields)
    Enum.each([plaintext, signing_secret_key, channel_master_key], &require_binary!/1)
    if byte_size(plaintext) > @max_ciphertext_bytes, do: fail!("length_limit_exceeded")
    require_length!(signing_secret_key, 64)
    require_length!(channel_master_key, 32)
    plaintext_hash = Sha256.sha256(plaintext)
    header = authenticated_header(fields, plaintext_hash)

    {ciphertext, authentication_tag} =
      ChaCha20Poly1305.xchacha20_poly1305_encrypt(
        plaintext,
        channel_master_key,
        message_nonce(fields.channel_id, fields.sequence),
        header
      )

    D18Message.new!(
      Map.merge(Map.from_struct(fields), %{
        plaintext_hash: plaintext_hash,
        ciphertext: ciphertext,
        authentication_tag: authentication_tag,
        originator_signature: Ed25519.sign(header, signing_secret_key)
      })
    )
  rescue
    error in MessageProfileError -> raise error
    _ -> fail!("invalid_field")
  end

  def message_create(_, _, _, _), do: fail!("invalid_field")

  def message_create_with_sources(
        %SourcedMessageFields{} = fields,
        plaintext,
        signing_secret_key,
        channel_master_key,
        uuid_source,
        clock
      )
      when is_function(uuid_source, 0) and is_function(clock, 0) do
    complete =
      MessageFields.new!(
        uuid_source.(),
        clock.(),
        fields.originator_id,
        fields.channel_id,
        fields.sequence,
        fields.key_epoch,
        fields.content_type
      )

    message_create(complete, plaintext, signing_secret_key, channel_master_key)
  rescue
    error in MessageProfileError -> raise error
    _ -> fail!("invalid_field")
  end

  def message_create_with_sources(_, _, _, _, _, _), do: fail!("invalid_field")

  def message_verify(%D18Message{} = message, originator_public_key, channel_master_key) do
    validate_message_fields(message_fields(message))
    require_binary!(channel_master_key)
    require_length!(channel_master_key, 32)
    verify_cryptography(message, originator_public_key, channel_master_key)
  end

  def message_verify(_, _, _), do: fail!("invalid_field")

  def message_verify_with_key_resolver(
        %D18Message{} = message,
        originator_public_key,
        key_for_epoch
      )
      when is_function(key_for_epoch, 1) do
    validate_message_fields(message_fields(message))
    key = key_for_epoch.(message.key_epoch)
    if key == nil, do: fail!("missing_epoch_key")
    require_binary!(key)
    require_length!(key, 32)
    verify_cryptography(message, originator_public_key, key)
  end

  def message_verify_with_key_resolver(_, _, _), do: fail!("invalid_field")

  defp verify_cryptography(message, originator_public_key, channel_master_key) do
    require_binary!(originator_public_key)
    require_length!(originator_public_key, 32)
    header = message_authenticated_header(message)

    signature_valid =
      try do
        Ed25519.verify(header, message.originator_signature, originator_public_key)
      rescue
        _ -> false
      end

    unless signature_valid, do: fail!("invalid_signature")

    plaintext =
      case ChaCha20Poly1305.xchacha20_poly1305_decrypt(
             message.ciphertext,
             channel_master_key,
             message_nonce(message.channel_id, message.sequence),
             header,
             message.authentication_tag
           ) do
        {:ok, value} -> value
        _ -> fail!("authentication_failed")
      end

    unless equal_bytes?(Sha256.sha256(plaintext), message.plaintext_hash),
      do: fail!("plaintext_hash_mismatch")

    plaintext
  rescue
    error in MessageProfileError -> raise error
    _ -> fail!("invalid_signature")
  end

  def message_authenticated_header(%D18Message{} = message) do
    authenticated_header(message_fields(message), message.plaintext_hash)
  end

  def message_authenticated_header(_), do: fail!("invalid_field")

  defp authenticated_header(fields, plaintext_hash) do
    frame([
      @message_context,
      fields.message_id,
      u64be(fields.timestamp_ns),
      fields.originator_id,
      fields.channel_id,
      u64be(fields.sequence),
      u64be(fields.key_epoch),
      fields.content_type,
      plaintext_hash
    ])
  end

  defp frame(parts), do: Enum.map_join(parts, fn part -> u64be(byte_size(part)) <> part end)
  defp message_nonce(channel_id, sequence), do: channel_id <> u64be(sequence)

  def message_serialize(%D18Message{} = message) do
    <<
      @message_magic::binary,
      @wire_version,
      message.message_id::binary-size(16),
      message.timestamp_ns::unsigned-big-64,
      byte_size(message.originator_id)::unsigned-big-32,
      message.originator_id::binary,
      message.channel_id::binary-size(16),
      message.sequence::unsigned-big-64,
      message.key_epoch::unsigned-big-64,
      byte_size(message.content_type)::unsigned-big-32,
      message.content_type::binary,
      message.plaintext_hash::binary-size(32),
      byte_size(message.ciphertext)::unsigned-big-64,
      message.ciphertext::binary,
      message.authentication_tag::binary-size(16),
      message.originator_signature::binary-size(64)
    >>
  rescue
    _ -> fail!("invalid_field")
  end

  def message_serialize(_), do: fail!("invalid_field")

  def message_deserialize(data) when is_binary(data) do
    {magic, position} = take!(data, 0, 4)
    unless magic == @message_magic, do: fail!("invalid_magic")
    {<<version>>, position} = take!(data, position, 1)
    unless version == @wire_version, do: fail!("unsupported_version")
    {message_id, position} = take!(data, position, 16)
    {timestamp_ns, position} = read_u64!(data, position)
    {originator_id, position} = read_bounded_u32!(data, position, @max_identity_bytes)
    {channel_id, position} = take!(data, position, 16)
    {sequence, position} = read_u64!(data, position)
    {key_epoch, position} = read_u64!(data, position)
    {content_type, position} = read_bounded_u32!(data, position, @max_content_type_bytes)
    unless String.valid?(content_type), do: fail!("invalid_utf8")
    {plaintext_hash, position} = take!(data, position, 32)
    {ciphertext, position} = read_bounded_u64!(data, position, @max_ciphertext_bytes)
    {authentication_tag, position} = take!(data, position, 16)
    {originator_signature, position} = take!(data, position, 64)
    unless position == byte_size(data), do: fail!("trailing_bytes")

    D18Message.new!(%{
      message_id: message_id,
      timestamp_ns: timestamp_ns,
      originator_id: originator_id,
      channel_id: channel_id,
      sequence: sequence,
      key_epoch: key_epoch,
      content_type: content_type,
      plaintext_hash: plaintext_hash,
      ciphertext: ciphertext,
      authentication_tag: authentication_tag,
      originator_signature: originator_signature
    })
  end

  def message_deserialize(_), do: fail!("invalid_field")

  def message_to_json(%D18Message{} = message) do
    values = [
      {"record_type", Jason.encode!("D18M")},
      {"wire_version", "1"},
      {"message_id", Jason.encode!(uuid_string(message.message_id))},
      {"timestamp_ns", Jason.encode!(Integer.to_string(message.timestamp_ns))},
      {"originator_id_b64", Jason.encode!(Base.encode64(message.originator_id))},
      {"channel_id", Jason.encode!(uuid_string(message.channel_id))},
      {"sequence", Jason.encode!(Integer.to_string(message.sequence))},
      {"key_epoch", Jason.encode!(Integer.to_string(message.key_epoch))},
      {"content_type", Jason.encode!(message.content_type)},
      {"plaintext_hash_hex", Jason.encode!(Base.encode16(message.plaintext_hash, case: :lower))},
      {"ciphertext_b64", Jason.encode!(Base.encode64(message.ciphertext))},
      {"authentication_tag_b64", Jason.encode!(Base.encode64(message.authentication_tag))},
      {"originator_signature_b64", Jason.encode!(Base.encode64(message.originator_signature))}
    ]

    encoded =
      "{" <>
        Enum.map_join(values, ",", fn {name, value} -> Jason.encode!(name) <> ":" <> value end) <>
        "}"

    if byte_size(encoded) > @max_message_json_bytes, do: fail!("length_limit_exceeded")
    encoded
  rescue
    error in MessageProfileError -> raise error
    _ -> fail!("invalid_field")
  end

  def message_to_json(_), do: fail!("invalid_field")

  def message_from_json(data) when is_binary(data) do
    if byte_size(data) > @max_message_json_bytes, do: fail!("length_limit_exceeded")
    unless String.valid?(data), do: fail!("invalid_json")
    validate_json_surrogates!(data)

    pairs =
      case Jason.decode(data, objects: :ordered_objects) do
        {:ok, %Jason.OrderedObject{values: values}} -> values
        _ -> fail!("invalid_json")
      end

    keys = Enum.map(pairs, &elem(&1, 0))
    unless length(keys) == length(Enum.uniq(keys)), do: fail!("invalid_json")
    unless Enum.sort(keys) == Enum.sort(@json_fields), do: fail!("invalid_json")
    value = Map.new(pairs)
    unless is_integer(value["wire_version"]), do: fail!("invalid_json")

    Enum.each(@json_fields -- ["wire_version"], fn name ->
      unless is_binary(value[name]), do: fail!("invalid_json")
    end)

    unless value["record_type"] == "D18M", do: fail!("invalid_magic")
    unless value["wire_version"] == 1, do: fail!("unsupported_version")
    message_id = decode_uuid_v7!(value["message_id"])
    timestamp_ns = decode_decimal!(value["timestamp_ns"])
    originator_id = decode_base64!(value["originator_id_b64"], @max_identity_bytes)
    channel_id = decode_uuid_v7!(value["channel_id"])
    sequence = decode_decimal!(value["sequence"])
    key_epoch = decode_decimal!(value["key_epoch"])
    content_type = value["content_type"]
    if byte_size(content_type) > @max_content_type_bytes, do: fail!("length_limit_exceeded")
    plaintext_hash = decode_hex!(value["plaintext_hash_hex"], 32)
    ciphertext = decode_base64!(value["ciphertext_b64"], @max_ciphertext_bytes)
    authentication_tag = decode_base64!(value["authentication_tag_b64"], 16, 16)
    originator_signature = decode_base64!(value["originator_signature_b64"], 64, 64)

    D18Message.new!(%{
      message_id: message_id,
      timestamp_ns: timestamp_ns,
      originator_id: originator_id,
      channel_id: channel_id,
      sequence: sequence,
      key_epoch: key_epoch,
      content_type: content_type,
      plaintext_hash: plaintext_hash,
      ciphertext: ciphertext,
      authentication_tag: authentication_tag,
      originator_signature: originator_signature
    })
  rescue
    error in MessageProfileError -> raise error
    _ -> fail!("invalid_json")
  end

  def message_from_json(_), do: fail!("invalid_field")

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

  defp validate_uuid_v7!(value) do
    require_binary!(value)
    require_length!(value, 16)
    <<_::binary-size(6), version::4, _::12, variant::2, _::6, _::binary>> = value
    unless version == 7 and variant == 2, do: fail!("invalid_field")
  end

  defp uuid_string(value) do
    require_length!(value, 16)
    hex = Base.encode16(value, case: :lower)

    Enum.join(
      [
        binary_part(hex, 0, 8),
        binary_part(hex, 8, 4),
        binary_part(hex, 12, 4),
        binary_part(hex, 16, 4),
        binary_part(hex, 20, 12)
      ],
      "-"
    )
  end

  defp decode_uuid_v7!(value) do
    unless Regex.match?(
             ~r/\A[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\z/,
             value
           ),
           do: fail!("invalid_field")

    case Base.decode16(String.replace(value, "-", ""), case: :lower) do
      {:ok, decoded} ->
        validate_uuid_v7!(decoded)
        decoded

      :error ->
        fail!("invalid_field")
    end
  end

  defp decode_decimal!(value) do
    unless Regex.match?(~r/\A(?:0|[1-9][0-9]*)\z/, value), do: fail!("invalid_field")

    case Integer.parse(value) do
      {decoded, ""} ->
        require_u64!(decoded)
        decoded

      _ ->
        fail!("invalid_field")
    end
  end

  defp decode_base64!(value, maximum, exact \\ nil) do
    unless rem(byte_size(value), 4) == 0, do: fail!("invalid_field")
    if div(byte_size(value), 4) * 3 > maximum + 2, do: fail!("length_limit_exceeded")

    case Base.decode64(value, padding: true) do
      {:ok, decoded} ->
        if byte_size(decoded) > maximum, do: fail!("length_limit_exceeded")
        if exact != nil and byte_size(decoded) != exact, do: fail!("invalid_field")
        unless Base.encode64(decoded) == value, do: fail!("invalid_field")
        decoded

      :error ->
        fail!("invalid_field")
    end
  end

  defp decode_hex!(value, length) do
    unless byte_size(value) == length * 2, do: fail!("invalid_field")

    case Base.decode16(value, case: :lower) do
      {:ok, decoded} -> decoded
      :error -> fail!("invalid_field")
    end
  end

  defp validate_mime!(value) do
    bytes = :binary.bin_to_list(value)

    if bytes == [] or Enum.any?(bytes, &(&1 < 0x20 or &1 > 0x7E)),
      do: fail!("invalid_field")

    index = consume_token!(bytes, 0)
    unless Enum.at(bytes, index) == ?/, do: fail!("invalid_field")
    index = consume_token!(bytes, index + 1)
    consume_parameters!(bytes, index)
  end

  defp consume_parameters!(bytes, index) when index >= length(bytes), do: :ok

  defp consume_parameters!(bytes, index) do
    index = consume_spaces(bytes, index)
    unless Enum.at(bytes, index) == ?;, do: fail!("invalid_field")
    index = consume_spaces(bytes, index + 1)
    index = consume_token!(bytes, index)
    index = consume_spaces(bytes, index)
    unless Enum.at(bytes, index) == ?=, do: fail!("invalid_field")
    index = consume_spaces(bytes, index + 1)

    index =
      if Enum.at(bytes, index) == ?" do
        consume_quoted!(bytes, index + 1)
      else
        consume_token!(bytes, index)
      end

    consume_parameters!(bytes, index)
  end

  defp consume_quoted!(bytes, index) when index >= length(bytes), do: fail!("invalid_field")

  defp consume_quoted!(bytes, index) do
    case Enum.at(bytes, index) do
      ?" ->
        index + 1

      ?\\ ->
        if(index + 1 < length(bytes),
          do: consume_quoted!(bytes, index + 2),
          else: fail!("invalid_field")
        )

      _ ->
        consume_quoted!(bytes, index + 1)
    end
  end

  defp consume_token!(bytes, index) do
    next =
      Enum.reduce_while(index..(length(bytes) - 1), index, fn position, _ ->
        if mime_token?(Enum.at(bytes, position)),
          do: {:cont, position + 1},
          else: {:halt, position}
      end)

    if next == index, do: fail!("invalid_field")
    next
  end

  defp consume_spaces(bytes, index) do
    if Enum.at(bytes, index) == ?\s, do: consume_spaces(bytes, index + 1), else: index
  end

  defp mime_token?(byte) when is_integer(byte) do
    byte in ?0..?9 or byte in ?A..?Z or byte in ?a..?z or byte in ~c"!#$%&'*+-.^_`|~"
  end

  defp mime_token?(_), do: false

  defp validate_json_surrogates!(data), do: scan_json(data, 0, false)
  defp scan_json(data, index, _in_string) when index >= byte_size(data), do: :ok

  defp scan_json(data, index, false) do
    scan_json(data, index + 1, :binary.at(data, index) == ?")
  end

  defp scan_json(data, index, true) do
    case :binary.at(data, index) do
      ?" -> scan_json(data, index + 1, false)
      ?\\ -> scan_json_escape(data, index + 1)
      _ -> scan_json(data, index + 1, true)
    end
  end

  defp scan_json_escape(data, index) when index >= byte_size(data), do: :ok

  defp scan_json_escape(data, index) do
    if :binary.at(data, index) == ?u and index + 4 < byte_size(data) do
      hex = binary_part(data, index + 1, 4)

      case Integer.parse(hex, 16) do
        {code, ""} when code in 0xD800..0xDBFF ->
          if index + 10 < byte_size(data) and binary_part(data, index + 5, 2) == "\\u" do
            low_hex = binary_part(data, index + 7, 4)

            case Integer.parse(low_hex, 16) do
              {low, ""} when low in 0xDC00..0xDFFF -> scan_json(data, index + 11, true)
              _ -> fail!("invalid_field")
            end
          else
            fail!("invalid_field")
          end

        {code, ""} when code in 0xDC00..0xDFFF ->
          fail!("invalid_field")

        {_code, ""} ->
          scan_json(data, index + 5, true)

        _ ->
          scan_json(data, index + 1, true)
      end
    else
      scan_json(data, index + 1, true)
    end
  end

  defp take!(data, position, length) when length >= 0 do
    if position + length > byte_size(data), do: fail!("truncated_record")
    {binary_part(data, position, length), position + length}
  end

  defp read_u64!(data, position) do
    {<<value::unsigned-big-64>>, position} = take!(data, position, 8)
    {value, position}
  end

  defp read_bounded_u32!(data, position, maximum) do
    {<<length::unsigned-big-32>>, position} = take!(data, position, 4)
    if length > maximum, do: fail!("length_limit_exceeded")
    take!(data, position, length)
  end

  defp read_bounded_u64!(data, position, maximum) do
    {length, position} = read_u64!(data, position)
    if length > maximum, do: fail!("length_limit_exceeded")
    take!(data, position, length)
  end

  defp require_binary!(value), do: unless(is_binary(value), do: fail!("invalid_field"))

  defp require_length!(value, length),
    do: unless(byte_size(value) == length, do: fail!("invalid_field"))

  defp require_u64!(value),
    do:
      unless(is_integer(value) and value >= 0 and value <= @max_u64,
        do: fail!("invalid_field")
      )

  defp u64be(value),
    do:
      (
        require_u64!(value)
        <<value::unsigned-big-64>>
      )

  defp equal_bytes?(left, right) when byte_size(left) != byte_size(right), do: false

  defp equal_bytes?(left, right) do
    left
    |> :binary.bin_to_list()
    |> Enum.zip(:binary.bin_to_list(right))
    |> Enum.reduce(0, fn {a, b}, acc -> acc ||| bxor(a, b) end) == 0
  end

  defp fail!(code), do: raise(MessageProfileError, code: code)
end
