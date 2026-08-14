defmodule CodingAdventures.ChiefOfStaffChannelStore.MessageMetadata do
  @enforce_keys [:message_id, :timestamp_ns]
  defstruct @enforce_keys
end

defmodule CodingAdventures.ChiefOfStaffChannelStore.PublishedMessage do
  @enforce_keys [:message_id, :sequence, :timestamp_ns]
  defstruct @enforce_keys
end

defmodule CodingAdventures.ChiefOfStaffChannelStore.ReceivedMessage do
  @enforce_keys [:message_id, :sequence, :timestamp_ns, :content_type, :payload]
  defstruct @enforce_keys
end

defmodule CodingAdventures.ChiefOfStaffChannelStore.ChannelDefinitionStore do
  @moduledoc "Atomic creation, loading, and irreversible retirement of D18C records."

  alias CodingAdventures.ChiefOfStaffChannelStore, as: Profile

  alias CodingAdventures.ChiefOfStaffChannelStore.{
    Backend,
    ChannelDefinition,
    ChannelStore,
    StorageConflictError,
    StoragePut
  }

  @enforce_keys [:backend]
  defstruct @enforce_keys

  def new(backend), do: %__MODULE__{backend: backend}

  def create!(%__MODULE__{} = store, %ChannelDefinition{} = definition) do
    if definition.lifecycle != "active", do: Profile.fail!("invalid_definition")
    Backend.initialize!(store.backend)
    key = Profile.definition_key(definition.channel_id)
    body = Profile.definition_serialize(definition)

    persisted =
      try do
        record =
          Backend.put!(store.backend, %StoragePut{
            namespace: Profile.storage_namespace(),
            key: key,
            content_type: Profile.definition_content_type(),
            body: body,
            if_absent: true
          })

        require_definition_record!(record, definition.channel_id)
      rescue
        _ in StorageConflictError ->
          case Backend.get!(store.backend, Profile.storage_namespace(), key) do
            nil ->
              Profile.fail!("definition_not_found")

            existing ->
              if existing.content_type != Profile.definition_content_type(),
                do: Profile.fail!("corrupt_definition")

              if existing.body != body, do: Profile.fail!("conflicting_definition")
              require_definition_record!(existing, definition.channel_id)
          end
      end

    if persisted != definition, do: Profile.fail!("conflicting_definition")
    store.backend |> ChannelStore.new!(definition.channel_id) |> ChannelStore.initialize!()
    require_current!(store, definition)
  end

  def load!(%__MODULE__{} = store, channel_id) do
    Backend.initialize!(store.backend)

    case load_record!(store, channel_id) do
      nil -> nil
      {definition, _revision} -> definition
    end
  end

  def destroy!(%__MODULE__{} = store, channel_id) do
    Backend.initialize!(store.backend)
    destroy_attempt!(store, channel_id, Profile.max_definition_cas_attempts())
  end

  def require_current!(%__MODULE__{} = store, %ChannelDefinition{} = expected) do
    case load!(store, expected.channel_id) do
      nil -> Profile.fail!("definition_not_found")
      %{lifecycle: "destroyed"} -> Profile.fail!("channel_destroyed")
      actual when actual != expected -> Profile.fail!("definition_changed")
      actual -> actual
    end
  end

  defp destroy_attempt!(_store, _channel_id, 0), do: Profile.fail!("concurrent_update")

  defp destroy_attempt!(store, channel_id, attempts) do
    case load_record!(store, channel_id) do
      nil ->
        Profile.fail!("definition_not_found")

      {%{lifecycle: "destroyed"} = definition, _revision} ->
        definition

      {definition, revision} ->
        destroyed = Profile.with_lifecycle!(definition, "destroyed")

        try do
          store.backend
          |> Backend.put!(%StoragePut{
            namespace: Profile.storage_namespace(),
            key: Profile.definition_key(channel_id),
            content_type: Profile.definition_content_type(),
            body: Profile.definition_serialize(destroyed),
            if_revision: revision
          })
          |> require_definition_record!(channel_id)
        rescue
          _ in StorageConflictError -> destroy_attempt!(store, channel_id, attempts - 1)
        end
    end
  end

  defp load_record!(store, channel_id) do
    key = Profile.definition_key(channel_id)

    case Backend.get!(store.backend, Profile.storage_namespace(), key) do
      nil -> nil
      record -> {require_definition_record!(record, channel_id), record.revision}
    end
  end

  defp require_definition_record!(record, channel_id) do
    if record.content_type != Profile.definition_content_type(),
      do: Profile.fail!("corrupt_definition")

    definition = Profile.definition_deserialize(record.body)

    if definition.channel_id != channel_id or record.key != Profile.definition_key(channel_id),
      do: Profile.fail!("corrupt_definition")

    definition
  end
