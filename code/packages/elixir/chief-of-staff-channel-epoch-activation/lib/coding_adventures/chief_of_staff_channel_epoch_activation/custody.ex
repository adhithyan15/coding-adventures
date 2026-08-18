defmodule CodingAdventures.ChiefOfStaffChannelEpochActivation.EpochKeyHandle do
  @moduledoc """
  Opaque, redacted reference to one retained epoch key.

  Carries no key bytes and no reversible locator — only the channel and epoch,
  both already public. Resolving a handle to an actual CMK is the sole privilege
  of the originator encryption boundary, via `with_key/3`.
  """
  @derive {Inspect, only: []}
  @enforce_keys [:channel_id, :epoch]
  defstruct @enforce_keys
end

defmodule CodingAdventures.ChiefOfStaffChannelEpochActivation.PublicPreparation do
  @moduledoc """
  Exact secret-free recovery bundle retained beside a prepared CMK. After any
  crash this is enough to replay every public write without regenerating a
  single byte.
  """
  @enforce_keys [:channel_id, :base_epoch, :new_epoch, :plan_bytes, :grants]
  defstruct @enforce_keys
end

defmodule CodingAdventures.ChiefOfStaffChannelEpochActivation.PreparedEpoch do
  @moduledoc """
  One indivisible candidate offered to custody: the public recovery bundle *and*
  the secret CMK, together.

  "Indivisible" is the whole point. Custody must never store the plan without the
  key or the key without the plan — either half alone leaves a channel that
  cannot recover. That is why this is a single struct with a single custody entry
  point rather than two calls a caller could interleave.
  """
  @derive {Inspect, only: []}
  @enforce_keys [:public_preparation, :cmk]
  defstruct @enforce_keys
end

