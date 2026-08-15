defmodule CodingAdventures.ChiefOfStaffChannelCrypto.KeyGrantProfile do
  @moduledoc """
  Portable D18Q channel-key grants, receiver epoch state, and rotation plans.

  Cryptographic operations are composed exclusively from repository-owned
  X25519, HKDF-SHA256, XChaCha20-Poly1305, and Ed25519 packages. Secret-bearing
  values redact their contents when inspected. Because BEAM values are
  immutable and garbage-collected, physical secret erasure is reported as
  `not_enforceable`.
  """

  import Bitwise

  alias CodingAdventures.{ChaCha20Poly1305, Ed25519, Hkdf, X25519}

  alias __MODULE__.{
    ChannelMasterKey,
    GrantFields,
    OriginatorSigningKey,
    PortableKeyGrant,
    ProfileError,
    ReceiverEpochKeys,
    ReceiverKeyPair,
    RotationPlan,
    RotationReceiver
  }

  @magic "D18G"
  @wire_version 1
  @grant_context "chief-channel-key-grant-v1"
  @wrap_context "chief-channel-key-wrap-v1"
  @max_identity_bytes 4096
  @max_u64 (1 <<< 64) - 1
  @error_codes ~w(
    invalid_magic unsupported_version truncated_record trailing_bytes
    length_limit_exceeded invalid_field randomness_unavailable
    invalid_key_agreement key_derivation_failed invalid_signature
    unexpected_originator unexpected_receiver unexpected_channel
    authentication_failed invalid_wrapped_key conflicting_grant
    decreasing_epoch epoch_exhausted missing_epoch_key
  )

  defmodule ProfileError do
    @moduledoc "One fail-closed D18Q error with a stable portable code."
    defexception [:code]

    @impl true
    def message(%{code: code}), do: code
  end

  defmodule ChannelMasterKey do
    @moduledoc "A redacted immutable 256-bit channel master key."
    @derive {Inspect, only: [:destroyed]}
    @enforce_keys [:value]
    defstruct [:value, destroyed: false]
  end

  defmodule ReceiverKeyPair do
    @moduledoc "A redacted immutable X25519 receiver key pair."
    @derive {Inspect, only: [:public_key, :destroyed]}
    @enforce_keys [:private_key, :public_key]
    defstruct [:private_key, :public_key, destroyed: false]
  end

  defmodule OriginatorSigningKey do
    @moduledoc "A redacted immutable Ed25519 originator signing key."
    @derive {Inspect, only: [:public_key, :destroyed]}
    @enforce_keys [:secret_key, :public_key]
    defstruct [:secret_key, :public_key, destroyed: false]
  end

  defmodule GrantFields do
    @moduledoc "Validated immutable fields used to seal one D18Q grant."
    @enforce_keys [:originator_id, :receiver_id, :channel_id, :key_epoch]
    defstruct @enforce_keys
  end

  defmodule PortableKeyGrant do
    @moduledoc "One immutable D18G version 1 sealed channel-key grant."
    @enforce_keys [
      :originator_id,
      :receiver_id,
      :channel_id,
      :key_epoch,
      :ephemeral_public_key,
      :wrapping_nonce,
      :wrapped_cmk,
      :originator_signature
    ]
    defstruct @enforce_keys
  end

  defmodule ReceiverEpochKeys do
    @moduledoc "Immutable receiver state retaining successfully installed epochs."
    @derive {Inspect,
             only: [
               :originator_id,
               :receiver_id,
               :channel_id,
               :originator_public_key,
               :latest_grant
             ]}
    @enforce_keys [
      :originator_id,
      :receiver_id,
      :channel_id,
      :receiver_key_pair,
      :originator_public_key
    ]
    defstruct @enforce_keys ++ [epoch_keys: %{}, latest_grant: nil]
  end

  defmodule RotationReceiver do
    @moduledoc "Explicit material for sealing one receiver's rotation grant."
    @derive {Inspect, only: [:receiver_id, :public_key]}
    @enforce_keys [:receiver_id, :public_key, :ephemeral_private_key, :wrapping_nonce]
    defstruct @enforce_keys
  end

  defmodule RotationPlan do
    @moduledoc "A pure, non-durable new-epoch plan and its receiver-sorted grants."
    @derive {Inspect, only: [:new_epoch, :grants]}
    @enforce_keys [:new_epoch, :new_cmk, :grants]
    defstruct @enforce_keys
  end

  @doc "Stable D18Q error-code roster."
  def error_codes, do: @error_codes

  @doc "BEAM values cannot promise physical overwrite of immutable secret copies."
  def secret_erasure_capability, do: "not_enforceable"

  def channel_master_key_from_bytes(value) do
    %ChannelMasterKey{value: fixed_binary!(value, 32)}
  end

  def generate_channel_master_key(source \\ &:crypto.strong_rand_bytes/1) do
    source |> secure_random_bytes(32) |> channel_master_key_from_bytes()
  end

  def channel_master_key_bytes(%ChannelMasterKey{value: value, destroyed: false}), do: value
  def channel_master_key_bytes(_), do: fail!("invalid_field")

  def destroy_channel_master_key(%ChannelMasterKey{value: value}) do
    %ChannelMasterKey{value: :binary.copy(<<0>>, byte_size(value)), destroyed: true}
  end

  def destroy_channel_master_key(_), do: fail!("invalid_field")

  def receiver_key_pair_from_private_key(private_key) do
    private = fixed_binary!(private_key, 32)
    {_private, public} = x25519_keypair(private)
    %ReceiverKeyPair{private_key: private, public_key: public}
  end

  def generate_receiver_key_pair(source \\ &:crypto.strong_rand_bytes/1) do
    source |> secure_random_bytes(32) |> receiver_key_pair_from_private_key()
  end

  def receiver_public_key(%ReceiverKeyPair{public_key: public, destroyed: false}), do: public
  def receiver_public_key(_), do: fail!("invalid_field")

  def destroy_receiver_key_pair(%ReceiverKeyPair{private_key: private, public_key: public}) do
    %ReceiverKeyPair{
      private_key: :binary.copy(<<0>>, byte_size(private)),
      public_key: public,
      destroyed: true
    }
  end

  def destroy_receiver_key_pair(_), do: fail!("invalid_field")

  def originator_signing_key_from_seed(seed) do
    seed = fixed_binary!(seed, 32)

    try do
      {public, secret} = Ed25519.generate_keypair(seed)
      %OriginatorSigningKey{secret_key: secret, public_key: public}
    rescue
      _ -> fail!("invalid_field")
    end
  end

  def generate_originator_signing_key(source \\ &:crypto.strong_rand_bytes/1) do
    source |> secure_random_bytes(32) |> originator_signing_key_from_seed()
  end

  def originator_public_key(%OriginatorSigningKey{public_key: public, destroyed: false}),
    do: public

  def originator_public_key(_), do: fail!("invalid_field")

  def destroy_originator_signing_key(%OriginatorSigningKey{
        secret_key: secret,
        public_key: public
      }) do
    %OriginatorSigningKey{
      secret_key: :binary.copy(<<0>>, byte_size(secret)),
      public_key: public,
      destroyed: true
    }
  end

  def destroy_originator_signing_key(_), do: fail!("invalid_field")

  def grant_fields(originator_id, receiver_id, channel_id, key_epoch) do
    originator = require_identity!(originator_id)
    receiver = require_identity!(receiver_id)
    channel = require_channel_id!(channel_id)

    %GrantFields{
      originator_id: originator,
      receiver_id: receiver,
      channel_id: channel,
      key_epoch: require_u64!(key_epoch)
    }
  end

  def grant_deserialize(data) when is_binary(data) do
    {magic, rest} = take(data, 4)
    if magic != @magic, do: fail!("invalid_magic")
    {version, rest} = take(rest, 1)
    if version != <<@wire_version>>, do: fail!("unsupported_version")
    {originator_id, rest} = read_identity(rest)
    {receiver_id, rest} = read_identity(rest)
    {channel_id, rest} = take(rest, 16)
    {epoch_bytes, rest} = take(rest, 8)
    <<key_epoch::unsigned-big-64>> = epoch_bytes
    {ephemeral_public_key, rest} = take(rest, 32)
    {wrapping_nonce, rest} = take(rest, 24)
    {wrapped_cmk, rest} = take(rest, 48)
    {originator_signature, rest} = take(rest, 64)
    if rest != <<>>, do: fail!("trailing_bytes")

    structural_grant!(%{
      originator_id: originator_id,
      receiver_id: receiver_id,
      channel_id: channel_id,
      key_epoch: key_epoch,
      ephemeral_public_key: ephemeral_public_key,
      wrapping_nonce: wrapping_nonce,
      wrapped_cmk: wrapped_cmk,
      originator_signature: originator_signature
    })
  end

  def grant_deserialize(_), do: fail!("invalid_field")

  def grant_serialize(%PortableKeyGrant{} = grant) do
    validate_grant!(grant)

    <<
      @magic,
      @wire_version,
      byte_size(grant.originator_id)::unsigned-big-32,
      grant.originator_id::binary,
      byte_size(grant.receiver_id)::unsigned-big-32,
      grant.receiver_id::binary,
      grant.channel_id::binary,
      grant.key_epoch::unsigned-big-64,
      grant.ephemeral_public_key::binary,
      grant.wrapping_nonce::binary,
      grant.wrapped_cmk::binary,
      grant.originator_signature::binary
    >>
  end

  def grant_serialize(_), do: fail!("invalid_field")

  def seal_channel_key(
        fields,
        cmk,
        receiver_public_key,
        signing_key,
        source \\ &:crypto.strong_rand_bytes/1
      )

  def seal_channel_key(
        %GrantFields{} = fields,
        %ChannelMasterKey{} = cmk,
        receiver_public_key,
        %OriginatorSigningKey{} = signing_key,
        source
      ) do
    ephemeral_private_key = secure_random_bytes(source, 32)
    wrapping_nonce = secure_random_bytes(source, 24)

    seal_channel_key_with_material(
      fields,
      cmk,
      receiver_public_key,
      signing_key,
      ephemeral_private_key,
      wrapping_nonce
    )
  end

  def seal_channel_key(_, _, _, _, _), do: fail!("invalid_field")

  def seal_channel_key_with_material(
        %GrantFields{} = fields,
        %ChannelMasterKey{} = cmk,
        receiver_public_key,
        %OriginatorSigningKey{} = signing_key,
        ephemeral_private_key,
        wrapping_nonce
      ) do
    validate_fields!(fields)
    cmk_bytes = channel_master_key_bytes(cmk)
    receiver_public = fixed_binary!(receiver_public_key, 32)
    ephemeral_private = fixed_binary!(ephemeral_private_key, 32)
    nonce = fixed_binary!(wrapping_nonce, 24)
    ephemeral_public = x25519_public(ephemeral_private)
    shared_secret = x25519_agree(ephemeral_private, receiver_public)

    wrapping_key =
      derive_key_grant_wrapping_key(
        shared_secret,
        fields.channel_id,
        fields.key_epoch,
        fields.receiver_id
      )

    aad =
      grant_aad_values(
        fields.originator_id,
        fields.receiver_id,
        fields.channel_id,
        fields.key_epoch,
        ephemeral_public
      )

    {ciphertext, tag} =
      ChaCha20Poly1305.xchacha20_poly1305_encrypt(cmk_bytes, wrapping_key, nonce, aad)

    wrapped_cmk = ciphertext <> tag

    signature_input =
      grant_signature_values(
        fields.originator_id,
        fields.receiver_id,
        fields.channel_id,
        fields.key_epoch,
        ephemeral_public,
        nonce,
        wrapped_cmk
      )

    signature = sign(signing_key, signature_input)

    structural_grant!(%{
      originator_id: fields.originator_id,
      receiver_id: fields.receiver_id,
      channel_id: fields.channel_id,
      key_epoch: fields.key_epoch,
      ephemeral_public_key: ephemeral_public,
      wrapping_nonce: nonce,
      wrapped_cmk: wrapped_cmk,
      originator_signature: signature
    })
  rescue
    error in ProfileError -> raise error
    _ -> fail!("authentication_failed")
  end

  def seal_channel_key_with_material(_, _, _, _, _, _), do: fail!("invalid_field")

  def open_channel_key_grant(
        %PortableKeyGrant{} = grant,
        expected_originator_id,
        expected_receiver_id,
        expected_channel_id,
        %ReceiverKeyPair{} = receiver_key_pair,
        originator_public_key
      ) do
    validate_grant!(grant)
    expected_originator = require_binary!(expected_originator_id)
    expected_receiver = require_binary!(expected_receiver_id)
    expected_channel = fixed_binary!(expected_channel_id, 16)
    public_key = fixed_binary!(originator_public_key, 32)

    unless constant_time_equal(grant.originator_id, expected_originator),
      do: fail!("unexpected_originator")

    unless constant_time_equal(grant.receiver_id, expected_receiver),
      do: fail!("unexpected_receiver")

    unless constant_time_equal(grant.channel_id, expected_channel),
      do: fail!("unexpected_channel")

    signature_input = key_grant_signature_input(grant)

    signature_valid =
      try do
        Ed25519.verify(signature_input, grant.originator_signature, public_key)
      rescue
        _ -> false
      end

    unless signature_valid, do: fail!("invalid_signature")

    shared_secret = receiver_agree(receiver_key_pair, grant.ephemeral_public_key)

    wrapping_key =
      derive_key_grant_wrapping_key(
        shared_secret,
        grant.channel_id,
        grant.key_epoch,
        grant.receiver_id
      )

    aad = key_grant_aad(grant)
    <<ciphertext::binary-size(32), tag::binary-size(16)>> = grant.wrapped_cmk

    case ChaCha20Poly1305.xchacha20_poly1305_decrypt(
           ciphertext,
           wrapping_key,
           grant.wrapping_nonce,
           aad,
           tag
         ) do
      {:ok, plaintext} when byte_size(plaintext) == 32 ->
        channel_master_key_from_bytes(plaintext)

      {:ok, _} ->
        fail!("invalid_wrapped_key")

      _ ->
        fail!("authentication_failed")
    end
  rescue
    error in ProfileError -> raise error
    _ -> fail!("authentication_failed")
  end

  def open_channel_key_grant(_, _, _, _, _, _), do: fail!("invalid_field")

  def receiver_epoch_keys(
        originator_id,
        receiver_id,
        channel_id,
        %ReceiverKeyPair{} = receiver_key_pair,
        originator_public_key
      ) do
    %ReceiverEpochKeys{
      originator_id: require_identity!(originator_id),
      receiver_id: require_identity!(receiver_id),
      channel_id: require_channel_id!(channel_id),
      receiver_key_pair: clone_receiver_key_pair(receiver_key_pair),
      originator_public_key: fixed_binary!(originator_public_key, 32)
    }
  end

  def receiver_epoch_keys(_, _, _, _, _), do: fail!("invalid_field")

  def receiver_state_public_key(%ReceiverEpochKeys{receiver_key_pair: pair}),
    do: receiver_public_key(pair)

  def receiver_state_latest_epoch(%ReceiverEpochKeys{latest_grant: nil}), do: nil

  def receiver_state_latest_epoch(%ReceiverEpochKeys{latest_grant: grant}),
    do: grant.key_epoch

  def receiver_state_install(%ReceiverEpochKeys{} = state, %PortableKeyGrant{} = grant) do
    case state.latest_grant do
      nil ->
        install_new_grant(state, grant)

      latest when grant.key_epoch < latest.key_epoch ->
        fail!("decreasing_epoch")

      latest when grant.key_epoch == latest.key_epoch ->
        if grant == latest do
          {"idempotent", state}
        else
          fail!("conflicting_grant")
        end

      _ ->
        install_new_grant(state, grant)
    end
  end

  def receiver_state_install(_, _), do: fail!("invalid_field")

  def receiver_state_key(%ReceiverEpochKeys{} = state, epoch) do
    epoch = require_u64!(epoch)

    case Map.fetch(state.epoch_keys, epoch) do
      {:ok, key} -> clone_channel_master_key(key)
      :error -> fail!("missing_epoch_key")
    end
  end

  def receiver_state_key(_, _), do: fail!("invalid_field")

  def receiver_state_retained_epochs(%ReceiverEpochKeys{} = state) do
    state.epoch_keys |> Map.keys() |> Enum.sort()
  end

  def destroy_receiver_state(%ReceiverEpochKeys{} = state) do
    %{
      state
      | receiver_key_pair: destroy_receiver_key_pair(state.receiver_key_pair),
        epoch_keys: %{},
        latest_grant: nil
    }
  end

  def destroy_receiver_state(_), do: fail!("invalid_field")

  def rotation_receiver_with_material(
        receiver_id,
        public_key,
        ephemeral_private_key,
        wrapping_nonce
      ) do
    %RotationReceiver{
      receiver_id: require_identity!(receiver_id),
      public_key: fixed_binary!(public_key, 32),
      ephemeral_private_key: fixed_binary!(ephemeral_private_key, 32),
      wrapping_nonce: fixed_binary!(wrapping_nonce, 24)
    }
  end

  def generate_rotation_receiver(receiver_id, public_key, source \\ &:crypto.strong_rand_bytes/1) do
    rotation_receiver_with_material(
      receiver_id,
      public_key,
      secure_random_bytes(source, 32),
      secure_random_bytes(source, 24)
    )
  end

  def plan_rotation(
        originator_id,
        channel_id,
        current_epoch,
        %ChannelMasterKey{} = new_cmk,
        receivers,
        %OriginatorSigningKey{} = signing_key
      )
      when is_list(receivers) do
    originator = require_identity!(originator_id)
    channel = require_channel_id!(channel_id)
    current = require_u64!(current_epoch)
    if current == @max_u64, do: fail!("epoch_exhausted")
    if receivers == [], do: fail!("invalid_field")
    unless Enum.all?(receivers, &match?(%RotationReceiver{}, &1)), do: fail!("invalid_field")

    ordered = Enum.sort_by(receivers, & &1.receiver_id)

    if ordered
       |> Enum.chunk_every(2, 1, :discard)
       |> Enum.any?(fn [a, b] -> a.receiver_id == b.receiver_id end) do
      fail!("invalid_field")
    end

    new_epoch = current + 1

    grants =
      Enum.map(ordered, fn receiver ->
        fields = grant_fields(originator, receiver.receiver_id, channel, new_epoch)

        seal_channel_key_with_material(
          fields,
          new_cmk,
          receiver.public_key,
          signing_key,
          receiver.ephemeral_private_key,
          receiver.wrapping_nonce
        )
      end)

    %RotationPlan{
      new_epoch: new_epoch,
      new_cmk: clone_channel_master_key(new_cmk),
      grants: grants
    }
  end

  def plan_rotation(_, _, _, _, _, _), do: fail!("invalid_field")

  def destroy_rotation_plan(%RotationPlan{} = plan) do
    %{plan | new_cmk: destroy_channel_master_key(plan.new_cmk)}
  end

  def destroy_rotation_plan(_), do: fail!("invalid_field")

  def key_grant_hkdf_salt(channel_id, key_epoch) do
    frame([fixed_binary!(channel_id, 16), u64be(key_epoch)])
  end

  def key_grant_hkdf_info(receiver_id) do
    receiver = require_binary!(receiver_id)
    if byte_size(receiver) > @max_identity_bytes, do: fail!("length_limit_exceeded")
    frame([@wrap_context, receiver])
  end

  def key_grant_aad(%PortableKeyGrant{} = grant) do
    grant_aad_values(
      grant.originator_id,
      grant.receiver_id,
      grant.channel_id,
      grant.key_epoch,
      grant.ephemeral_public_key
    )
  end

  def key_grant_aad(_), do: fail!("invalid_field")

  def key_grant_signature_input(%PortableKeyGrant{} = grant) do
    grant_signature_values(
      grant.originator_id,
      grant.receiver_id,
      grant.channel_id,
      grant.key_epoch,
      grant.ephemeral_public_key,
      grant.wrapping_nonce,
      grant.wrapped_cmk
    )
  end

  def key_grant_signature_input(_), do: fail!("invalid_field")

  def key_grant_wrapping_key(shared_secret, channel_id, key_epoch, receiver_id) do
    derive_key_grant_wrapping_key(
      fixed_binary!(shared_secret, 32),
      fixed_binary!(channel_id, 16),
      require_u64!(key_epoch),
      require_binary!(receiver_id)
    )
  end

  defp install_new_grant(state, grant) do
    key =
      open_channel_key_grant(
        grant,
        state.originator_id,
        state.receiver_id,
        state.channel_id,
        state.receiver_key_pair,
        state.originator_public_key
      )

    updated = %{
      state
      | epoch_keys: Map.put(state.epoch_keys, grant.key_epoch, key),
        latest_grant: grant
    }

    {"installed", updated}
  end

  defp structural_grant!(attributes) do
    originator = require_binary!(Map.fetch!(attributes, :originator_id))
    receiver = require_binary!(Map.fetch!(attributes, :receiver_id))
    if byte_size(originator) > @max_identity_bytes, do: fail!("length_limit_exceeded")
    if byte_size(receiver) > @max_identity_bytes, do: fail!("length_limit_exceeded")

    struct!(PortableKeyGrant, %{
      originator_id: originator,
      receiver_id: receiver,
      channel_id: fixed_binary!(Map.fetch!(attributes, :channel_id), 16),
      key_epoch: require_u64!(Map.fetch!(attributes, :key_epoch)),
      ephemeral_public_key: fixed_binary!(Map.fetch!(attributes, :ephemeral_public_key), 32),
      wrapping_nonce: fixed_binary!(Map.fetch!(attributes, :wrapping_nonce), 24),
      wrapped_cmk: fixed_binary!(Map.fetch!(attributes, :wrapped_cmk), 48),
      originator_signature: fixed_binary!(Map.fetch!(attributes, :originator_signature), 64)
    })
  rescue
    error in ProfileError -> raise error
    _ -> fail!("invalid_field")
  end

  defp validate_fields!(%GrantFields{} = fields) do
    require_identity!(fields.originator_id)
    require_identity!(fields.receiver_id)
    require_channel_id!(fields.channel_id)
    require_u64!(fields.key_epoch)
    fields
  end

  defp validate_grant!(%PortableKeyGrant{} = grant) do
    require_identity!(grant.originator_id)
    require_identity!(grant.receiver_id)
    require_channel_id!(grant.channel_id)
    require_u64!(grant.key_epoch)
    fixed_binary!(grant.ephemeral_public_key, 32)
    fixed_binary!(grant.wrapping_nonce, 24)
    fixed_binary!(grant.wrapped_cmk, 48)
    fixed_binary!(grant.originator_signature, 64)
    grant
  end

  defp clone_channel_master_key(%ChannelMasterKey{} = key),
    do: key |> channel_master_key_bytes() |> channel_master_key_from_bytes()

  defp clone_receiver_key_pair(%ReceiverKeyPair{private_key: private, destroyed: false}),
    do: receiver_key_pair_from_private_key(private)

  defp clone_receiver_key_pair(_), do: fail!("invalid_field")

  defp receiver_agree(%ReceiverKeyPair{private_key: private, destroyed: false}, peer),
    do: x25519_agree(private, fixed_binary!(peer, 32))

  defp receiver_agree(_, _), do: fail!("invalid_field")

  defp sign(%OriginatorSigningKey{secret_key: secret, destroyed: false}, message) do
    try do
      Ed25519.sign(message, secret)
    rescue
      _ -> fail!("invalid_field")
    end
  end

  defp sign(_, _), do: fail!("invalid_field")

  defp derive_key_grant_wrapping_key(shared_secret, channel_id, key_epoch, receiver_id) do
    require_length!(shared_secret, 32)

    try do
      key =
        Hkdf.hkdf(
          key_grant_hkdf_salt(channel_id, key_epoch),
          shared_secret,
          key_grant_hkdf_info(receiver_id),
          32,
          :sha256
        )

      if byte_size(key) != 32, do: fail!("key_derivation_failed")
      key
    rescue
      error in ProfileError -> raise error
      _ -> fail!("key_derivation_failed")
    end
  end

  defp grant_aad_values(originator, receiver, channel, epoch, ephemeral_public) do
    frame([@grant_context, originator, channel, u64be(epoch), receiver, ephemeral_public])
  end

  defp grant_signature_values(
         originator,
         receiver,
         channel,
         epoch,
         ephemeral_public,
         nonce,
         wrapped_cmk
       ) do
    frame([
      @grant_context,
      originator,
      channel,
      u64be(epoch),
      receiver,
      ephemeral_public,
      nonce,
      wrapped_cmk
    ])
  end

  defp frame(fields) do
    fields
    |> Enum.map(fn field ->
      value = require_binary!(field)
      <<byte_size(value)::unsigned-big-64, value::binary>>
    end)
    |> IO.iodata_to_binary()
  end

  defp u64be(value), do: <<require_u64!(value)::unsigned-big-64>>

  defp x25519_keypair(private) do
    try do
      X25519.generate_keypair(private)
    rescue
      _ -> fail!("invalid_key_agreement")
    end
  end

  defp x25519_public(private), do: private |> x25519_keypair() |> elem(1)

  defp x25519_agree(private, public) do
    try do
      X25519.x25519(private, public)
    rescue
      _ -> fail!("invalid_key_agreement")
    end
  end

  defp secure_random_bytes(source, length) when is_function(source, 1) do
    try do
      value = source.(length)

      if is_binary(value) and byte_size(value) == length do
        value
      else
        fail!("randomness_unavailable")
      end
    rescue
      error in ProfileError -> raise error
      _ -> fail!("randomness_unavailable")
    end
  end

  defp secure_random_bytes(_, _), do: fail!("randomness_unavailable")

  defp take(data, length) when is_binary(data) and byte_size(data) >= length do
    <<value::binary-size(length), rest::binary>> = data
    {value, rest}
  end

  defp take(_, _), do: fail!("truncated_record")

  defp read_identity(data) do
    {length_bytes, rest} = take(data, 4)
    <<length::unsigned-big-32>> = length_bytes
    if length > @max_identity_bytes, do: fail!("length_limit_exceeded")
    take(rest, length)
  end

  defp require_identity!(value) do
    identity = require_binary!(value)
    if identity == <<>>, do: fail!("invalid_field")
    if byte_size(identity) > @max_identity_bytes, do: fail!("length_limit_exceeded")
    identity
  end

  defp require_channel_id!(value) do
    channel = fixed_binary!(value, 16)
    <<_::binary-size(6), version::4, _::12, variant::2, _::6, _::binary>> = channel
    unless version == 7 and variant == 2, do: fail!("invalid_field")
    channel
  end

  defp fixed_binary!(value, length) do
    binary = require_binary!(value)
    require_length!(binary, length)
    binary
  end

  defp require_binary!(value) when is_binary(value), do: value
  defp require_binary!(_), do: fail!("invalid_field")

  defp require_length!(value, length) do
    unless byte_size(value) == length, do: fail!("invalid_field")
    value
  end

  defp require_u64!(value)
       when is_integer(value) and value >= 0 and value <= @max_u64,
       do: value

  defp require_u64!(_), do: fail!("invalid_field")

  defp constant_time_equal(left, right) when byte_size(left) == byte_size(right) do
    left
    |> :binary.bin_to_list()
    |> Enum.zip(:binary.bin_to_list(right))
    |> Enum.reduce(0, fn {a, b}, difference -> difference ||| bxor(a, b) end)
    |> Kernel.==(0)
  end

  defp constant_time_equal(_, _), do: false

  defp fail!(code) when code in @error_codes, do: raise(ProfileError, code: code)
end
