defmodule CodingAdventures.ChiefOfStaffChannelEpochActivation.ActiveEpochAppendRequest do
  @moduledoc """
  Asks to publish at whatever epoch is currently active. `key_epoch` is
  optional: leave it `nil` to accept the active epoch, or set it to assert an
  expectation that is checked before any encryption.
  """
  @enforce_keys [:message_id, :timestamp_ns, :originator_id, :content_type]
  defstruct [:message_id, :timestamp_ns, :originator_id, :content_type, key_epoch: nil]
end

defmodule CodingAdventures.ChiefOfStaffChannelEpochActivation.EpochReservation do
  @moduledoc "Durable D18H reservation plus the redacted handle it was bound to."
  @enforce_keys [:header, :key_handle]
  defstruct @enforce_keys
end

defmodule CodingAdventures.ChiefOfStaffChannelEpochActivation.Store do
  @moduledoc """
  D18T coordinator over injected public storage and secret custody.
  """

  alias CodingAdventures.ChiefOfStaffChannelCrypto.KeyGrantProfile, as: Grants
  alias CodingAdventures.Sha256

  alias CodingAdventures.ChiefOfStaffChannelStore, as: Profile

  alias CodingAdventures.ChiefOfStaffChannelStore.{
    Backend,
    ChannelDefinitionStore,
    ProfileError,
    StorageConflictError,
    StoragePut
  }

  alias CodingAdventures.ChiefOfStaffChannelEpochActivation, as: Epoch
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.ActiveEpochAppendRequest
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.Custody
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.EpochReservation
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.PreparedEpoch

  @enforce_keys [:backend, :custody, :channel_id]
  defstruct @enforce_keys

  @doc """
  Open a production coordinator. Refuses custody that reports itself
  non-durable, so a test double cannot be wired into a real channel.
  """
  def open(backend, custody, channel_id) do
    unless Custody.durable?(custody), do: Epoch.fail!("custody_error")
    initialize!(backend)
    %__MODULE__{backend: backend, custody: custody, channel_id: channel_id}
  end

  @doc "Open a coordinator that accepts non-durable custody, for tests only."
  def open_for_testing(backend, custody, channel_id) do
    initialize!(backend)
    %__MODULE__{backend: backend, custody: custody, channel_id: channel_id}
  end

  @doc """
  Create a D18T-aware channel, custody before any D18S state.

  The definition is settled *before* the custody import. Custody slots are keyed
  by `{channel_id, epoch}` and the first writer wins permanently, so importing
  first would let a caller presenting a mismatched definition claim an unclaimed
  slot and then fail — leaving the legitimate import to hit
  `conflicting_active_key` forever. Fail closed, but permanently wedged. D18T
  only requires custody before *state*.
  """
  def create_epoch_channel(%__MODULE__{} = store, definition, initial_cmk) do
    unless definition.channel_id == store.channel_id and definition.lifecycle == "active" do
      Epoch.fail!("invalid_plan")
    end

    definitions = ChannelDefinitionStore.new(store.backend)

    try do
      case ChannelDefinitionStore.load!(definitions, store.channel_id) do
        nil -> ChannelDefinitionStore.create!(definitions, definition)
        %{lifecycle: "destroyed"} -> Epoch.fail!("channel_destroyed")
        existing when existing != definition -> Epoch.fail!("invalid_plan")
        _ -> :ok
      end
    rescue
      error in [Epoch.ActivationError] -> raise error
      error in [ProfileError] -> raise translate_store_error(error)
    end

    import_initial_key(store, definition.key_epoch, initial_cmk)
    migrate_epoch_state(store, definition)
  end

  @doc """
  Bring a channel to D18S version 2, whether new or created under D18P version 1.

  Publishing version 2 is the rolling-upgrade boundary: a version 1 process
  rejects the record rather than misreading it, so operators must deploy
  D18T-aware readers and writers before migrating. Nothing here ever clears a
  pending publish, resets a sequence, or generates key material.
  """
  def migrate_epoch_state(%__MODULE__{} = store, definition, current_cmk \\ nil) do
    require_definition!(store, definition, false)
    migrate_loop(store, definition, current_cmk, Epoch.max_epoch_cas_attempts())
  end

  @doc "Load the canonical D18S version 2 state."
  def state(%__MODULE__{} = store) do
    case state_record(store) do
      nil -> Epoch.fail!("not_initialized")
      record -> decode_v2_state_record(store, record)
    end
  end

  @doc """
  Run the full prepare-and-replay protocol for one candidate.

  Custody comes first, before any public write, because it is the only operation
  that both selects a winner and makes everything needed for replay durable in
  one atomic step. A crash before it leaves no candidate; a crash after it is
  fully recoverable from custody plus the public store.
  """
  def prepare_rotation(%__MODULE__{} = store, definition, target_roster, rotation) do
    require_definition!(store, definition, false)
    current = state(store)
    unless is_nil(current.pending_header), do: Epoch.fail!("pending_append")
    if current.active_epoch == Epoch.max_u64(), do: Epoch.fail!("epoch_exhausted")
    expected = current.active_epoch + 1
    unless rotation.new_epoch == expected, do: Epoch.fail!("unexpected_epoch")

    prepared = prepare_rotation_candidate(definition, current.active_epoch, target_roster, rotation)
    selection = Custody.prepare_if_absent!(store.custody, prepared)
    if selection == Custody.conflict(), do: Epoch.fail!("conflicting_preparation")

    replay_preparation(store, definition, expected)
    if selection == Custody.selected(), do: "prepared", else: "idempotent"
  end

  @doc """
  Replay the durable bundle after a crash. Never generates a CMK, reseals a
  grant, accepts replacement bytes, or picks a different candidate.
  """
  def recover_preparation(%__MODULE__{} = store, definition, new_epoch) do
    require_definition!(store, definition, false)
    active = state(store).active_epoch
    if new_epoch < active, do: Epoch.fail!("decreasing_epoch")

    unless new_epoch == active do
      if active == Epoch.max_u64(), do: Epoch.fail!("epoch_exhausted")
      unless new_epoch == active + 1, do: Epoch.fail!("unexpected_epoch")
    end

    replay_preparation(store, definition, new_epoch)
    "idempotent"
  end

  @doc """
  Commit the epoch transition with a bounded CAS.

  Every precondition is re-checked inside the retry loop, because a CAS conflict
  means somebody else changed the state and any fact read before it may now be
  stale.
  """
  def activate_prepared_epoch(%__MODULE__{} = store, definition, new_epoch) do
    require_definition!(store, definition, false)

    case Custody.load_preparation!(store.custody, store.channel_id, new_epoch) do
      nil -> Epoch.fail!("preparation_missing")
      prepared -> activate_loop(store, definition, new_epoch, prepared, Epoch.max_epoch_cas_attempts())
    end
  end

  @doc """
  Build a D18H reservation bound to the current active epoch and its resolved
  key handle.

  This is the publication half of the shared CAS. If activation wins the race,
  this loop's put conflicts, reloads, and rebuilds against E+1. If this wins,
  activation observes the pending header and reports `pending_append`.
  Encryption never falls back to an old epoch and never invents a missing key.
  """
  def reserve_publish_using_active_epoch(
        %__MODULE__{} = store,
        definition,
        %ActiveEpochAppendRequest{} = request,
        plaintext
      ) do
    require_definition!(store, definition, false)

    unless request.originator_id == definition.originator.agent_id do
      Epoch.fail!("invalid_plan")
    end

    reserve_loop(store, request, plaintext, Epoch.max_epoch_cas_attempts())
  end

  @doc """
  Clear an in-flight reservation without publishing, releasing the CAS so a
  blocked activation can proceed. The sequence is not rewound — D18P sequences
  are append-only, so the abandoned slot simply stays empty.
  """
  def abandon_pending(%__MODULE__{} = store),
    do: abandon_loop(store, Epoch.max_epoch_cas_attempts())

  @doc "Load the immutable public plan for an epoch, or nil."
  def activation_plan(%__MODULE__{} = store, new_epoch) do
    key = Epoch.activation_plan_record_key(store.channel_id, new_epoch)

    case backend!(fn -> Backend.get!(store.backend, Profile.storage_namespace(), key) end) do
      nil ->
        nil

      record ->
        require_envelope!(record, key, Epoch.activation_plan_content_type())
        plan = Epoch.activation_plan_deserialize(record.body)

        unless plan.channel_id == store.channel_id and plan.new_epoch == new_epoch do
          Epoch.fail!("corrupt_record")
        end

        plan
    end
  end

  @doc """
  Wipe custody for a destroyed channel while leaving every public plan, grant,
  and message exactly where it is. D18T revocation is prospective: it stops
  future access, it does not rewrite history.
  """
  def apply_destruction(%__MODULE__{} = store, definition) do
    require_definition!(store, definition, true)
    Custody.destroy_channel!(store.custody, store.channel_id)
    :ok
  end

  @doc """
  Build one pure custody candidate from a trusted D18Q plan.

  Two orderings matter here and they are different. D18Q grants are ordered by
  *raw* receiver ID, because that is the order D18Q produces and the order the
  grants must be replayed in. The public D18T plan entries are sorted by
  receiver ID *hash*, because the plan must not reveal the raw roster. The two
  orders are unrelated, so entries are derived from the D18Q order and
  `new_activation_plan!/4` re-sorts for the wire.
  """
  def prepare_rotation_candidate(definition, base_epoch, target_roster, rotation) do
    grants = rotation.grants
    roster = Enum.to_list(target_roster)
    count = length(roster)

    unless count >= 1 and count <= Epoch.max_plan_receivers() and count == length(grants) do
      Epoch.fail!("invalid_plan")
    end

    ordered = Enum.sort_by(roster, & &1.agent_id)

    if length(Enum.uniq_by(ordered, & &1.agent_id)) != count, do: Epoch.fail!("invalid_plan")

    ordered
    |> Enum.zip(grants)
    |> Enum.each(fn {receiver, grant} ->
      # The epoch check lives here, not in verify_grant_signature: D18Q's
      # signature covers the epoch but the verifier deliberately takes no
      # expected epoch, so a validly signed grant for the wrong epoch would
      # otherwise pass. D18T step 5 owns this comparison.
      unless receiver.agent_id == grant.receiver_id and grant.key_epoch == rotation.new_epoch do
        Epoch.fail!("invalid_plan")
      end

      verify_grant_public!(definition, grant, receiver.agent_id)
    end)

    if base_epoch == Epoch.max_u64(), do: Epoch.fail!("epoch_exhausted")
    unless rotation.new_epoch == base_epoch + 1, do: Epoch.fail!("unexpected_epoch")

    grant_bytes = Enum.map(grants, &serialize_grant!/1)

    entries =
      grants
      |> Enum.zip(grant_bytes)
      |> Enum.map(fn {grant, data} ->
        Epoch.new_plan_entry!(Sha256.sha256(grant.receiver_id), Sha256.sha256(data))
      end)

    plan =
      Epoch.new_activation_plan!(
        definition.channel_id,
        base_epoch,
        rotation.new_epoch,
        entries
      )

    public =
      Custody.new_public_preparation(
        definition.channel_id,
        base_epoch,
        rotation.new_epoch,
        Epoch.activation_plan_serialize(plan),
        grant_bytes
      )

    %PreparedEpoch{public_preparation: public, cmk: rotation.new_cmk}
  end

  @doc """
  Re-derive the entire plan from the durable grants and require it to equal the
  stored plan bytes.

  This runs on every replay, including recovery after a crash, and is
  deliberately not a shortcut comparison of the plan commitment. Recomputing
  from the grants is what makes a tampered custody bundle detectable.
  """
  def validate_public_preparation(definition, prepared) do
    grant_count = length(prepared.grants)

    unless prepared.channel_id == definition.channel_id, do: Epoch.fail!("invalid_plan")

    # Exhaustion sits between the channel comparison and the successor
    # comparison, exactly where Rust's short-circuiting chain evaluates it. It
    # must precede the successor check because base_epoch + 1 is not a
    # meaningful question once base_epoch is saturated; it must follow the
    # channel check so a bundle that is BOTH foreign and saturated still reports
    # invalid_plan, as it did before and as Rust still does.
    if prepared.base_epoch == Epoch.max_u64(), do: Epoch.fail!("epoch_exhausted")

    unless prepared.new_epoch == prepared.base_epoch + 1 and
             grant_count >= 1 and grant_count <= Epoch.max_plan_receivers() do
      Epoch.fail!("invalid_plan")
    end

    plan =
      try do
        Epoch.activation_plan_deserialize(prepared.plan_bytes)
      rescue
        _ -> Epoch.fail!("corrupt_record")
      end

    unless plan.channel_id == prepared.channel_id and plan.base_epoch == prepared.base_epoch and
             plan.new_epoch == prepared.new_epoch and length(plan.receivers) == grant_count do
      Epoch.fail!("invalid_plan")
    end

    {entries, _} =
      Enum.map_reduce(prepared.grants, nil, fn data, previous ->
        grant = deserialize_grant!(data)

        unless grant.channel_id == prepared.channel_id and
                 grant.key_epoch == prepared.new_epoch and
                 (is_nil(previous) or previous < grant.receiver_id) do
          Epoch.fail!("invalid_plan")
        end

        verify_grant_public!(definition, grant, grant.receiver_id)

        entry = Epoch.new_plan_entry!(Sha256.sha256(grant.receiver_id), Sha256.sha256(data))
        {entry, grant.receiver_id}
      end)

    expected =
      Epoch.new_activation_plan!(
        prepared.channel_id,
        prepared.base_epoch,
        prepared.new_epoch,
        entries
      )

    unless plan == expected, do: Epoch.fail!("invalid_plan")
    plan
  end

  # ---------------------------------------------------------------------------
  # Internal loops and helpers
  # ---------------------------------------------------------------------------

  defp migrate_loop(_store, _definition, _cmk, 0), do: Epoch.fail!("concurrent_update")

  defp migrate_loop(store, definition, current_cmk, remaining) do
    record = state_record(store)

    if not is_nil(record) and record.content_type == Epoch.epoch_state_content_type() do
      decoded = decode_v2_state_record(store, record)
      # An existing version 2 state is an idempotent success only once custody
      # proves its active epoch is still resolvable. Its epoch is never reset
      # from the immutable definition.
      if is_nil(resolve_handle(store, decoded.active_epoch)) do
        Epoch.fail!("active_key_missing")
      end

      decoded
    else
      ensure_initial_key(store, definition.key_epoch, current_cmk)
      next = build_migration_state(store, definition, record)

      try do
        stored =
          backend_put!(
            store.backend,
            public_put(
              Profile.state_key(store.channel_id),
              Epoch.epoch_state_content_type(),
              Epoch.epoch_state_serialize(next),
              if_absent: is_nil(record),
              if_revision: record && record.revision
            )
          )

        decode_v2_state_record(store, stored)
      rescue
        StorageConflictError -> migrate_loop(store, definition, nil, remaining - 1)
      end
    end
  end

  defp activate_loop(_store, _definition, _new_epoch, _prepared, 0),
    do: Epoch.fail!("concurrent_update")

  defp activate_loop(store, definition, new_epoch, prepared, remaining) do
    require_definition!(store, definition, false)
    record = state_record(store)
    if is_nil(record), do: Epoch.fail!("not_initialized")
    current = decode_v2_state_record(store, record)
    active = current.active_epoch

    cond do
      active == new_epoch ->
        validate_and_replay(store, definition, prepared)
        require_handle!(store, new_epoch)
        "idempotent"

      active > new_epoch ->
        Epoch.fail!("decreasing_epoch")

      active == Epoch.max_u64() ->
        Epoch.fail!("epoch_exhausted")

      active + 1 != new_epoch or prepared.base_epoch != active or prepared.new_epoch != new_epoch ->
        Epoch.fail!("unexpected_epoch")

      true ->
        validate_and_replay(store, definition, prepared)
        require_handle!(store, new_epoch)
        # Checked last, immediately before the CAS: a reservation that landed
        # during replay must still block this activation.
        unless is_nil(current.pending_header), do: Epoch.fail!("pending_append")

        updated = Epoch.with_active_epoch!(current, store.channel_id, new_epoch)

        try do
          stored =
            backend_put!(
              store.backend,
              public_put(
                Profile.state_key(store.channel_id),
                Epoch.epoch_state_content_type(),
                Epoch.epoch_state_serialize(updated),
                if_revision: record.revision
              )
            )

          unless decode_v2_state_record(store, stored) == updated do
            Epoch.fail!("corrupt_record")
          end

          "activated"
        rescue
          StorageConflictError ->
            activate_loop(store, definition, new_epoch, prepared, remaining - 1)
        end
    end
  end

  defp reserve_loop(_store, _request, _plaintext, 0), do: Epoch.fail!("concurrent_update")

  defp reserve_loop(store, request, plaintext, remaining) do
    record = state_record(store)
    if is_nil(record), do: Epoch.fail!("not_initialized")
    current = decode_v2_state_record(store, record)

    if not is_nil(request.key_epoch) and request.key_epoch != current.active_epoch do
      Epoch.fail!("unactivated_epoch")
    end

    handle = require_handle!(store, current.active_epoch)
    unless is_nil(current.pending_header), do: Epoch.fail!("pending_append")
    if current.next_sequence == Epoch.max_u64(), do: Epoch.fail!("crypto_error")

    header =
      try do
        Profile.new_header!(%{
          message_id: request.message_id,
          timestamp_ns: request.timestamp_ns,
          originator_id: request.originator_id,
          channel_id: store.channel_id,
          sequence: current.next_sequence,
          key_epoch: current.active_epoch,
          content_type: request.content_type,
          plaintext_hash: Sha256.sha256(plaintext)
        })
      rescue
        _ -> Epoch.fail!("crypto_error")
      end

    updated = Epoch.with_pending!(current, store.channel_id, current.next_sequence + 1, header)

    try do
      backend_put!(
        store.backend,
        public_put(
          Profile.state_key(store.channel_id),
          Epoch.epoch_state_content_type(),
          Epoch.epoch_state_serialize(updated),
          if_revision: record.revision
        )
      )

      %EpochReservation{header: header, key_handle: handle}
    rescue
      StorageConflictError -> reserve_loop(store, request, plaintext, remaining - 1)
    end
  end

  defp abandon_loop(_store, 0), do: Epoch.fail!("concurrent_update")

  defp abandon_loop(store, remaining) do
    record = state_record(store)
    if is_nil(record), do: Epoch.fail!("not_initialized")
    current = decode_v2_state_record(store, record)

    case current.pending_header do
      nil ->
        nil

      pending ->
        updated = Epoch.with_pending!(current, store.channel_id, current.next_sequence)

        try do
          backend_put!(
            store.backend,
            public_put(
              Profile.state_key(store.channel_id),
              Epoch.epoch_state_content_type(),
              Epoch.epoch_state_serialize(updated),
              if_revision: record.revision
            )
          )

          pending
        rescue
          StorageConflictError -> abandon_loop(store, remaining - 1)
        end
    end
  end

  defp build_migration_state(store, definition, nil) do
    Epoch.new_epoch_state!(store.channel_id, definition.key_epoch, 0)
  end

  defp build_migration_state(store, definition, record) do
    require_envelope!(record, Profile.state_key(store.channel_id), Profile.state_content_type())

    prior =
      try do
        Profile.state_deserialize(record.body, store.channel_id)
      rescue
        _ -> Epoch.fail!("corrupt_record")
      end

    if not is_nil(prior.pending_header) and
         prior.pending_header.key_epoch != definition.key_epoch do
      Epoch.fail!("corrupt_record")
    end

    Epoch.new_epoch_state!(
      store.channel_id,
      definition.key_epoch,
      prior.next_sequence,
      prior.pending_header
    )
  end

  defp ensure_initial_key(store, epoch, current_cmk) do
    if is_nil(resolve_handle(store, epoch)) do
      if is_nil(current_cmk), do: Epoch.fail!("active_key_missing")
      import_initial_key(store, epoch, current_cmk)
    end

    :ok
  end

  defp import_initial_key(store, epoch, current_cmk) do
    if is_nil(current_cmk), do: Epoch.fail!("active_key_missing")

    selection =
      Custody.import_active_if_absent!(store.custody, store.channel_id, epoch, current_cmk)

    if selection == Custody.conflict(), do: Epoch.fail!("conflicting_active_key")
    :ok
  end

  defp replay_preparation(store, definition, new_epoch) do
    case Custody.load_preparation!(store.custody, store.channel_id, new_epoch) do
      nil -> Epoch.fail!("preparation_missing")
      prepared -> validate_and_replay(store, definition, prepared)
    end
  end

  # Replay phases 3 through 6: re-validate the durable bundle, write the plan
  # and every grant with create-if-absent, then reload and compare.
  defp validate_and_replay(store, definition, prepared) do
    plan = validate_public_preparation(definition, prepared)

    put_immutable(
      store,
      Epoch.activation_plan_record_key(store.channel_id, plan.new_epoch),
      Epoch.activation_plan_content_type(),
      prepared.plan_bytes,
      "conflicting_plan"
    )

    Enum.each(prepared.grants, fn data ->
      grant = deserialize_grant!(data)

      put_immutable(
        store,
        Profile.grant_key(store.channel_id, grant.key_epoch, grant.receiver_id),
        Profile.grant_content_type(),
        data,
        "conflicting_grant"
      )
    end)

    stored = activation_plan(store, plan.new_epoch)
    if is_nil(stored) or stored != plan, do: Epoch.fail!("corrupt_record")

    # Phase 6 reloads every grant too. This is invariant 3, "all grants before
    # visibility" — not paranoia about our own writes. The record a put echoes
    # back sits on the same trust boundary as the write itself, so against a
    # write-behind or eventually-consistent backend an echoed success does not
    # prove the grant is retrievable. Activation may only advance the epoch once
    # every receiver's grant can actually be read.
    Enum.each(prepared.grants, fn data ->
      grant = deserialize_grant!(data)
      key = Profile.grant_key(store.channel_id, grant.key_epoch, grant.receiver_id)

      case backend!(fn -> Backend.get!(store.backend, Profile.storage_namespace(), key) end) do
        nil ->
          Epoch.fail!("corrupt_record")

        record ->
          require_envelope!(record, key, Profile.grant_content_type())
          # corrupt_record, not conflicting_grant: put_immutable already reports
          # a genuine slot conflict, so reaching here means the backend returned
          # something other than what it acknowledged writing.
          unless record.body == data, do: Epoch.fail!("corrupt_record")
      end
    end)

    :ok
  end

  defp require_handle!(store, epoch) do
    case resolve_handle(store, epoch) do
      nil -> Epoch.fail!("active_key_missing")
      handle -> handle
    end
  end

  defp resolve_handle(store, epoch),
    do: Custody.resolve_handle!(store.custody, store.channel_id, epoch)

  defp require_definition!(store, expected, require_destroyed) do
    unless expected.channel_id == store.channel_id, do: Epoch.fail!("invalid_plan")

    actual =
      try do
        ChannelDefinitionStore.load!(ChannelDefinitionStore.new(store.backend), store.channel_id)
      rescue
        error in [ProfileError] -> raise translate_store_error(error)
      end

    if is_nil(actual), do: Epoch.fail!("not_initialized")
    unless actual == expected, do: Epoch.fail!("invalid_plan")

    cond do
      require_destroyed and actual.lifecycle != "destroyed" -> Epoch.fail!("invalid_plan")
      not require_destroyed and actual.lifecycle == "destroyed" -> Epoch.fail!("channel_destroyed")
      true -> :ok
    end
  end

  # Every backend call goes through here.
  #
  # D18P's own Backend.get!/put! already map an injected-backend failure to a
  # ProfileError, which is a *foreign* exception as far as a D18T caller is
  # concerned. The stable-error contract says every failure this API raises is
  # an ActivationError carrying one of the 19 codes, so a caller rescuing
  # ActivationError would otherwise miss storage failures entirely. Rust and
  # Ruby both translate at this boundary; this keeps Elixir from being the
  # outlier.
  #
  # StorageConflictError deliberately passes through untouched: the CAS loops
  # rescue it to retry, and turning it into an ActivationError here would break
  # every one of them.
  defp backend!(operation) when is_function(operation, 0) do
    operation.()
  rescue
    error in [Epoch.ActivationError] -> raise error
    error in [StorageConflictError] -> raise error
    error in [ProfileError] -> raise translate_store_error(error)
  end

  # The CAS loops call this instead of Backend.put! directly, so a backend
  # failure becomes an ActivationError while a genuine conflict still reaches
  # the surrounding retry rescue untouched.
  defp backend_put!(backend, value), do: backend!(fn -> Backend.put!(backend, value) end)

  defp state_record(store) do
    backend!(fn ->
      Backend.get!(store.backend, Profile.storage_namespace(), Profile.state_key(store.channel_id))
    end)
  end

  defp decode_v2_state_record(store, record) do
    require_envelope!(
      record,
      Profile.state_key(store.channel_id),
      Epoch.epoch_state_content_type()
    )

    Epoch.epoch_state_deserialize(record.body, store.channel_id)
  end

  defp require_envelope!(record, key, content_type) do
    unless record.namespace == Profile.storage_namespace() and record.key == key and
             record.content_type == content_type do
      Epoch.fail!("corrupt_record")
    end

    :ok
  end

  defp put_immutable(store, key, content_type, body, conflict_code) do
    record =
      backend!(fn ->
        Backend.put!(store.backend, public_put(key, content_type, body, if_absent: true))
      end)
    require_envelope!(record, key, content_type)
    unless record.body == body, do: Epoch.fail!("corrupt_record")
    :ok
  rescue
    StorageConflictError ->
      case backend!(fn -> Backend.get!(store.backend, Profile.storage_namespace(), key) end) do
        nil ->
          Epoch.fail!("corrupt_record")

        existing ->
          require_envelope!(existing, key, content_type)
          unless existing.body == body, do: Epoch.fail!(conflict_code)
          :ok
      end
  end

  defp public_put(key, content_type, body, options) do
    %StoragePut{
      namespace: Profile.storage_namespace(),
      key: key,
      content_type: content_type,
      body: body,
      if_absent: Keyword.get(options, :if_absent, false),
      if_revision: Keyword.get(options, :if_revision)
    }
  end

  defp initialize!(backend) do
    Backend.initialize!(backend)
  rescue
    _ -> Epoch.fail!("storage_error")
  end

  defp serialize_grant!(grant) do
    Grants.grant_serialize(grant)
  rescue
    error in [Epoch.ActivationError] -> raise error
    _ -> Epoch.fail!("crypto_error")
  end

  defp deserialize_grant!(data) do
    Grants.grant_deserialize(data)
  rescue
    error in [Epoch.ActivationError] -> raise error
    _ -> Epoch.fail!("crypto_error")
  end

  defp verify_grant_public!(definition, grant, receiver_id) do
    Grants.verify_grant_signature(
      grant,
      definition.originator.agent_id,
      receiver_id,
      definition.channel_id,
      definition.originator.public_key
    )
  rescue
    error in [Epoch.ActivationError] -> raise error
    _ -> Epoch.fail!("crypto_error")
  end

  # Map D18P codes onto the D18T roster. Anything without a D18T meaning becomes
  # storage_error rather than leaking a foreign code.
  defp translate_store_error(%ProfileError{code: code}) do
    mapped =
      case code do
        "channel_destroyed" -> "channel_destroyed"
        code when code in ["conflicting_definition", "definition_changed"] -> "invalid_plan"
        code when code in ["corrupt_definition", "corrupt_record"] -> "corrupt_record"
        code when code in ["definition_not_found", "not_initialized"] -> "not_initialized"
        _ -> "storage_error"
      end

    %Epoch.ActivationError{code: mapped}
  end
end
