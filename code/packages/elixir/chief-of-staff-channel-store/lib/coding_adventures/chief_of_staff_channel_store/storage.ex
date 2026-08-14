defmodule CodingAdventures.ChiefOfStaffChannelStore.StorageConflictError do
  @moduledoc "Expected failure of an atomic create or revision-CAS condition."
  defexception message: "storage conflict"
end

defmodule CodingAdventures.ChiefOfStaffChannelStore.StorageRecord do
  @enforce_keys [:namespace, :key, :content_type, :body, :revision]
  defstruct @enforce_keys
end

defmodule CodingAdventures.ChiefOfStaffChannelStore.StoragePut do
  @enforce_keys [:namespace, :key, :content_type, :body]
  defstruct [:namespace, :key, :content_type, :body, if_absent: false, if_revision: nil]
end

defmodule CodingAdventures.ChiefOfStaffChannelStore.StoragePage do
  @enforce_keys [:records]
  defstruct [:records, next_cursor: nil]
end

defmodule CodingAdventures.ChiefOfStaffChannelStore.AppendRequest do
  @enforce_keys [:message_id, :timestamp_ns, :originator_id, :key_epoch, :content_type]
  defstruct @enforce_keys
end

defmodule CodingAdventures.ChiefOfStaffChannelStore.MessagePage do
  @enforce_keys [:messages]
  defstruct [:messages, next_start: nil]
end

defmodule CodingAdventures.ChiefOfStaffChannelStore.OpaqueKeyGrant do
  @enforce_keys [:channel_id, :key_epoch, :receiver_id, :body]
  defstruct @enforce_keys
end

defmodule CodingAdventures.ChiefOfStaffChannelStore.MemoryChannelStorage do
  @moduledoc "Deterministic atomic in-memory D18P backend."

  alias CodingAdventures.ChiefOfStaffChannelStore.{
    StorageConflictError,
    StoragePage,
    StoragePut,
    StorageRecord
  }

  @enforce_keys [:pid]
  defstruct @enforce_keys

  def new! do
    {:ok, pid} = Agent.start_link(fn -> %{records: %{}, revision: 0} end)
    %__MODULE__{pid: pid}
  end

  def initialize(%__MODULE__{}), do: :ok

  def get(%__MODULE__{pid: pid}, namespace, key) do
    Agent.get(pid, fn state -> Map.get(state.records, {namespace, key}) end)
  end

  def put(%__MODULE__{pid: pid}, %StoragePut{} = value) do
    outcome =
      Agent.get_and_update(pid, fn state ->
        current = Map.get(state.records, {value.namespace, value.key})

        cond do
          value.if_absent == not is_nil(value.if_revision) ->
            {{:error, :condition}, state}

          value.if_absent and not is_nil(current) ->
            {{:error, :conflict}, state}

          not value.if_absent and
              (is_nil(current) or current.revision != value.if_revision) ->
            {{:error, :conflict}, state}

          true ->
            revision = state.revision + 1

            record = %StorageRecord{
              namespace: value.namespace,
              key: value.key,
              content_type: value.content_type,
              body: :binary.copy(value.body),
              revision: "r#{revision}"
            }

            next = %{
              records: Map.put(state.records, {value.namespace, value.key}, record),
              revision: revision
            }

            {{:ok, record}, next}
        end
      end)

    case outcome do
      {:ok, record} -> record
      {:error, :condition} -> raise ArgumentError, "exactly one storage condition is required"
      {:error, :conflict} -> raise StorageConflictError
    end
  end

  def list(%__MODULE__{pid: pid}, namespace, options) do
    prefix = Keyword.fetch!(options, :prefix)
    recursive = Keyword.fetch!(options, :recursive)
    page_size = Keyword.fetch!(options, :page_size)
    cursor = Keyword.get(options, :cursor)

    unless recursive and is_integer(page_size) and page_size > 0,
      do: raise(ArgumentError, "invalid backend list options")

    Agent.get(pid, fn state ->
      records =
        state.records
        |> Map.values()
        |> Enum.filter(fn record ->
          record.namespace == namespace and String.starts_with?(record.key, prefix) and
            (is_nil(cursor) or record.key > cursor)
        end)
        |> Enum.sort_by(& &1.key)

      selected = Enum.take(records, page_size)

      %StoragePage{
        records: selected,
        next_cursor: if(length(records) > length(selected), do: List.last(selected).key)
      }
    end)
  end

  def corrupt(%__MODULE__{pid: pid}, %StorageRecord{} = record) do
    Agent.update(pid, fn state ->
      %{state | records: Map.put(state.records, {record.namespace, record.key}, record)}
    end)
  end