defmodule CodingAdventures.ChiefOfStaffChannelEpochActivation.Custody do
  @moduledoc """
  Injected atomic originator-key custody, plus a deterministic non-durable
  implementation for conformance tests.

  A production implementation MUST survive process and machine restart.
  `durable?/1` is how an implementation declares that honestly; the production
  constructor refuses anything answering `false`, so a test double cannot be
  wired into a real channel by accident.
  """

  alias CodingAdventures.ChiefOfStaffChannelCrypto.KeyGrantProfile, as: Grants

  alias CodingAdventures.ChiefOfStaffChannelEpochActivation, as: Epoch

  alias CodingAdventures.ChiefOfStaffChannelEpochActivation.{
    EpochKeyHandle,
    PreparedEpoch,
    PublicPreparation
  }

  @selected "selected"
  @idempotent "idempotent"
  @conflict "conflict"

  @doc """
  Three-valued result of an atomic custody claim.

  Three values, not two, and the distinction is the heart of D18T. `selected`
  and `idempotent` are both successes but mean different things: the first says
  *you* won the slot, the second says you are retrying something already won
  with byte-identical inputs. `conflict` means somebody else owns it and you
  must not proceed — notably, you may not look at what they stored.
  """
  def selected, do: @selected
  def idempotent, do: @idempotent
  def conflict, do: @conflict

  @doc """
  Constant-time CMK comparison.

  Both operands are secrets, so a content-dependent early exit would leak
  information about the stored key to a caller who controls the candidate.
  Delegates to the repository's audited primitive rather than reimplementing it.
  """
  def same_cmk?(left, right) do
    CodingAdventures.CtCompare.ct_eq_fixed(
      Grants.channel_master_key_bytes(left),
      Grants.channel_master_key_bytes(right)
    )
  end

  # ---------------------------------------------------------------------------
  # In-memory, explicitly non-durable custody
  # ---------------------------------------------------------------------------

  defmodule InMemory do
    @moduledoc """
    Deterministic, explicitly non-durable custody for conformance tests.

    `durable?/1` returns `false`, so `Store.open/3` refuses it and only
    `Store.open_for_testing/3` will accept it.
    """
    use Agent

    alias CodingAdventures.ChiefOfStaffChannelEpochActivation.Custody

    @enforce_keys [:pid]
    defstruct @enforce_keys

    def new! do
      {:ok, pid} = Agent.start_link(fn -> %{keys: %{}, preparations: %{}} end)
      %__MODULE__{pid: pid}
    end

    def durable?(%__MODULE__{}), do: false

    @doc """
    Claim an already-active epoch key. Used only at channel creation and at
    version 1 migration — never to invent a key.
    """
    def import_active_if_absent(%__MODULE__{pid: pid}, channel_id, epoch, cmk) do
      Agent.get_and_update(pid, fn state ->
        slot = {channel_id, epoch}

        case Map.get(state.keys, slot) do
          nil ->
            {Custody.selected(), put_in(state.keys[slot], cmk)}

          current ->
            # Deliberately does not reveal *how* the stored secret differs.
            outcome =
              if Custody.same_cmk?(current, cmk),
                do: Custody.idempotent(),
                else: Custody.conflict()

            {outcome, state}
        end
      end)
    end

    def resolve_handle(%__MODULE__{pid: pid}, channel_id, epoch) do
      Agent.get(pid, fn state ->
        if Map.has_key?(state.keys, {channel_id, epoch}) do
          %CodingAdventures.ChiefOfStaffChannelEpochActivation.EpochKeyHandle{
            channel_id: channel_id,
            epoch: epoch
          }
        end
      end)
    end

    @doc """
    Atomically claim the epoch slot for one complete bundle.

    Both halves are checked before either is written, and a partially present
    slot (key without bundle, or bundle without key) is a conflict rather than
    something to repair — a half-written slot means an invariant already broke,
    and guessing at the missing half is exactly the fallback D18T forbids.
    """
    def prepare_if_absent(%__MODULE__{pid: pid}, prepared) do
      Agent.get_and_update(pid, fn state ->
        public = prepared.public_preparation
        slot = {public.channel_id, public.new_epoch}
        current_public = Map.get(state.preparations, slot)
        current_cmk = Map.get(state.keys, slot)

        cond do
          is_nil(current_public) and is_nil(current_cmk) ->
            updated =
              state
              |> put_in([:preparations, slot], public)
              |> put_in([:keys, slot], prepared.cmk)

            {Custody.selected(), updated}

          is_nil(current_public) or is_nil(current_cmk) ->
            {Custody.conflict(), state}

          current_public != public ->
            {Custody.conflict(), state}

          Custody.same_cmk?(current_cmk, prepared.cmk) ->
            {Custody.idempotent(), state}

          true ->
            {Custody.conflict(), state}
        end
      end)
    end

    def load_preparation(%__MODULE__{pid: pid}, channel_id, new_epoch) do
      Agent.get(pid, fn state -> Map.get(state.preparations, {channel_id, new_epoch}) end)
    end

    @doc "Lend the CMK for exactly one operation."
    def with_key(%__MODULE__{pid: pid}, handle, operation) do
      cmk =
        Agent.get(pid, fn state -> Map.get(state.keys, {handle.channel_id, handle.epoch}) end)

      if is_nil(cmk), do: Epoch.fail!("custody_error")
      operation.(cmk)
    end

    @doc """
    Erase every retained secret for one channel. Public history is untouched —
    that is the store's business, and D18T keeps it append-only.
    """
    def destroy_channel(%__MODULE__{pid: pid}, channel_id) do
      Agent.update(pid, fn state ->
        %{
          state
          | keys: reject_channel(state.keys, channel_id),
            preparations: reject_channel(state.preparations, channel_id)
        }
      end)
    end

    def retained_key_count(%__MODULE__{pid: pid}), do: Agent.get(pid, &map_size(&1.keys))

    defp reject_channel(map, channel_id) do
      Map.reject(map, fn {{channel, _epoch}, _value} -> channel == channel_id end)
    end
  end

  # ---------------------------------------------------------------------------
  # Dispatch, mirroring the D18P Backend pattern
  # ---------------------------------------------------------------------------

  @doc false
  def durable?(custody), do: invoke!(custody, :durable?, [])

  @doc false
  def import_active_if_absent!(custody, channel_id, epoch, cmk),
    do: invoke!(custody, :import_active_if_absent, [channel_id, epoch, cmk])

  @doc false
  def resolve_handle!(custody, channel_id, epoch),
    do: invoke!(custody, :resolve_handle, [channel_id, epoch])

  @doc false
  def prepare_if_absent!(custody, %PreparedEpoch{} = prepared),
    do: invoke!(custody, :prepare_if_absent, [prepared])

  @doc false
  def load_preparation!(custody, channel_id, new_epoch),
    do: invoke!(custody, :load_preparation, [channel_id, new_epoch])

  @doc false
  def with_key!(custody, %EpochKeyHandle{} = handle, operation),
    do: invoke!(custody, :with_key, [handle, operation])

  @doc false
  def destroy_channel!(custody, channel_id), do: invoke!(custody, :destroy_channel, [channel_id])

  @doc false
  def new_public_preparation(channel_id, base_epoch, new_epoch, plan_bytes, grants) do
    %PublicPreparation{
      channel_id: channel_id,
      base_epoch: base_epoch,
      new_epoch: new_epoch,
      plan_bytes: plan_bytes,
      grants: grants
    }
  end

  defp invoke!(custody, function, arguments) do
    apply(custody.__struct__, function, [custody | arguments])
  rescue
    error in [Epoch.ActivationError] -> raise error
    _ -> Epoch.fail!("custody_error")
  catch
    _, _ -> Epoch.fail!("custody_error")
  end
end
