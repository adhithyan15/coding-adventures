defmodule CodingAdventures.ChiefOfStaffChannelEpochActivation.InvariantThreeTest do
  @moduledoc """
  Invariant 3: "all grants before visibility".

  Writing a grant successfully is not the same as being able to read it back.
  The record a put echoes sits on the same trust boundary as the write, so
  against a write-behind or eventually-consistent backend an echoed success
  proves nothing. Activation must re-READ every grant and byte-compare before
  advancing the epoch — otherwise it can make E+1 current while a receiver's
  grant is unretrievable, locking that receiver out of a channel it was
  authorized for.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.ChiefOfStaffChannelCrypto.KeyGrantProfile, as: Grants
  alias CodingAdventures.ChiefOfStaffChannelStore, as: Profile
  alias CodingAdventures.ChiefOfStaffChannelStore.MemoryChannelStorage
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation, as: Epoch
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.ActivationError
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.Custody.InMemory
  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.Store, as: EpochStore

  @channel_id Base.decode16!("018f47a09b6c7def923456789abcdef0", case: :lower)
  @current_cmk :binary.copy(<<0x22>>, 32)
  @next_cmk :binary.copy(<<0x33>>, 32)

  defmodule GrantFaultBackend do
    @moduledoc """
    Accepts writes normally; diverges only on reads of one grant key.

    Corrupting an already-written record would NOT test this. Activation replays
    through put_immutable first, whose if-absent put conflicts, re-gets, and
    returns before control ever reaches the phase-6 loop — so such a test passes
    even with the entire invariant-3 check deleted. That is exactly how the Go
    port's first attempt at this test went wrong. Skipping one read aims the
    fault at phase 6.
    """
    use Agent

    alias CodingAdventures.ChiefOfStaffChannelStore.MemoryChannelStorage

    @enforce_keys [:inner, :grant_key, :pid]
    defstruct @enforce_keys

    def new!(inner, grant_key) do
      {:ok, pid} = Agent.start_link(fn -> %{fault: :healthy, skip_reads: 1} end)
      %__MODULE__{inner: inner, grant_key: grant_key, pid: pid}
    end

    def arm(%__MODULE__{pid: pid}, fault),
      do: Agent.update(pid, &Map.put(&1, :fault, fault))

    def reads(%__MODULE__{pid: pid}), do: Agent.get(pid, &Map.get(&1, :reads, 0))

    def initialize(%__MODULE__{inner: inner}), do: MemoryChannelStorage.initialize(inner)

    def get(%__MODULE__{} = backend, namespace, key) do
      record = MemoryChannelStorage.get(backend.inner, namespace, key)

      if is_nil(record) or key != backend.grant_key do
        record
      else
        Agent.get_and_update(backend.pid, fn state ->
          counted = Map.update(state, :reads, 1, &(&1 + 1))

          cond do
            counted.skip_reads > 0 ->
              {record, %{counted | skip_reads: counted.skip_reads - 1}}

            counted.fault == :vanishes ->
              {nil, counted}

            counted.fault == :mutated_body ->
              {%{record | body: record.body <> <<0>>}, counted}

            counted.fault == :mutated_content_type ->
              {%{record | content_type: "application/vnd.wrong"}, counted}

            true ->
              {record, counted}
          end
        end)
      end
    end

    def put(%__MODULE__{inner: inner}, value), do: MemoryChannelStorage.put(inner, value)

    def list(%__MODULE__{inner: inner}, namespace, options),
      do: MemoryChannelStorage.list(inner, namespace, options)
  end

  setup do
    signer = Grants.originator_signing_key_from_seed(:binary.copy(<<0x11>>, 32))
    receiver_a_key = Grants.receiver_key_pair_from_private_key(:binary.copy(<<0x41>>, 32))
    receiver_b_key = Grants.receiver_key_pair_from_private_key(:binary.copy(<<0x42>>, 32))
    receiver_a = Profile.new_receiver!("receiver-a", Grants.receiver_public_key(receiver_a_key))
    receiver_b = Profile.new_receiver!("receiver-b", Grants.receiver_public_key(receiver_b_key))

    definition =
      Profile.new_definition!(
        @channel_id,
        Profile.new_originator!("originator", Grants.originator_public_key(signer)),
        [receiver_a, receiver_b],
        1_725_000_000_000_000_000,
        0
      )

    %{signer: signer, receiver_b: receiver_b, definition: definition}
  end

  defp rotation(context) do
    Grants.plan_rotation(
      "originator",
      @channel_id,
      0,
      Grants.channel_master_key_from_bytes(@next_cmk),
      [
        Grants.rotation_receiver_with_material(
          context.receiver_b.agent_id,
          context.receiver_b.public_key,
          :binary.copy(<<0x51>>, 32),
          :binary.copy(<<0x61>>, 24)
        )
      ],
      context.signer
    )
  end

  defp prepared_channel(context) do
    backend = MemoryChannelStorage.new!()
    custody = InMemory.new!()
    store = EpochStore.open_for_testing(backend, custody, @channel_id)

    EpochStore.create_epoch_channel(
      store,
      context.definition,
      Grants.channel_master_key_from_bytes(@current_cmk)
    )

    EpochStore.prepare_rotation(store, context.definition, [context.receiver_b], rotation(context))
    {backend, custody}
  end

  defp grant_key(context),
    do: Profile.grant_key(@channel_id, 1, context.receiver_b.agent_id)

  test "activation refuses when a grant is not retrievable", context do
    Enum.each([:vanishes, :mutated_body, :mutated_content_type], fn fault ->
      {backend, custody} = prepared_channel(context)
      faulty = GrantFaultBackend.new!(backend, grant_key(context))
      store = EpochStore.open_for_testing(faulty, custody, @channel_id)
      # Arm the fault only AFTER prepare has written the grant cleanly, so the
      # write genuinely succeeds and only the read-back diverges.
      GrantFaultBackend.arm(faulty, fault)

      error =
        assert_raise ActivationError, fn ->
          EpochStore.activate_prepared_epoch(store, context.definition, 1)
        end

      assert error.code == "corrupt_record", "#{fault}"

      # The epoch must not have advanced.
      GrantFaultBackend.arm(faulty, :healthy)
      assert EpochStore.state(store).active_epoch == 0, "#{fault}"
    end)
  end

  # Guard against the phase-6 loop being skipped entirely. Without this, a
  # refactor that drops the read-back would leave the test above passing via
  # some earlier check.
  test "the grant read-back is reached at all", context do
    {backend, custody} = prepared_channel(context)
    counting = GrantFaultBackend.new!(backend, grant_key(context))
    store = EpochStore.open_for_testing(counting, custody, @channel_id)

    assert EpochStore.activate_prepared_epoch(store, context.definition, 1) == "activated"

    # One read from put_immutable's conflict path, one from phase 6.
    reads = GrantFaultBackend.reads(counting)
    assert reads >= 2, "expected the grant to be re-read during replay, saw #{reads} read(s)"
  end

  # Tampered custody bundles are rejected, which is why
  # validate_public_preparation recomputes the plan from the grants instead of
  # trusting the stored commitment.
  test "tampered custody bundles are rejected", context do
    {_backend, custody} = prepared_channel(context)
    genuine = InMemory.load_preparation(custody, @channel_id, 1)
    assert genuine != nil

    [
      {%{genuine | plan_bytes: "not a plan"}, "corrupt_record"},
      {%{genuine | base_epoch: 5, new_epoch: 6}, "invalid_plan"},
      {%{genuine | grants: []}, "invalid_plan"},
      {%{genuine | grants: ["not a grant"]}, "crypto_error"}
    ]
    |> Enum.each(fn {bundle, code} ->
      error =
        assert_raise ActivationError, fn ->
          EpochStore.validate_public_preparation(context.definition, bundle)
        end

      assert error.code == code
    end)
  end

  # Proves the D18T layer actually checks signatures rather than trusting
  # whatever custody produced. This is the test that would fail if
  # verify_grant_signature were dropped from the validation path.
  test "a grant signed by another originator is rejected", context do
    impostor = Grants.originator_signing_key_from_seed(:binary.copy(<<0x12>>, 32))

    forged =
      Grants.plan_rotation(
        "originator",
        @channel_id,
        0,
        Grants.channel_master_key_from_bytes(@next_cmk),
        [
          Grants.rotation_receiver_with_material(
            context.receiver_b.agent_id,
            context.receiver_b.public_key,
            :binary.copy(<<0x57>>, 32),
            :binary.copy(<<0x67>>, 24)
          )
        ],
        impostor
      )

    error =
      assert_raise ActivationError, fn ->
        EpochStore.prepare_rotation_candidate(context.definition, 0, [context.receiver_b], forged)
      end

    assert error.code == "crypto_error"
  end

  test "the D18T error roster is closed", _context do
    assert length(Epoch.error_codes()) == 19
    assert Enum.uniq(Epoch.error_codes()) == Epoch.error_codes()
  end

  defmodule RaisingBackend do
    @moduledoc """
    A backend whose reads and writes fail. D18P's own dispatch turns that into a
    ProfileError, which is a *foreign* exception to a D18T caller.
    """
    alias CodingAdventures.ChiefOfStaffChannelStore.MemoryChannelStorage

    @enforce_keys [:inner, :mode]
    defstruct @enforce_keys

    def new!(inner, mode), do: %__MODULE__{inner: inner, mode: mode}

    def initialize(%__MODULE__{inner: inner}), do: MemoryChannelStorage.initialize(inner)

    def get(%__MODULE__{mode: :get} = _backend, _namespace, _key),
      do: raise(RuntimeError, "injected backend failure")

    def get(%__MODULE__{inner: inner}, namespace, key),
      do: MemoryChannelStorage.get(inner, namespace, key)

    def put(%__MODULE__{mode: :put}, _value), do: raise(RuntimeError, "injected backend failure")
    def put(%__MODULE__{inner: inner}, value), do: MemoryChannelStorage.put(inner, value)

    def list(%__MODULE__{inner: inner}, namespace, options),
      do: MemoryChannelStorage.list(inner, namespace, options)
  end

  # Every failure this API raises must be an ActivationError carrying one of the
  # 19 stable codes. Without translation at the backend boundary a caller
  # rescuing ActivationError would silently miss storage failures, because D18P
  # raises its own ProfileError there. Rust and Ruby both translate; this keeps
  # Elixir from being the outlier.
  test "backend failures surface as ActivationError, not a foreign exception", context do
    {backend, custody} = prepared_channel(context)

    reading = EpochStore.open_for_testing(RaisingBackend.new!(backend, :get), custody, @channel_id)
    writing = EpochStore.open_for_testing(RaisingBackend.new!(backend, :put), custody, @channel_id)

    [
      {"state", fn -> EpochStore.state(reading) end},
      {"activation_plan", fn -> EpochStore.activation_plan(reading, 1) end},
      {"activate", fn -> EpochStore.activate_prepared_epoch(writing, context.definition, 1) end}
    ]
    |> Enum.each(fn {name, operation} ->
      error = assert_raise ActivationError, operation
      assert error.code in Epoch.error_codes(), "#{name} produced #{error.code}"
    end)
  end
end