end

defmodule CodingAdventures.ChiefOfStaffChannelStore.Backend do
  @moduledoc false

  alias CodingAdventures.ChiefOfStaffChannelStore.{ProfileError, StorageConflictError}
  alias CodingAdventures.ChiefOfStaffChannelStore, as: Profile

  def initialize!(backend), do: invoke!(backend, :initialize, [])
  def get!(backend, namespace, key), do: invoke!(backend, :get, [namespace, key])
  def put!(backend, value), do: invoke!(backend, :put, [value])
  def list!(backend, namespace, options), do: invoke!(backend, :list, [namespace, options])

  defp invoke!(backend, function, arguments) do
    apply(backend.__struct__, function, [backend | arguments])
  rescue
    error in [ProfileError, StorageConflictError] -> raise error
    _ -> Profile.fail!("storage_error")
  catch
    _, _ -> Profile.fail!("storage_error")
  end
end

defmodule CodingAdventures.ChiefOfStaffChannelStore.ChannelStore do
  @moduledoc "CAS-backed view of one encrypted durable channel."

  alias CodingAdventures.ChiefOfStaffChannelCrypto, as: Crypto
  alias CodingAdventures.ChiefOfStaffChannelCrypto.{MessageFields, MessageProfileError}
  alias CodingAdventures.Sha256
  alias CodingAdventures.ChiefOfStaffChannelStore, as: Profile

  alias CodingAdventures.ChiefOfStaffChannelStore.{
    AppendRequest,
    Backend,
    ChannelState,
    MessageHeader,
    MessagePage,
    OpaqueKeyGrant,
    ProfileError,
    StorageConflictError,
    StoragePut
  }

  @enforce_keys [:backend, :channel_id]
  defstruct @enforce_keys

  def new!(backend, channel_id) do
    Profile.validate_uuid_v7!(channel_id, "corrupt_record")
    %__MODULE__{backend: backend, channel_id: channel_id}
  end

  def initialize!(%__MODULE__{} = store) do
    Backend.initialize!(store.backend)

    case state_record(store) do
      nil ->
        body = Profile.state_serialize(%ChannelState{next_sequence: 0})

        try do
          store.backend
          |> Backend.put!(
            put_input(Profile.state_key(store.channel_id), Profile.state_content_type(), body,
              if_absent: true
            )
          )
          |> decode_state(store.channel_id)
        rescue
          _ in StorageConflictError -> state!(store)
        end

      record ->
        decode_state(record, store.channel_id)
    end
  end

  def state!(%__MODULE__{} = store) do
    case state_record(store) do
      nil -> Profile.fail!("not_initialized")
      record -> decode_state(record, store.channel_id)
    end
  end

  def reserve_append!(%__MODULE__{} = store, %AppendRequest{} = request, plaintext) do
    Profile.validate_uuid_v7!(request.message_id)

    crypto_call!(fn ->
      fields =
        MessageFields.new!(
          request.message_id,
          request.timestamp_ns,
          request.originator_id,
          store.channel_id,
          0,
          request.key_epoch,
          request.content_type
        )

      Crypto.validate_message_fields(fields)
    end)

    reserve_attempt!(store, request, plaintext, Profile.max_store_cas_attempts())
  end

  def commit_reserved!(
        %__MODULE__{} = store,
        %MessageHeader{} = header,
        plaintext,
        channel_master_key,
        signing_secret_key
      ) do
    if header.channel_id != store.channel_id, do: Profile.fail!("pending_header_mismatch")
    current = state!(store)

    if is_nil(current.pending_header) do
      key = Profile.message_key(store.channel_id, header.sequence)

      case Backend.get!(store.backend, Profile.storage_namespace(), key) do
        nil ->
          Profile.fail!("no_pending_append")

        record ->
          require_content_type!(record, Profile.message_content_type())
          stored = decode_message!(record.body)
          unless message_matches_header?(stored, header), do: Profile.fail!("conflicting_record")
          expected = create_message!(header, plaintext, signing_secret_key, channel_master_key)

          unless Crypto.message_serialize(expected) == record.body,
            do: Profile.fail!("conflicting_record")

          stored
      end
    else
      if current.pending_header != header, do: Profile.fail!("pending_header_mismatch")
      message = create_message!(header, plaintext, signing_secret_key, channel_master_key)

      put_idempotent!(
        store,
        Profile.message_key(store.channel_id, header.sequence),
        Profile.message_content_type(),
        Crypto.message_serialize(message)
      )

      clear_pending!(store, header, Profile.max_store_cas_attempts())
      message
    end
  end

  def append!(store, request, plaintext, channel_master_key, signing_secret_key) do
    header = reserve_append!(store, request, plaintext)
    commit_reserved!(store, header, plaintext, channel_master_key, signing_secret_key)
  end

  def abandon_pending!(%__MODULE__{} = store) do
    abandon_attempt!(store, Profile.max_store_cas_attempts())
  end

  def read_messages!(%__MODULE__{} = store, start, page_size) do
    Profile.require_u64!(start, "corrupt_record")

    unless is_integer(page_size) and page_size > 0,
      do: Profile.fail!("invalid_page_size")

    cursor = if start > 0, do: Profile.message_key(store.channel_id, start - 1)

    page =
      Backend.list!(store.backend, Profile.storage_namespace(),
        prefix: Profile.message_prefix(store.channel_id),
        recursive: true,
        page_size: page_size,
        cursor: cursor
      )

    messages =
      Enum.reduce(page.records, [], fn record, reversed ->
        require_content_type!(record, Profile.message_content_type())
        message = decode_message!(record.body)
        previous = List.first(reversed)

        if message.channel_id != store.channel_id or message.sequence < start or
             record.key != Profile.message_key(store.channel_id, message.sequence) or
             (not is_nil(previous) and previous.sequence >= message.sequence),
           do: Profile.fail!("corrupt_record")

        [message | reversed]
      end)
      |> Enum.reverse()

    next_start =
      if is_nil(page.next_cursor) do
        nil
      else
        last = List.last(messages)
        if is_nil(last) or last.sequence == Profile.max_u64(), do: Profile.fail!("corrupt_record")
        last.sequence + 1
      end

    %MessagePage{messages: messages, next_start: next_start}
  end

  def read_for_receiver!(store, receiver_id, page_size) do
    read_messages!(store, receiver_cursor!(store, receiver_id), page_size)
  end

  def receiver_cursor!(%__MODULE__{} = store, receiver_id) do
    Profile.validate_agent_id!(receiver_id, "invalid_receiver_id")
    key = Profile.ack_key(store.channel_id, receiver_id)

    case Backend.get!(store.backend, Profile.storage_namespace(), key) do
      nil ->
        0

      record ->
        require_content_type!(record, Profile.ack_content_type())
        Profile.cursor_deserialize(record.body)
    end
  end

  def acknowledge!(%__MODULE__{} = store, receiver_id, acknowledged) do
    Profile.validate_agent_id!(receiver_id, "invalid_receiver_id")
    Profile.require_u64!(acknowledged, "acknowledgement_ahead")
    state = state!(store)
    if acknowledged >= state.next_sequence, do: Profile.fail!("acknowledgement_ahead")

    if not is_nil(state.pending_header) and acknowledged >= state.pending_header.sequence,
      do: Profile.fail!("acknowledgement_pending")

    if acknowledged == Profile.max_u64(), do: Profile.fail!("sequence_exhausted")
    acknowledge_attempt!(store, receiver_id, acknowledged + 1, Profile.max_store_cas_attempts())
  end

  def save_key_grant!(%__MODULE__{} = store, %OpaqueKeyGrant{} = grant) do
    if grant.channel_id != store.channel_id, do: Profile.fail!("corrupt_record")
    Profile.validate_agent_id!(grant.receiver_id, "invalid_receiver_id")

    put_idempotent!(
      store,
      Profile.grant_key(store.channel_id, grant.key_epoch, grant.receiver_id),
      Profile.grant_content_type(),
      grant.body
    )

    :ok
  end

  def key_grant!(%__MODULE__{} = store, key_epoch, receiver_id) do
    Profile.validate_agent_id!(receiver_id, "invalid_receiver_id")
    key = Profile.grant_key(store.channel_id, key_epoch, receiver_id)

    case Backend.get!(store.backend, Profile.storage_namespace(), key) do
      nil ->
        nil

      record ->
        require_content_type!(record, Profile.grant_content_type())
        record.body
    end
  end

  defp reserve_attempt!(_store, _request, _plaintext, 0), do: Profile.fail!("concurrent_update")

  defp reserve_attempt!(store, request, plaintext, attempts) do
    record = state_record(store)
    if is_nil(record), do: Profile.fail!("not_initialized")
    current = decode_state(record, store.channel_id)
    unless is_nil(current.pending_header), do: Profile.fail!("pending_append")
    if current.next_sequence == Profile.max_u64(), do: Profile.fail!("sequence_exhausted")

    header =
      Profile.new_header!(%{
        message_id: request.message_id,
        timestamp_ns: request.timestamp_ns,
        originator_id: request.originator_id,
        channel_id: store.channel_id,
        sequence: current.next_sequence,
        key_epoch: request.key_epoch,
        content_type: request.content_type,
        plaintext_hash: Sha256.sha256(plaintext)
      })

    body =
      Profile.state_serialize(%ChannelState{
        next_sequence: current.next_sequence + 1,
        pending_header: header
      })

    try do
      Backend.put!(
        store.backend,
        put_input(Profile.state_key(store.channel_id), Profile.state_content_type(), body,
          if_revision: record.revision
        )
      )

      header
    rescue
      _ in StorageConflictError -> reserve_attempt!(store, request, plaintext, attempts - 1)
    end
  end

  defp abandon_attempt!(_store, 0), do: Profile.fail!("concurrent_update")

  defp abandon_attempt!(store, attempts) do
    record = state_record(store)
    if is_nil(record), do: Profile.fail!("not_initialized")
    current = decode_state(record, store.channel_id)

    if is_nil(current.pending_header) do
      nil
    else
      try do
        Backend.put!(
          store.backend,
          put_input(
            Profile.state_key(store.channel_id),
            Profile.state_content_type(),
            Profile.state_serialize(%ChannelState{next_sequence: current.next_sequence}),
            if_revision: record.revision
          )
        )

        current.pending_header
      rescue
        _ in StorageConflictError -> abandon_attempt!(store, attempts - 1)
      end
    end
  end

  defp acknowledge_attempt!(_store, _receiver_id, _desired, 0),
    do: Profile.fail!("concurrent_update")

  defp acknowledge_attempt!(store, receiver_id, desired, attempts) do
    key = Profile.ack_key(store.channel_id, receiver_id)
    record = Backend.get!(store.backend, Profile.storage_namespace(), key)

    if is_nil(record) do
      try do
        Backend.put!(
          store.backend,
          put_input(key, Profile.ack_content_type(), Profile.cursor_serialize(desired),
            if_absent: true
          )
        )

        desired
      rescue
        _ in StorageConflictError ->
          acknowledge_attempt!(store, receiver_id, desired, attempts - 1)
      end
    else
      require_content_type!(record, Profile.ack_content_type())
      current = Profile.cursor_deserialize(record.body)

      cond do
        desired < current ->
          Profile.fail!("acknowledgement_regression")

        desired == current ->
          current

        true ->
          try do
            Backend.put!(
              store.backend,
              put_input(key, Profile.ack_content_type(), Profile.cursor_serialize(desired),
                if_revision: record.revision
              )
            )

            desired
          rescue
            _ in StorageConflictError ->
              acknowledge_attempt!(store, receiver_id, desired, attempts - 1)
          end
      end
    end
  end

  defp put_idempotent!(store, key, content_type, body) do
    try do
      Backend.put!(store.backend, put_input(key, content_type, body, if_absent: true))
    rescue
      _ in StorageConflictError ->
        current = Backend.get!(store.backend, Profile.storage_namespace(), key)

        if is_nil(current) or current.content_type != content_type or current.body != body,
          do: Profile.fail!("conflicting_record")
    end
  end

  defp clear_pending!(_store, _header, 0), do: Profile.fail!("concurrent_update")

  defp clear_pending!(store, header, attempts) do
    record = state_record(store)
    if is_nil(record), do: Profile.fail!("not_initialized")
    current = decode_state(record, store.channel_id)

    cond do
      is_nil(current.pending_header) ->
        :ok

      current.pending_header != header ->
        Profile.fail!("pending_header_mismatch")

      true ->
        try do
          Backend.put!(
            store.backend,
            put_input(
              Profile.state_key(store.channel_id),
              Profile.state_content_type(),
              Profile.state_serialize(%ChannelState{next_sequence: current.next_sequence}),
              if_revision: record.revision
            )
          )

          :ok
        rescue
          _ in StorageConflictError -> clear_pending!(store, header, attempts - 1)
        end
    end
  end

  defp state_record(store) do
    Backend.get!(store.backend, Profile.storage_namespace(), Profile.state_key(store.channel_id))
  end

  defp decode_state(record, channel_id) do
    require_content_type!(record, Profile.state_content_type())
    Profile.state_deserialize(record.body, channel_id)
  end

  defp create_message!(header, plaintext, signing_secret_key, channel_master_key) do
    if Sha256.sha256(plaintext) != header.plaintext_hash, do: Profile.fail!("crypto_error")

    crypto_call!(fn ->
      fields =
        MessageFields.new!(
          header.message_id,
          header.timestamp_ns,
          header.originator_id,
          header.channel_id,
          header.sequence,
          header.key_epoch,
          header.content_type
        )

      Crypto.message_create(fields, plaintext, signing_secret_key, channel_master_key)
    end)
  end

  defp decode_message!(data) do
    Crypto.message_deserialize(data)
  rescue
    _ in MessageProfileError -> Profile.fail!("wire_error")
  end

  defp message_matches_header?(message, header) do
    message.message_id == header.message_id and message.timestamp_ns == header.timestamp_ns and
      message.originator_id == header.originator_id and message.channel_id == header.channel_id and
      message.sequence == header.sequence and message.key_epoch == header.key_epoch and
      message.content_type == header.content_type and
      message.plaintext_hash == header.plaintext_hash
  end

  defp require_content_type!(record, expected) do
    if record.content_type != expected, do: Profile.fail!("corrupt_record")
  end

  defp put_input(key, content_type, body, options) do
    %StoragePut{
      namespace: Profile.storage_namespace(),
      key: key,
      content_type: content_type,
      body: body,
      if_absent: Keyword.get(options, :if_absent, false),
      if_revision: Keyword.get(options, :if_revision)
    }
  end

  defp crypto_call!(operation) do
    operation.()
  rescue
    _ in ProfileError -> Profile.fail!("crypto_error")
    _ -> Profile.fail!("crypto_error")
  catch
    _, _ -> Profile.fail!("crypto_error")
  end
end
