# frozen_string_literal: true

require "securerandom"
require "coding_adventures_chacha20_poly1305"
require "coding_adventures_ed25519"
require "coding_adventures_hkdf"
require "coding_adventures_x25519"

module CodingAdventures
  module ChiefOfStaffChannelCrypto
    GRANT_MAGIC = "D18G".b.freeze
    GRANT_WIRE_VERSION = 1
    KEY_GRANT_CONTEXT = "chief-channel-key-grant-v1".b.freeze
    KEY_WRAP_CONTEXT = "chief-channel-key-wrap-v1".b.freeze
    KEY_GRANT_ERROR_CODES = %w[
      invalid_magic unsupported_version truncated_record trailing_bytes
      length_limit_exceeded invalid_field randomness_unavailable
      invalid_key_agreement key_derivation_failed invalid_signature
      unexpected_originator unexpected_receiver unexpected_channel
      authentication_failed invalid_wrapped_key conflicting_grant
      decreasing_epoch epoch_exhausted missing_epoch_key
    ].freeze

    class KeyGrantProfileError < StandardError
      attr_reader :code

      def initialize(code)
        raise ArgumentError, "unknown D18Q error code" unless KEY_GRANT_ERROR_CODES.include?(code)

        @code = code.freeze
        super(code)
      end
    end

    class SystemSecureRandomSource
      def random_bytes(length) = SecureRandom.random_bytes(length)
    end

    SYSTEM_SECURE_RANDOM_SOURCE = SystemSecureRandomSource.new.freeze

    class ChannelMasterKey
      def self.from_bytes(value) = new(value)

      def self.generate(source = SYSTEM_SECURE_RANDOM_SOURCE)
        new(KeyGrantSupport.secure_random_bytes(source, 32))
      end

      def initialize(value)
        copied = KeyGrantSupport.copy_bytes(value)
        KeyGrantSupport.require_length(copied, 32)
        @bytes = copied.dup
        @destroyed = false
      end

      def bytes
        require_alive
        @bytes.dup
      end

      def clone = self.class.new(bytes)

      def destroy
        KeyGrantSupport.wipe(@bytes)
        @destroyed = true
        nil
      end

      def inspect = "ChannelMasterKey(<#{@destroyed ? 'destroyed' : 'secret'}>)"
      alias to_s inspect

      private

      def require_alive
        KeyGrantSupport.fail!("invalid_field") if @destroyed
      end
    end

    class ReceiverKeyPair
      def self.from_private_key(private_key)
        private_copy = KeyGrantSupport.copy_bytes(private_key)
        KeyGrantSupport.require_length(private_copy, 32)
        public_key = KeyGrantSupport.x25519_public(private_copy)
        new(private_copy, public_key)
      rescue KeyGrantProfileError
        raise
      rescue StandardError
        KeyGrantSupport.fail!("invalid_key_agreement")
      end

      def self.generate(source = SYSTEM_SECURE_RANDOM_SOURCE)
        from_private_key(KeyGrantSupport.secure_random_bytes(source, 32))
      end

      def initialize(private_key, public_key)
        @private_key = private_key.dup
        @public_key = public_key.dup.freeze
        @destroyed = false
      end

      def public_key
        require_alive
        @public_key.dup
      end

      def agree(peer_public_key)
        require_alive
        peer = KeyGrantSupport.copy_bytes(peer_public_key)
        KeyGrantSupport.require_length(peer, 32)
        KeyGrantSupport.x25519(@private_key, peer)
      rescue KeyGrantProfileError
        raise
      rescue StandardError
        KeyGrantSupport.fail!("invalid_key_agreement")
      end

      def clone
        require_alive
        self.class.new(@private_key, @public_key)
      end

      def destroy
        KeyGrantSupport.wipe(@private_key)
        @destroyed = true
        nil
      end

      def inspect
        "ReceiverKeyPair(<#{@destroyed ? 'destroyed' : 'secret'}>, public_key=#{@public_key.unpack1('H*')})"
      end
      alias to_s inspect

      private

      def require_alive
        KeyGrantSupport.fail!("invalid_field") if @destroyed
      end
    end

    class OriginatorSigningKey
      def self.from_seed(seed)
        seed_copy = KeyGrantSupport.copy_bytes(seed)
        KeyGrantSupport.require_length(seed_copy, 32)
        public_key, secret_key = Ed25519.generate_keypair(seed_copy)
        new(secret_key, public_key)
      rescue KeyGrantProfileError
        raise
      rescue StandardError
        KeyGrantSupport.fail!("invalid_field")
      end

      def self.generate(source = SYSTEM_SECURE_RANDOM_SOURCE)
        from_seed(KeyGrantSupport.secure_random_bytes(source, 32))
      end

      def initialize(secret_key, public_key)
        @secret_key = secret_key.dup
        @public_key = public_key.dup.freeze
        @destroyed = false
      end

      def public_key
        require_alive
        @public_key.dup
      end

      def sign(message)
        require_alive
        Ed25519.sign(KeyGrantSupport.copy_bytes(message), @secret_key)
      end

      def destroy
        KeyGrantSupport.wipe(@secret_key)
        @destroyed = true
        nil
      end

      def inspect
        "OriginatorSigningKey(<#{@destroyed ? 'destroyed' : 'secret'}>, public_key=#{@public_key.unpack1('H*')})"
      end
      alias to_s inspect

      private

      def require_alive
        KeyGrantSupport.fail!("invalid_field") if @destroyed
      end
    end

    class KeyGrantFields
      attr_reader :key_epoch

      def initialize(originator_id, receiver_id, channel_id, key_epoch)
        @originator_id = KeyGrantSupport.copy_bytes(originator_id).freeze
        @receiver_id = KeyGrantSupport.copy_bytes(receiver_id).freeze
        @channel_id = KeyGrantSupport.copy_bytes(channel_id).freeze
        KeyGrantSupport.validate_identity(@originator_id)
        KeyGrantSupport.validate_identity(@receiver_id)
        KeyGrantSupport.validate_channel_id(@channel_id)
        KeyGrantSupport.require_u64(key_epoch)
        @key_epoch = key_epoch
        freeze
      end

      def originator_id = @originator_id.dup
      def receiver_id = @receiver_id.dup
      def channel_id = @channel_id.dup
      def __values = [@originator_id, @receiver_id, @channel_id, @key_epoch]
    end

    class PortableKeyGrant
      attr_reader :key_epoch

      def initialize(originator_id:, receiver_id:, channel_id:, key_epoch:, ephemeral_public_key:, wrapping_nonce:,
                     wrapped_cmk:, originator_signature:)
        @originator_id = KeyGrantSupport.copy_bytes(originator_id).freeze
        @receiver_id = KeyGrantSupport.copy_bytes(receiver_id).freeze
        if @originator_id.bytesize > MAX_IDENTITY_BYTES || @receiver_id.bytesize > MAX_IDENTITY_BYTES
          KeyGrantSupport.fail!("length_limit_exceeded")
        end
        @channel_id = KeyGrantSupport.fixed_copy(channel_id, 16)
        @ephemeral_public_key = KeyGrantSupport.fixed_copy(ephemeral_public_key, 32)
        @wrapping_nonce = KeyGrantSupport.fixed_copy(wrapping_nonce, 24)
        @wrapped_cmk = KeyGrantSupport.fixed_copy(wrapped_cmk, 48)
        @originator_signature = KeyGrantSupport.fixed_copy(originator_signature, 64)
        KeyGrantSupport.require_u64(key_epoch)
        @key_epoch = key_epoch
        freeze
      end

      def originator_id = @originator_id.dup
      def receiver_id = @receiver_id.dup
      def channel_id = @channel_id.dup
      def ephemeral_public_key = @ephemeral_public_key.dup
      def wrapping_nonce = @wrapping_nonce.dup
      def wrapped_cmk = @wrapped_cmk.dup
      def originator_signature = @originator_signature.dup

      def __values
        [@originator_id, @receiver_id, @channel_id, @key_epoch, @ephemeral_public_key, @wrapping_nonce,
         @wrapped_cmk, @originator_signature]
      end

      def ==(other) = other.is_a?(PortableKeyGrant) && __values == other.__values
      alias eql? ==
      def hash = __values.hash
    end

    class ReceiverEpochKeys
      def initialize(originator_id, receiver_id, channel_id, receiver_key_pair, originator_public_key)
        @originator_id = KeyGrantSupport.copy_bytes(originator_id).freeze
        @receiver_id = KeyGrantSupport.copy_bytes(receiver_id).freeze
        @channel_id = KeyGrantSupport.copy_bytes(channel_id).freeze
        @originator_public_key = KeyGrantSupport.fixed_copy(originator_public_key, 32)
        KeyGrantSupport.validate_identity(@originator_id)
        KeyGrantSupport.validate_identity(@receiver_id)
        KeyGrantSupport.validate_channel_id(@channel_id)
        KeyGrantSupport.fail!("invalid_field") unless receiver_key_pair.is_a?(ReceiverKeyPair)
        @receiver_key_pair = receiver_key_pair.clone
        @epoch_keys = {}
        @latest_grant = nil
      end

      def receiver_public_key = @receiver_key_pair.public_key
      def latest_epoch = @latest_grant&.key_epoch

      def install_grant(grant)
        KeyGrantSupport.fail!("invalid_field") unless grant.is_a?(PortableKeyGrant)
        unless @latest_grant.nil?
          KeyGrantSupport.fail!("decreasing_epoch") if grant.key_epoch < @latest_grant.key_epoch
          if grant.key_epoch == @latest_grant.key_epoch
            return "idempotent" if grant == @latest_grant

            KeyGrantSupport.fail!("conflicting_grant")
          end
        end
        key = ChiefOfStaffChannelCrypto.open_channel_key_grant(
          grant, @originator_id, @receiver_id, @channel_id, @receiver_key_pair, @originator_public_key
        )
        @epoch_keys[grant.key_epoch] = key
        @latest_grant = grant
        "installed"
      end

      def key(epoch)
        KeyGrantSupport.require_u64(epoch)
        retained = @epoch_keys[epoch]
        KeyGrantSupport.fail!("missing_epoch_key") if retained.nil?
        retained.clone
      end

      def destroy
        @epoch_keys.each_value(&:destroy)
        @epoch_keys.clear
        @receiver_key_pair.destroy
        @latest_grant = nil
        nil
      end
    end

    class RotationReceiver
      def self.with_material(receiver_id, public_key, ephemeral_private_key, wrapping_nonce)
        new(receiver_id, public_key, ephemeral_private_key, wrapping_nonce)
      end

      def self.generate(receiver_id, public_key, source = SYSTEM_SECURE_RANDOM_SOURCE)
        new(
          receiver_id, public_key,
          KeyGrantSupport.secure_random_bytes(source, 32),
          KeyGrantSupport.secure_random_bytes(source, 24)
        )
      end

      def initialize(receiver_id, public_key, ephemeral_private_key, wrapping_nonce)
        @receiver_id = KeyGrantSupport.copy_bytes(receiver_id).freeze
        @public_key = KeyGrantSupport.fixed_copy(public_key, 32)
        @ephemeral_private_key = KeyGrantSupport.fixed_copy(ephemeral_private_key, 32).dup
        @wrapping_nonce = KeyGrantSupport.fixed_copy(wrapping_nonce, 24)
        KeyGrantSupport.validate_identity(@receiver_id)
        @destroyed = false
      end

      def receiver_id = @receiver_id.dup

      def seal(fields, cmk, signing_key)
        KeyGrantSupport.fail!("invalid_field") if @destroyed
        ChiefOfStaffChannelCrypto.seal_channel_key_with_material(
          fields, cmk, @public_key, signing_key, @ephemeral_private_key, @wrapping_nonce
        )
      end

      def destroy
        KeyGrantSupport.wipe(@ephemeral_private_key)
        @destroyed = true
        nil
      end
    end

    class RotationPlan
      attr_reader :new_epoch

      def initialize(new_epoch, new_cmk, grants)
        @new_epoch = new_epoch
        @new_cmk = new_cmk.clone
        @grants = grants.dup.freeze
        freeze
      end

      def new_cmk = @new_cmk.clone
      def grants = @grants.dup
      def destroy = @new_cmk.destroy
    end

    module KeyGrantSupport
      module_function

      def fail!(code) = raise(KeyGrantProfileError, code)

      def copy_bytes(value)
        fail!("invalid_field") unless value.is_a?(String)
        value.b.dup
      rescue Encoding::CompatibilityError
        fail!("invalid_field")
      end

      def fixed_copy(value, length)
        copy = copy_bytes(value)
        require_length(copy, length)
        copy.freeze
      end

      def require_length(value, length)
        fail!("invalid_field") unless value.bytesize == length
      end

      def require_u64(value)
        fail!("invalid_field") unless value.is_a?(Integer) && value.between?(0, MAX_U64)
      end

      def validate_identity(value)
        fail!("invalid_field") if value.empty?
        fail!("length_limit_exceeded") if value.bytesize > MAX_IDENTITY_BYTES
      end

      def validate_channel_id(value)
        require_length(value, 16)
        fail!("invalid_field") unless value.getbyte(6) >> 4 == 7 && value.getbyte(8) & 0xc0 == 0x80
      end

      def validate_grant(grant)
        fail!("invalid_field") unless grant.is_a?(PortableKeyGrant)
        originator_id, receiver_id, channel_id, key_epoch = grant.__values
        validate_identity(originator_id)
        validate_identity(receiver_id)
        validate_channel_id(channel_id)
        require_u64(key_epoch)
      end

      def u64be(value)
        require_u64(value)
        [value].pack("Q>")
      end

      def frame(fields) = fields.map { |field| u64be(field.bytesize) + field }.join.b

      def x25519_public(private_key)
        X25519.generate_keypair(private_key.bytes).pack("C*").b
      rescue StandardError
        fail!("invalid_key_agreement")
      end

      def x25519(private_key, public_key)
        X25519.x25519(private_key.bytes, public_key.bytes).pack("C*").b
      rescue StandardError
        fail!("invalid_key_agreement")
      end

      def secure_random_bytes(source, length)
        value = copy_bytes(source.random_bytes(length))
        fail!("randomness_unavailable") unless value.bytesize == length
        value
      rescue KeyGrantProfileError
        raise
      rescue StandardError
        fail!("randomness_unavailable")
      end

      def equal_bytes(left, right)
        return false unless left.bytesize == right.bytesize
        left.bytes.zip(right.bytes).reduce(0) { |difference, pair| difference | (pair[0] ^ pair[1]) }.zero?
      end

      def wipe(value)
        value.replace("\0" * value.bytesize)
        nil
      end
    end

    class GrantDecoder
      def initialize(data)
        @data = KeyGrantSupport.copy_bytes(data)
        @offset = 0
      end

      def take(length)
        KeyGrantSupport.fail!("truncated_record") if length.negative? || length > @data.bytesize - @offset
        value = @data.byteslice(@offset, length)
        @offset += length
        value
      end

      def read_identity
        length = take(4).unpack1("N")
        KeyGrantSupport.fail!("length_limit_exceeded") if length > MAX_IDENTITY_BYTES
        take(length)
      end

      def read_u64 = take(8).unpack1("Q>")

      def finish
        KeyGrantSupport.fail!("trailing_bytes") unless @offset == @data.bytesize
      end
    end

    module_function

    def grant_deserialize(data)
      decoder = GrantDecoder.new(data)
      KeyGrantSupport.fail!("invalid_magic") unless decoder.take(4) == GRANT_MAGIC
      KeyGrantSupport.fail!("unsupported_version") unless decoder.take(1).getbyte(0) == GRANT_WIRE_VERSION
      grant = PortableKeyGrant.new(
        originator_id: decoder.read_identity,
        receiver_id: decoder.read_identity,
        channel_id: decoder.take(16),
        key_epoch: decoder.read_u64,
        ephemeral_public_key: decoder.take(32),
        wrapping_nonce: decoder.take(24),
        wrapped_cmk: decoder.take(48),
        originator_signature: decoder.take(64)
      )
      decoder.finish
      grant
    end

    def grant_serialize(grant)
      KeyGrantSupport.validate_grant(grant)
      originator_id, receiver_id, channel_id, key_epoch, ephemeral_public_key, wrapping_nonce, wrapped_cmk,
        signature = grant.__values
      [
        GRANT_MAGIC, GRANT_WIRE_VERSION.chr.b, [originator_id.bytesize].pack("N"), originator_id,
        [receiver_id.bytesize].pack("N"), receiver_id, channel_id, KeyGrantSupport.u64be(key_epoch),
        ephemeral_public_key, wrapping_nonce, wrapped_cmk, signature
      ].join.b
    end

    def seal_channel_key(fields, cmk, receiver_public_key, signing_key, source = SYSTEM_SECURE_RANDOM_SOURCE)
      ephemeral_private_key = KeyGrantSupport.secure_random_bytes(source, 32)
      wrapping_nonce = KeyGrantSupport.secure_random_bytes(source, 24)
      seal_channel_key_with_material(
        fields, cmk, receiver_public_key, signing_key, ephemeral_private_key, wrapping_nonce
      )
    ensure
      KeyGrantSupport.wipe(ephemeral_private_key) unless ephemeral_private_key.nil?
    end

    def seal_channel_key_with_material(fields, cmk, receiver_public_key, signing_key, ephemeral_private_key,
                                       wrapping_nonce)
      KeyGrantSupport.fail!("invalid_field") unless fields.is_a?(KeyGrantFields)
      KeyGrantSupport.fail!("invalid_field") unless cmk.is_a?(ChannelMasterKey)
      KeyGrantSupport.fail!("invalid_field") unless signing_key.is_a?(OriginatorSigningKey)
      receiver_public = KeyGrantSupport.fixed_copy(receiver_public_key, 32)
      ephemeral_private = KeyGrantSupport.fixed_copy(ephemeral_private_key, 32).dup
      nonce = KeyGrantSupport.fixed_copy(wrapping_nonce, 24)
      originator_id, receiver_id, channel_id, key_epoch = fields.__values
      ephemeral_public = KeyGrantSupport.x25519_public(ephemeral_private)
      shared_secret = KeyGrantSupport.x25519(ephemeral_private, receiver_public)
      wrapping_key = derive_key_grant_wrapping_key(shared_secret, channel_id, key_epoch, receiver_id)
      aad = grant_aad_values(originator_id, receiver_id, channel_id, key_epoch, ephemeral_public)
      ciphertext, tag = Chacha20Poly1305.xchacha20_poly1305_encrypt(cmk.bytes, wrapping_key, nonce, aad)
      wrapped_cmk = ciphertext + tag
      signature_input = grant_signature_values(
        originator_id, receiver_id, channel_id, key_epoch, ephemeral_public, nonce, wrapped_cmk
      )
      PortableKeyGrant.new(
        originator_id: originator_id,
        receiver_id: receiver_id,
        channel_id: channel_id,
        key_epoch: key_epoch,
        ephemeral_public_key: ephemeral_public,
        wrapping_nonce: nonce,
        wrapped_cmk: wrapped_cmk,
        originator_signature: signing_key.sign(signature_input)
      )
    rescue KeyGrantProfileError
      raise
    rescue StandardError
      KeyGrantSupport.fail!("authentication_failed")
    ensure
      KeyGrantSupport.wipe(ephemeral_private) unless ephemeral_private.nil?
      KeyGrantSupport.wipe(shared_secret) unless shared_secret.nil?
      KeyGrantSupport.wipe(wrapping_key) unless wrapping_key.nil?
    end

    def open_channel_key_grant(grant, expected_originator_id, expected_receiver_id, expected_channel_id,
                               receiver_key_pair, originator_public_key)
      KeyGrantSupport.validate_grant(grant)
      expected_originator = KeyGrantSupport.copy_bytes(expected_originator_id)
      expected_receiver = KeyGrantSupport.copy_bytes(expected_receiver_id)
      expected_channel = KeyGrantSupport.fixed_copy(expected_channel_id, 16)
      public_key = KeyGrantSupport.fixed_copy(originator_public_key, 32)
      KeyGrantSupport.fail!("invalid_field") unless receiver_key_pair.is_a?(ReceiverKeyPair)
      originator_id, receiver_id, channel_id, key_epoch, ephemeral_public, nonce, wrapped_cmk,
        signature = grant.__values
      KeyGrantSupport.fail!("unexpected_originator") unless KeyGrantSupport.equal_bytes(originator_id, expected_originator)
      KeyGrantSupport.fail!("unexpected_receiver") unless KeyGrantSupport.equal_bytes(receiver_id, expected_receiver)
      KeyGrantSupport.fail!("unexpected_channel") unless KeyGrantSupport.equal_bytes(channel_id, expected_channel)
      signature_input = grant_signature_values(
        originator_id, receiver_id, channel_id, key_epoch, ephemeral_public, nonce, wrapped_cmk
      )
      signature_valid = Ed25519.verify(signature_input, signature, public_key)
      KeyGrantSupport.fail!("invalid_signature") unless signature_valid
      shared_secret = receiver_key_pair.agree(ephemeral_public)
      wrapping_key = derive_key_grant_wrapping_key(shared_secret, channel_id, key_epoch, receiver_id)
      aad = grant_aad_values(originator_id, receiver_id, channel_id, key_epoch, ephemeral_public)
      plaintext = Chacha20Poly1305.xchacha20_poly1305_decrypt(
        wrapped_cmk.byteslice(0, 32), wrapping_key, nonce, aad, wrapped_cmk.byteslice(32, 16)
      )
      KeyGrantSupport.fail!("invalid_wrapped_key") unless plaintext.bytesize == 32
      ChannelMasterKey.from_bytes(plaintext)
    rescue KeyGrantProfileError
      raise
    rescue StandardError
      if signature_valid == false || signature_valid.nil?
        KeyGrantSupport.fail!("invalid_signature")
      else
        KeyGrantSupport.fail!("authentication_failed")
      end
    ensure
      KeyGrantSupport.wipe(shared_secret) unless shared_secret.nil?
      KeyGrantSupport.wipe(wrapping_key) unless wrapping_key.nil?
      KeyGrantSupport.wipe(plaintext) unless plaintext.nil?
    end

    def plan_rotation(originator_id, channel_id, current_epoch, new_cmk, receivers, signing_key)
      originator = KeyGrantSupport.copy_bytes(originator_id)
      channel = KeyGrantSupport.copy_bytes(channel_id)
      KeyGrantSupport.validate_identity(originator)
      KeyGrantSupport.validate_channel_id(channel)
      KeyGrantSupport.require_u64(current_epoch)
      KeyGrantSupport.fail!("epoch_exhausted") if current_epoch == MAX_U64
      KeyGrantSupport.fail!("invalid_field") unless new_cmk.is_a?(ChannelMasterKey)
      KeyGrantSupport.fail!("invalid_field") unless signing_key.is_a?(OriginatorSigningKey)
      KeyGrantSupport.fail!("invalid_field") unless receivers.is_a?(Array) && !receivers.empty?
      KeyGrantSupport.fail!("invalid_field") unless receivers.all? { |receiver| receiver.is_a?(RotationReceiver) }
      ordered = receivers.sort_by(&:receiver_id)
      if ordered.each_cons(2).any? { |left, right| left.receiver_id == right.receiver_id }
        ordered.each(&:destroy)
        KeyGrantSupport.fail!("invalid_field")
      end
      grants = ordered.map do |receiver|
        fields = KeyGrantFields.new(originator, receiver.receiver_id, channel, current_epoch + 1)
        receiver.seal(fields, new_cmk, signing_key)
      end
      RotationPlan.new(current_epoch + 1, new_cmk, grants)
    ensure
      ordered&.each(&:destroy)
    end

    def secret_erasure_capability = "best_effort"

    def key_grant_hkdf_salt(channel_id, key_epoch)
      channel = KeyGrantSupport.copy_bytes(channel_id)
      KeyGrantSupport.require_length(channel, 16)
      KeyGrantSupport.frame([channel, KeyGrantSupport.u64be(key_epoch)])
    end

    def key_grant_hkdf_info(receiver_id)
      receiver = KeyGrantSupport.copy_bytes(receiver_id)
      KeyGrantSupport.fail!("length_limit_exceeded") if receiver.bytesize > MAX_IDENTITY_BYTES
      KeyGrantSupport.frame([KEY_WRAP_CONTEXT, receiver])
    end

    def key_grant_aad(grant)
      KeyGrantSupport.fail!("invalid_field") unless grant.is_a?(PortableKeyGrant)
      values = grant.__values
      grant_aad_values(values[0], values[1], values[2], values[3], values[4])
    end

    def key_grant_signature_input(grant)
      KeyGrantSupport.fail!("invalid_field") unless grant.is_a?(PortableKeyGrant)
      grant_signature_values(*grant.__values.first(7))
    end

    def key_grant_wrapping_key(shared_secret, channel_id, key_epoch, receiver_id)
      derive_key_grant_wrapping_key(
        KeyGrantSupport.copy_bytes(shared_secret), KeyGrantSupport.copy_bytes(channel_id), key_epoch,
        KeyGrantSupport.copy_bytes(receiver_id)
      )
    end

    def derive_key_grant_wrapping_key(shared_secret, channel_id, key_epoch, receiver_id)
      KeyGrantSupport.require_length(shared_secret, 32)
      key = HKDF.hkdf(
        key_grant_hkdf_salt(channel_id, key_epoch), shared_secret, key_grant_hkdf_info(receiver_id), 32, "sha256"
      )
      KeyGrantSupport.fail!("key_derivation_failed") unless key.bytesize == 32
      key
    rescue KeyGrantProfileError
      raise
    rescue StandardError
      KeyGrantSupport.fail!("key_derivation_failed")
    end
    private_class_method :derive_key_grant_wrapping_key

    def grant_aad_values(originator_id, receiver_id, channel_id, key_epoch, ephemeral_public_key)
      KeyGrantSupport.frame(
        [KEY_GRANT_CONTEXT, originator_id, channel_id, KeyGrantSupport.u64be(key_epoch), receiver_id,
         ephemeral_public_key]
      )
    end
    private_class_method :grant_aad_values

    def grant_signature_values(originator_id, receiver_id, channel_id, key_epoch, ephemeral_public_key,
                               wrapping_nonce, wrapped_cmk)
      KeyGrantSupport.frame(
        [KEY_GRANT_CONTEXT, originator_id, channel_id, KeyGrantSupport.u64be(key_epoch), receiver_id,
         ephemeral_public_key, wrapping_nonce, wrapped_cmk]
      )
    end
    private_class_method :grant_signature_values
  end
end
