defmodule CodingAdventures.ChiefOfStaffChannelStore do
  @moduledoc "Portable D18P durable-channel values, codecs, keys, and stable failures."

  import Bitwise
  alias CodingAdventures.Sha256

  @storage_namespace "chief-channels"
  @definition_content_type "application/vnd.coding-adventures.chief-channel-definition-v1"
  @state_content_type "application/vnd.coding-adventures.chief-channel-state-v1"
  @message_content_type "application/vnd.coding-adventures.chief-channel-message-v1"
  @grant_content_type "application/vnd.coding-adventures.chief-channel-key-grant-v1"
  @ack_content_type "application/vnd.coding-adventures.chief-channel-ack-v1"
  @max_identity_bytes 4 * 1024
  @max_content_type_bytes 1024
  @max_receivers 1024
  @max_pending_header_bytes 16 * 1024
  @max_store_cas_attempts 16
  @max_definition_cas_attempts 16
  @max_u64 (1 <<< 64) - 1
  @error_codes ~w(invalid_definition invalid_message_id definition_not_found
    conflicting_definition corrupt_definition definition_changed channel_destroyed
    unauthorized_originator unauthorized_receiver public_key_mismatch missing_key_grant
    unknown_message_id unauthorized_message not_initialized corrupt_record pending_append
    no_pending_append pending_header_mismatch conflicting_record concurrent_update
    invalid_receiver_id invalid_page_size acknowledgement_regression acknowledgement_ahead
    acknowledgement_pending sequence_exhausted storage_error wire_error crypto_error
    metadata_error)

  defmodule ProfileError do
    @moduledoc "One fail-closed D18P operation error with a stable portable code."
    defexception [:code]
    @impl true
    def message(%{code: code}), do: code
  end

  defmodule OriginatorIdentity do
    @enforce_keys [:agent_id, :public_key]
    defstruct @enforce_keys
  end

  defmodule ReceiverIdentity do
    @enforce_keys [:agent_id, :public_key]
    defstruct @enforce_keys
  end

  defmodule ChannelDefinition do
    @enforce_keys [:channel_id, :originator, :receivers, :created_at_ns, :key_epoch, :lifecycle]
    defstruct @enforce_keys
  end

  defmodule MessageHeader do
    @enforce_keys [
      :message_id,
      :timestamp_ns,
      :originator_id,
      :channel_id,
      :sequence,
      :key_epoch,
      :content_type,
      :plaintext_hash
    ]
    defstruct @enforce_keys
  end

  defmodule ChannelState do
    @enforce_keys [:next_sequence]
    defstruct [:next_sequence, pending_header: nil]
  end

  def storage_namespace, do: @storage_namespace
  def definition_content_type, do: @definition_content_type
  def state_content_type, do: @state_content_type
  def message_content_type, do: @message_content_type
  def grant_content_type, do: @grant_content_type
  def ack_content_type, do: @ack_content_type
  def max_identity_bytes, do: @max_identity_bytes
  def max_receivers, do: @max_receivers
  def max_pending_header_bytes, do: @max_pending_header_bytes
  def max_store_cas_attempts, do: @max_store_cas_attempts
  def max_definition_cas_attempts, do: @max_definition_cas_attempts
  def max_u64, do: @max_u64
  def error_codes, do: @error_codes

  def fail!(code) when code in @error_codes, do: raise(ProfileError, code: code)
  def fail!(_), do: raise(ArgumentError, "unknown D18P error code")

  def new_originator!(agent_id, public_key) do
    validate_agent_id!(agent_id, "invalid_definition")
    require_binary_length!(public_key, 32, "invalid_definition")
    %OriginatorIdentity{agent_id: agent_id, public_key: public_key}
  end

  def new_receiver!(agent_id, public_key) do
    validate_agent_id!(agent_id, "invalid_definition")
    require_binary_length!(public_key, 32, "invalid_definition")
    %ReceiverIdentity{agent_id: agent_id, public_key: public_key}
  end

  def new_definition!(
        channel_id,
        originator,
        receivers,
        created_at_ns,
        key_epoch,
        lifecycle \\ "active"
      ) do
    validate_uuid_v7!(channel_id, "invalid_definition")
    unless match?(%OriginatorIdentity{}, originator), do: fail!("invalid_definition")
    canonical_originator = new_originator!(originator.agent_id, originator.public_key)
    unless is_list(receivers), do: fail!("invalid_definition")

    canonical_receivers =
      receivers
      |> Enum.map(fn
        %ReceiverIdentity{} = receiver -> new_receiver!(receiver.agent_id, receiver.public_key)
        _ -> fail!("invalid_definition")
      end)
      |> Enum.sort_by(& &1.agent_id)

    unless length(canonical_receivers) in 1..@max_receivers, do: fail!("invalid_definition")
    require_u64!(created_at_ns, "invalid_definition")
    require_u64!(key_epoch, "invalid_definition")
    unless lifecycle in ["active", "destroyed"], do: fail!("invalid_definition")

    canonical_receivers
    |> Enum.reduce(nil, fn receiver, previous ->
      if receiver.agent_id == canonical_originator.agent_id or receiver.agent_id == previous,
        do: fail!("invalid_definition")

      receiver.agent_id
    end)

    %ChannelDefinition{
      channel_id: channel_id,
      originator: canonical_originator,
      receivers: canonical_receivers,
      created_at_ns: created_at_ns,
      key_epoch: key_epoch,
      lifecycle: lifecycle
    }
  end

  def receiver(%ChannelDefinition{} = definition, agent_id) do
    Enum.find(definition.receivers, &(&1.agent_id == agent_id))
  end

  def with_lifecycle!(%ChannelDefinition{} = definition, lifecycle) do
    new_definition!(
      definition.channel_id,
      definition.originator,
      definition.receivers,
      definition.created_at_ns,
      definition.key_epoch,
      lifecycle
    )
  end

  def new_header!(attributes) when is_map(attributes) do
    message_id = Map.fetch!(attributes, :message_id)
    timestamp_ns = Map.fetch!(attributes, :timestamp_ns)
    originator_id = Map.fetch!(attributes, :originator_id)
    channel_id = Map.fetch!(attributes, :channel_id)
    sequence = Map.fetch!(attributes, :sequence)
    key_epoch = Map.fetch!(attributes, :key_epoch)
    content_type = Map.fetch!(attributes, :content_type)
    plaintext_hash = Map.fetch!(attributes, :plaintext_hash)
    require_binary_length!(message_id, 16, "wire_error")
    require_u64!(timestamp_ns, "wire_error")
    require_binary!(originator_id, "wire_error")
    if byte_size(originator_id) > @max_identity_bytes, do: fail!("wire_error")
    require_binary_length!(channel_id, 16, "wire_error")
    require_u64!(sequence, "wire_error")
    require_u64!(key_epoch, "wire_error")
    require_utf8!(content_type, @max_content_type_bytes, "wire_error")
    require_binary_length!(plaintext_hash, 32, "wire_error")

    struct!(MessageHeader, attributes)
  rescue
    error in ProfileError -> raise error
    _ -> fail!("wire_error")
  end

  def definition_serialize(%ChannelDefinition{} = definition) do
    receiver_bytes =
      Enum.map(definition.receivers, fn item ->
        [sized32!(item.agent_id), item.public_key]
      end)

    IO.iodata_to_binary([
      "D18C",
      <<1>>,
      definition.channel_id,
      sized32!(definition.originator.agent_id),
      definition.originator.public_key,
      <<length(definition.receivers)::unsigned-big-32>>,
      receiver_bytes,
      <<definition.created_at_ns::unsigned-big-64, definition.key_epoch::unsigned-big-64>>,
      if(definition.lifecycle == "active", do: <<0>>, else: <<1>>)
    ])
  end

  def definition_deserialize(data) do
    remap("corrupt_definition", fn ->
      {"D18C", rest} = take!(data, 4, "corrupt_definition")
      {1, rest} = u8!(rest, "corrupt_definition")
      {channel_id, rest} = take!(rest, 16, "corrupt_definition")
      {originator_id, rest} = sized32_read!(rest, @max_identity_bytes, "corrupt_definition")
      {originator_key, rest} = take!(rest, 32, "corrupt_definition")
      {count, rest} = u32!(rest, "corrupt_definition")
      unless count in 1..@max_receivers, do: fail!("corrupt_definition")
      {receivers, rest} = read_receivers!(count, rest, [])
      {created_at_ns, rest} = u64!(rest, "corrupt_definition")
      {key_epoch, rest} = u64!(rest, "corrupt_definition")
      {lifecycle, rest} = u8!(rest, "corrupt_definition")
      unless lifecycle in [0, 1] and rest == <<>>, do: fail!("corrupt_definition")

      new_definition!(
        channel_id,
        new_originator!(originator_id, originator_key),
        receivers,
        created_at_ns,
        key_epoch,
        if(lifecycle == 0, do: "active", else: "destroyed")
      )
    end)
  end

  def header_serialize(%MessageHeader{} = header) do
    IO.iodata_to_binary([
      "D18H",
      <<1>>,
      header.message_id,
      <<header.timestamp_ns::unsigned-big-64>>,
      sized32!(header.originator_id),
      header.channel_id,
      <<header.sequence::unsigned-big-64, header.key_epoch::unsigned-big-64>>,
      sized32!(header.content_type),
      header.plaintext_hash
    ])
  end

  def header_deserialize(data) do
    remap("wire_error", fn ->
      {"D18H", rest} = take!(data, 4, "wire_error")
      {1, rest} = u8!(rest, "wire_error")
      {message_id, rest} = take!(rest, 16, "wire_error")
      {timestamp_ns, rest} = u64!(rest, "wire_error")
      {originator_id, rest} = sized32_read!(rest, @max_identity_bytes, "wire_error")
      {channel_id, rest} = take!(rest, 16, "wire_error")
      {sequence, rest} = u64!(rest, "wire_error")
      {key_epoch, rest} = u64!(rest, "wire_error")
      {content_type, rest} = sized32_read!(rest, @max_content_type_bytes, "wire_error")
      unless String.valid?(content_type), do: fail!("wire_error")
      {plaintext_hash, rest} = take!(rest, 32, "wire_error")
      unless rest == <<>>, do: fail!("wire_error")

      new_header!(%{
        message_id: message_id,
        timestamp_ns: timestamp_ns,
        originator_id: originator_id,
        channel_id: channel_id,
        sequence: sequence,
        key_epoch: key_epoch,
        content_type: content_type,
        plaintext_hash: plaintext_hash
      })
    end)
  end

  def state_serialize(%ChannelState{} = state) do
    require_u64!(state.next_sequence, "corrupt_record")

    pending =
      case state.pending_header do
        nil ->
          <<0>>

        %MessageHeader{} = header ->
          encoded = header_serialize(header)
          if byte_size(encoded) > @max_pending_header_bytes, do: fail!("corrupt_record")
          <<1, byte_size(encoded)::unsigned-big-32, encoded::binary>>

        _ ->
          fail!("corrupt_record")
      end

    <<"D18S", 1, state.next_sequence::unsigned-big-64, pending::binary>>
  end

  def state_deserialize(data, channel_id) do
    remap("corrupt_record", fn ->
      {"D18S", rest} = take!(data, 4, "corrupt_record")
      {1, rest} = u8!(rest, "corrupt_record")
      {next_sequence, rest} = u64!(rest, "corrupt_record")
      {flag, rest} = u8!(rest, "corrupt_record")

      pending =
        case flag do
          0 ->
            unless rest == <<>>, do: fail!("corrupt_record")
            nil

          1 ->
            {length, rest} = u32!(rest, "corrupt_record")
            if length > @max_pending_header_bytes, do: fail!("corrupt_record")
            {encoded, rest} = take!(rest, length, "corrupt_record")
            unless rest == <<>>, do: fail!("corrupt_record")
            header = remap("corrupt_record", fn -> header_deserialize(encoded) end)

            if header.channel_id != channel_id or header.sequence == @max_u64 or
                 header.sequence + 1 != next_sequence,
               do: fail!("corrupt_record")

            header

          _ ->
            fail!("corrupt_record")
        end

      %ChannelState{next_sequence: next_sequence, pending_header: pending}
    end)
  end

  def cursor_serialize(first_unread_sequence) do
    require_u64!(first_unread_sequence, "corrupt_record")
    <<"D18A", 1, first_unread_sequence::unsigned-big-64>>
  end

  def cursor_deserialize(data) do
    remap("corrupt_record", fn ->
      {"D18A", rest} = take!(data, 4, "corrupt_record")
      {1, rest} = u8!(rest, "corrupt_record")
      {cursor, <<>>} = u64!(rest, "corrupt_record")
      cursor
    end)
  end

  def definition_key(channel_id) do
    require_binary_length!(channel_id, 16, "invalid_definition")
    Base.encode16(channel_id, case: :lower) <> "/definition"
  end

  def state_key(channel_id) do
    require_binary_length!(channel_id, 16, "invalid_definition")
    Base.encode16(channel_id, case: :lower) <> "/state/next-sequence"
  end

  def message_prefix(channel_id) do
    require_binary_length!(channel_id, 16, "invalid_definition")
    Base.encode16(channel_id, case: :lower) <> "/messages/"
  end

  def message_key(channel_id, sequence), do: message_prefix(channel_id) <> decimal20!(sequence)

  def grant_key(channel_id, key_epoch, receiver_id) do
    validate_agent_id!(receiver_id, "invalid_receiver_id")

    Base.encode16(channel_id, case: :lower) <>
      "/grants/" <>
      decimal20!(key_epoch) <>
      "/" <> Base.encode16(Sha256.sha256(receiver_id), case: :lower)
  end

  def ack_key(channel_id, receiver_id) do
    validate_agent_id!(receiver_id, "invalid_receiver_id")

    Base.encode16(channel_id, case: :lower) <>
      "/receivers/" <> Base.encode16(Sha256.sha256(receiver_id), case: :lower) <> "/ack"
  end

  def validate_uuid_v7!(value, code \\ "invalid_message_id") do
    require_binary_length!(value, 16, code)
    <<_::binary-size(6), version, _::binary-size(1), variant, _::binary>> = value
    unless version >>> 4 == 7 and (variant &&& 0xC0) == 0x80, do: fail!(code)
    :ok
  end

  def validate_agent_id!(value, code) do
    require_binary!(value, code)
    unless byte_size(value) in 1..@max_identity_bytes, do: fail!(code)
    :ok
  end

  def require_u64!(value, code) do
    unless is_integer(value) and value >= 0 and value <= @max_u64, do: fail!(code)
    :ok
  end

  def remap(code, operation) when is_function(operation, 0) do
    operation.()
  rescue
    error in ProfileError ->
      if error.code == code, do: raise(error), else: fail!(code)

    _ ->
      fail!(code)
  end

  defp require_binary!(value, code), do: unless(is_binary(value), do: fail!(code))

  defp require_binary_length!(value, length, code) do
    require_binary!(value, code)
    unless byte_size(value) == length, do: fail!(code)
  end

  defp require_utf8!(value, maximum, code) do
    require_binary!(value, code)
    unless String.valid?(value) and byte_size(value) <= maximum, do: fail!(code)
  end

  defp sized32!(value) when is_binary(value) and byte_size(value) <= 0xFFFF_FFFF,
    do: <<byte_size(value)::unsigned-big-32, value::binary>>

  defp sized32!(_), do: fail!("wire_error")

  defp decimal20!(value) do
    require_u64!(value, "corrupt_record")
    value |> Integer.to_string() |> String.pad_leading(20, "0")
  end

  defp take!(data, length, code) when is_binary(data) and is_integer(length) and length >= 0 do
    if byte_size(data) < length, do: fail!(code)
    <<value::binary-size(length), rest::binary>> = data
    {value, rest}
  end

  defp take!(_, _, code), do: fail!(code)

  defp u8!(data, code) do
    {<<value>>, rest} = take!(data, 1, code)
    {value, rest}
  end

  defp u32!(data, code) do
    {<<value::unsigned-big-32>>, rest} = take!(data, 4, code)
    {value, rest}
  end

  defp u64!(data, code) do
    {<<value::unsigned-big-64>>, rest} = take!(data, 8, code)
    {value, rest}
  end

  defp sized32_read!(data, maximum, code) do
    {length, rest} = u32!(data, code)
    if length > maximum, do: fail!(code)
    take!(rest, length, code)
  end

  defp read_receivers!(0, rest, receivers), do: {Enum.reverse(receivers), rest}

  defp read_receivers!(count, data, receivers) do
    {agent_id, rest} = sized32_read!(data, @max_identity_bytes, "corrupt_definition")
    {public_key, rest} = take!(rest, 32, "corrupt_definition")
    read_receivers!(count - 1, rest, [new_receiver!(agent_id, public_key) | receivers])
  end
end