end

defmodule CodingAdventures.ChiefOfStaffChannelStore.DurableOriginator do
  @moduledoc "The only D18P endpoint role with a publish operation."

  alias CodingAdventures.ChiefOfStaffChannelStore, as: Profile

  alias CodingAdventures.ChiefOfStaffChannelStore.{
    AppendRequest,
    ChannelDefinitionStore,
    ChannelStore,
    MessageMetadata,
    OpaqueKeyGrant,
    PublishedMessage
  }

  @enforce_keys [
    :backend,
    :definition,
    :signing_secret_key,
    :channel_master_key,
    :metadata_source
  ]
  defstruct @enforce_keys

  def open!(
        backend,
        channel_id,
        agent_id,
        signing_secret_key,
        channel_master_key,
        metadata_source
      ) do
    definition = active_definition!(backend, channel_id)

    if definition.originator.agent_id != agent_id,
      do: Profile.fail!("unauthorized_originator")

    unless is_binary(signing_secret_key) and byte_size(signing_secret_key) == 64 and
             definition.originator.public_key == binary_part(signing_secret_key, 32, 32),
           do: Profile.fail!("public_key_mismatch")

    unless is_binary(channel_master_key) and byte_size(channel_master_key) == 32,
      do: Profile.fail!("crypto_error")

    backend |> ChannelStore.new!(channel_id) |> ChannelStore.initialize!()

    %__MODULE__{
      backend: backend,
      definition: definition,
      signing_secret_key: signing_secret_key,
      channel_master_key: channel_master_key,
      metadata_source: metadata_source
    }
  end

  def id(%__MODULE__{} = originator), do: originator.definition.originator.agent_id
  def channel_id(%__MODULE__{} = originator), do: originator.definition.channel_id
  def public_key(%__MODULE__{} = originator), do: originator.definition.originator.public_key

  def publish!(%__MODULE__{} = originator, payload, content_type) do
    metadata =
      try do
        originator.metadata_source.()
      rescue
        _ -> Profile.fail!("metadata_error")
      catch
        _, _ -> Profile.fail!("metadata_error")
      end

    unless match?(%MessageMetadata{}, metadata), do: Profile.fail!("metadata_error")
    publish_with_metadata!(originator, metadata, payload, content_type)
  end

  def publish_with_metadata!(
        %__MODULE__{} = originator,
        %MessageMetadata{} = metadata,
        payload,
        content_type
      ) do
    Profile.validate_uuid_v7!(metadata.message_id)

    ChannelDefinitionStore.new(originator.backend)
    |> ChannelDefinitionStore.require_current!(originator.definition)

    request = %AppendRequest{
      message_id: metadata.message_id,
      timestamp_ns: metadata.timestamp_ns,
      originator_id: originator.definition.originator.agent_id,
      key_epoch: originator.definition.key_epoch,
      content_type: content_type
    }

    message =
      originator.backend
      |> ChannelStore.new!(originator.definition.channel_id)
      |> ChannelStore.append!(
        request,
        payload,
        originator.channel_master_key,
        originator.signing_secret_key
      )

    %PublishedMessage{
      message_id: metadata.message_id,
      sequence: message.sequence,
      timestamp_ns: metadata.timestamp_ns
    }
  end

  def save_receiver_grant!(%__MODULE__{} = originator, receiver_id, grant_body) do
    definition =
      originator.backend
      |> ChannelDefinitionStore.new()
      |> ChannelDefinitionStore.require_current!(originator.definition)

    if is_nil(Profile.receiver(definition, receiver_id)),
      do: Profile.fail!("unauthorized_receiver")

    originator.backend
    |> ChannelStore.new!(definition.channel_id)
    |> ChannelStore.save_key_grant!(%OpaqueKeyGrant{
      channel_id: definition.channel_id,
      key_epoch: definition.key_epoch,
      receiver_id: receiver_id,
      body: grant_body
    })
  end

  defp active_definition!(backend, channel_id) do
    case backend |> ChannelDefinitionStore.new() |> ChannelDefinitionStore.load!(channel_id) do
      nil -> Profile.fail!("definition_not_found")
      %{lifecycle: "destroyed"} -> Profile.fail!("channel_destroyed")
      definition -> definition
    end
  end
end

defmodule CodingAdventures.ChiefOfStaffChannelStore.DurableReceiver do
  @moduledoc "Read-only D18P endpoint with session-bound acknowledgement."

  alias CodingAdventures.ChiefOfStaffChannelCrypto, as: Crypto
  alias CodingAdventures.ChiefOfStaffChannelStore, as: Profile

  alias CodingAdventures.ChiefOfStaffChannelStore.{
    ChannelDefinitionStore,
    ChannelStore,
    ReceivedMessage
  }

  @enforce_keys [:backend, :definition, :receiver_id, :key_provider, :delivered]
  defstruct @enforce_keys

  def open!(backend, channel_id, receiver_id, key_provider) do
    Profile.validate_agent_id!(receiver_id, "invalid_receiver_id")

    definition =
      case backend |> ChannelDefinitionStore.new() |> ChannelDefinitionStore.load!(channel_id) do
        nil -> Profile.fail!("definition_not_found")
        %{lifecycle: "destroyed"} -> Profile.fail!("channel_destroyed")
        value -> value
      end

    receiver = Profile.receiver(definition, receiver_id)
    if is_nil(receiver), do: Profile.fail!("unauthorized_receiver")

    if receiver.public_key != provider_public_key!(key_provider),
      do: Profile.fail!("public_key_mismatch")

    backend |> ChannelStore.new!(channel_id) |> ChannelStore.initialize!()
    {:ok, delivered} = Agent.start_link(fn -> %{} end)

    %__MODULE__{
      backend: backend,
      definition: definition,
      receiver_id: receiver_id,
      key_provider: key_provider,
      delivered: delivered
    }
  end

  def id(%__MODULE__{} = receiver), do: receiver.receiver_id
  def channel_id(%__MODULE__{} = receiver), do: receiver.definition.channel_id
  def public_key(%__MODULE__{} = receiver), do: provider_public_key!(receiver.key_provider)

  def receive!(%__MODULE__{} = receiver, limit) do
    receiver.backend
    |> ChannelDefinitionStore.new()
    |> ChannelDefinitionStore.require_current!(receiver.definition)

    store = ChannelStore.new!(receiver.backend, receiver.definition.channel_id)
    page = ChannelStore.read_for_receiver!(store, receiver.receiver_id, limit)

    Enum.map(page.messages, fn message ->
      if message.channel_id != receiver.definition.channel_id or
           message.originator_id != receiver.definition.originator.agent_id or
           message.key_epoch > receiver.definition.key_epoch,
         do: Profile.fail!("unauthorized_message")

      grant = ChannelStore.key_grant!(store, message.key_epoch, receiver.receiver_id)
      if is_nil(grant), do: Profile.fail!("missing_key_grant")
      channel_key = open_grant!(receiver.key_provider, message.key_epoch, grant)
      if is_nil(channel_key), do: Profile.fail!("missing_key_grant")

      payload =
        try do
          Crypto.message_verify(
            message,
            receiver.definition.originator.public_key,
            channel_key
          )
        rescue
          _ -> Profile.fail!("crypto_error")
        catch
          _, _ -> Profile.fail!("crypto_error")
        end

      Profile.validate_uuid_v7!(message.message_id)

      outcome =
        Agent.get_and_update(receiver.delivered, fn delivered ->
          case Map.get(delivered, message.message_id) do
            nil -> {:ok, Map.put(delivered, message.message_id, message.sequence)}
            sequence when sequence == message.sequence -> {:ok, delivered}
            _ -> {:error, delivered}
          end
        end)

      if outcome == :error, do: Profile.fail!("unauthorized_message")

      %ReceivedMessage{
        message_id: message.message_id,
        sequence: message.sequence,
        timestamp_ns: message.timestamp_ns,
        content_type: message.content_type,
        payload: payload
      }
    end)
  end

  def acknowledge!(%__MODULE__{} = receiver, message_id) do
    Profile.validate_uuid_v7!(message_id)

    receiver.backend
    |> ChannelDefinitionStore.new()
    |> ChannelDefinitionStore.require_current!(receiver.definition)

    case Agent.get(receiver.delivered, &Map.get(&1, message_id)) do
      nil ->
        Profile.fail!("unknown_message_id")

      sequence ->
        receiver.backend
        |> ChannelStore.new!(receiver.definition.channel_id)
        |> ChannelStore.acknowledge!(receiver.receiver_id, sequence)
    end
  end

  defp provider_public_key!(%{public_key: public_key}) when is_binary(public_key), do: public_key
  defp provider_public_key!(_), do: Profile.fail!("public_key_mismatch")

  defp open_grant!(%{open_grant: operation}, key_epoch, body) when is_function(operation, 2) do
    try do
      operation.(key_epoch, body)
    rescue
      _ -> Profile.fail!("crypto_error")
    catch
      _, _ -> Profile.fail!("crypto_error")
    end
  end

  defp open_grant!(_, _, _), do: Profile.fail!("crypto_error")
end
