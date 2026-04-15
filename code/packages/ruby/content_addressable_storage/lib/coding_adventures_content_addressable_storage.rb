# frozen_string_literal: true

# coding_adventures_content_addressable_storage — Generic Content-Addressable Storage
#
# What Is Content-Addressable Storage?
# =====================================
# Ordinary storage maps a *name* to content: ask for "photo.jpg", get the photo.
# CAS flips the relationship — you ask for the *hash of the content*, and you
# get that content back. The hash is both the address and the integrity check.
#
#   Traditional:  name  ──►  content   (name can lie; content can change)
#   CAS:          hash  ──►  content   (hash is derived from content, cannot lie)
#
# The defining property: if you know the hash, you know the content. If the
# stored bytes don't hash to the address you asked for, the store is corrupt.
# This makes CAS self-authenticating — trust the hash, trust the data.
#
# Git's entire object model is CAS. Every file snapshot (blob), directory
# listing (tree), commit, and tag is stored by the SHA-1 hash of its serialized
# bytes. Two identical files → one object. A renamed file → zero new storage.
# History is an immutable DAG of hashes pointing to hashes.
#
# Architecture
# ============
#
#   ┌──────────────────────────────────────────────────────────┐
#   │  ContentAddressableStore                                  │
#   │                                                           │
#   │  put(data)       → 20-byte binary key (SHA-1)            │
#   │    1. key = SHA1(data)                                    │
#   │    2. if !store.exists?(key) → store.put(key, data)       │
#   │    3. return key                                          │
#   │                                                           │
#   │  get(key)        → raw bytes                             │
#   │    1. data = store.get(key)                               │
#   │    2. verify SHA1(data) == key  (integrity check)         │
#   │    3. return data                                         │
#   │                                                           │
#   │  find_by_prefix(hex) → 20-byte key                       │
#   │    1. decode hex prefix to bytes                          │
#   │    2. store.keys_with_prefix(prefix_bytes)                │
#   │    3. error if 0 or 2+ matches, else return the one key   │
#   └────────────────────────┬─────────────────────────────────┘
#                            │ BlobStore module (interface)
#              ┌─────────────┴──────────────────────────────┐
#              │                                            │
#       LocalDiskStore                          (future: S3Store, MemStore…)
#       root/XX/38-hex-chars
#       atomic rename writes
#
# Usage Example
# =============
#
#   require "coding_adventures_content_addressable_storage"
#   require "tmpdir"
#
#   Dir.mktmpdir do |root|
#     store = CodingAdventures::ContentAddressableStorage::LocalDiskStore.new(root)
#     cas   = CodingAdventures::ContentAddressableStorage::ContentAddressableStore.new(store)
#
#     key  = cas.put("hello, world")
#     data = cas.get(key)
#     puts data   # → "hello, world"
#   end

require "fileutils"
require "coding_adventures_sha1"
require_relative "coding_adventures/content_addressable_storage/version"

module CodingAdventures
  module ContentAddressableStorage
    # ─── Hex Utilities ─────────────────────────────────────────────────────────
    #
    # Keys are 20-byte binary Strings (Encoding::BINARY / ASCII-8BIT). Humans
    # interact with them as 40-character lowercase hex strings, e.g.
    # "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5".
    #
    # key_to_hex  — 20-byte binary String → 40-char hex String
    # hex_to_key  — 40-char hex String    → 20-byte binary String (raises on bad input)

    # Convert a 20-byte binary key to a 40-character lowercase hex string.
    #
    #   CodingAdventures::ContentAddressableStorage.key_to_hex("\xa3\xf4".b + "\x00" * 18)
    #   # → "a3f4000000000000000000000000000000000000"
    #
    # String#unpack1 with "H*" is the idiomatic Ruby way to hexlify a binary
    # string. "H*" means "as many uppercase nibble pairs as possible", but
    # we downcase the result to match Git's lowercase convention.
    def self.key_to_hex(key)
      key.unpack1("H*")
    end

    # Parse a 40-character hex string into a 20-byte binary String.
    #
    # Raises ArgumentError if the string is not exactly 40 hex characters.
    #
    #   key = CodingAdventures::ContentAddressableStorage.hex_to_key("a9993e364706816aba3e25717850c26c9cd0d89d")
    #   key.bytesize  # → 20
    #   key.encoding  # → #<Encoding:ASCII-8BIT>
    #
    # Array#pack with "H*" decodes a hex string into binary bytes.
    def self.hex_to_key(hex)
      raise ArgumentError, "expected 40 hex chars, got #{hex.length}" unless hex.length == 40
      raise ArgumentError, "invalid hex characters in: #{hex.inspect}" unless hex.match?(/\A[0-9a-fA-F]+\z/)

      [hex].pack("H*")
    end

    # ─── Internal: decode_hex_prefix ───────────────────────────────────────────
    #
    # Decode an arbitrary-length hex string (1–40 chars, may be odd length) into
    # a byte-prefix String (Encoding::BINARY).
    #
    # Odd-length strings are right-padded with '0' before decoding, because a
    # nibble prefix like "a3f" means "starts with 0xa3, 0xf0" — the trailing
    # nibble is the high nibble of the next byte.
    #
    #   "a3f"  → "\xa3\xf0"   (3 chars → pad to "a3f0" → 2 bytes)
    #   "a3f4" → "\xa3\xf4"   (4 chars → no pad → 2 bytes)
    #   ""     → raises (empty prefix is not allowed)
    #   "xyz"  → raises (non-hex characters)
    #
    # Returns a binary String on success, raises ArgumentError on failure.
    def self.decode_hex_prefix(hex)
      raise ArgumentError, "prefix cannot be empty" if hex.empty?
      raise ArgumentError, "invalid hex characters in prefix: #{hex.inspect}" unless hex.match?(/\A[0-9a-fA-F]+\z/)

      # Pad to even length if needed. "a3f" → "a3f0"
      padded = hex.length.odd? ? "#{hex}0" : hex
      [padded].pack("H*")
    end

    # ─── CasError Hierarchy ────────────────────────────────────────────────────
    #
    # Typed exceptions separate CAS-level failures from general runtime errors.
    # Each subclass carries the context needed to diagnose the problem:
    #
    #   CasError               — base class; all CAS exceptions are-a CasError
    #   ├─ CasNotFoundError    — key not in store; carries the missing key
    #   ├─ CasCorruptedError   — stored bytes don't hash to the key; integrity violation
    #   ├─ CasAmbiguousPrefixError  — hex prefix matches ≥2 objects; carries prefix string
    #   ├─ CasPrefixNotFoundError   — hex prefix matches 0 objects; carries prefix string
    #   └─ CasInvalidPrefixError    — hex prefix is not valid hex or is empty; carries prefix

    # Base class for all CAS exceptions.
    #
    # Rescue CasError to catch any CAS-level failure without caring which one.
    # Rescue a specific subclass (e.g., CasNotFoundError) for targeted handling.
    class CasError < StandardError; end

    # Raised when a blob is requested by key but the store has no such key.
    #
    # The `key` attribute holds the 20-byte binary key that was not found.
    # Use CodingAdventures::ContentAddressableStorage.key_to_hex(err.key) to convert it to a human-
    # readable 40-char hex string for display.
    class CasNotFoundError < CasError
      attr_reader :key

      def initialize(key)
        @key = key
        super("object not found: #{CodingAdventures::ContentAddressableStorage.key_to_hex(key)}")
      end
    end

    # Raised when the store returns bytes whose SHA-1 does not match the key.
    #
    # This is a data integrity violation — the stored bytes have been modified
    # since they were first written (disk corruption, manual editing, etc.).
    # The `key` attribute holds the *requested* key (the expected hash).
    class CasCorruptedError < CasError
      attr_reader :key

      def initialize(key)
        @key = key
        super("object corrupted: #{CodingAdventures::ContentAddressableStorage.key_to_hex(key)}")
      end
    end

    # Raised when a hex prefix matches two or more stored objects.
    #
    # Analogous to git's "error: short SHA1 ... is ambiguous". The caller should
    # supply a longer prefix to narrow the match to a single object.
    class CasAmbiguousPrefixError < CasError
      attr_reader :prefix

      def initialize(prefix)
        @prefix = prefix
        super("ambiguous prefix: #{prefix}")
      end
    end

    # Raised when a hex prefix matches zero stored objects.
    class CasPrefixNotFoundError < CasError
      attr_reader :prefix

      def initialize(prefix)
        @prefix = prefix
        super("object not found for prefix: #{prefix}")
      end
    end

    # Raised when a hex prefix string contains non-hex characters or is empty.
    class CasInvalidPrefixError < CasError
      attr_reader :prefix

      def initialize(prefix)
        @prefix = prefix
        super("invalid hex prefix: #{prefix.inspect}")
      end
    end

    # ─── BlobStore Module ──────────────────────────────────────────────────────
    #
    # BlobStore is a Ruby module used as an abstract interface (a "mixin trait").
    #
    # Any class that includes BlobStore and implements the four required methods
    # becomes a valid backend for ContentAddressableStore.
    #
    # Why a module rather than an abstract class?
    #
    #   Ruby supports single inheritance. Using a module as an interface keeps
    #   the backend classes free to inherit from whatever they need. It also
    #   makes it easy to include multiple interfaces in test doubles.
    #
    # The four required methods:
    #
    #   put(key, data)              — store data under the 20-byte binary key
    #   get(key)                    — retrieve the stored bytes (raise if missing)
    #   exists?(key)                — return true/false without fetching the blob
    #   keys_with_prefix(prefix)    — return all keys whose first bytes match prefix
    #
    # All methods accept `key` as a 20-byte binary String (Encoding::BINARY).
    # `data` is any String (will be treated as binary bytes).
    # `prefix` is a binary String of 1–20 bytes.
    #
    # Contract: `put` is idempotent. Storing the same key twice with the same
    # bytes is not an error. Implementations may skip the write if the object is
    # already present (recommended — avoid unnecessary I/O).
    module BlobStore
      # Called when this module is included in a class. Defines the interface
      # contract as comments rather than raising NotImplementedError, so that
      # subclasses see meaningful errors when they forget to implement a method,
      # while also enabling partial implementations for testing.
      def self.included(base)
        # Nothing to configure at include time. The four methods below must be
        # defined by the including class. We rely on Ruby's natural NoMethodError
        # rather than raising at include time, so partial implementations work
        # during development without tripping stubs.
        base
      end

      # Store `data` under `key`.
      #
      # `key`  — 20-byte binary String (the SHA-1 hash of data)
      # `data` — binary String of any length
      #
      # Must be idempotent: calling put twice with the same key and data is safe.
      # May short-circuit (return early) if the object is already stored.
      def put(_key, _data)
        raise NotImplementedError, "#{self.class}#put is not implemented"
      end

      # Retrieve the bytes stored under `key`.
      #
      # Raises an appropriate error if the key is not present.
      # Does NOT verify the hash — that is the CAS layer's job.
      def get(_key)
        raise NotImplementedError, "#{self.class}#get is not implemented"
      end

      # Return true if `key` is present, false otherwise.
      #
      # Must not raise on a missing key — that is the purpose of the method.
      def exists?(_key)
        raise NotImplementedError, "#{self.class}#exists? is not implemented"
      end

      # Return an Array of all stored 20-byte binary keys whose first bytes
      # match `prefix`.
      #
      # `prefix` — binary String, 1–20 bytes
      #
      # Used by ContentAddressableStore#find_by_prefix to resolve abbreviated
      # hex hashes (e.g., the 7-char prefix in `git log --oneline`).
      def keys_with_prefix(_prefix)
        raise NotImplementedError, "#{self.class}#keys_with_prefix is not implemented"
      end
    end

    # ─── ContentAddressableStore ───────────────────────────────────────────────
    #
    # The CAS class wraps any BlobStore and adds three capabilities:
    #
    #   1. Automatic keying  — callers pass content; SHA-1 is computed internally.
    #      Callers never choose a key; the hash *is* the key.
    #
    #   2. Integrity check on read — after store.get, the CAS re-hashes the bytes
    #      and raises CasCorruptedError if they don't match the requested key.
    #      The store cannot lie: if the bytes changed, we know immediately.
    #
    #   3. Prefix resolution — translates abbreviated hex (like "a3f4b2") into a
    #      full 20-byte key, with proper not-found / ambiguous discrimination.
    #      This mirrors `git show a3f4b2` in the Git command-line.
    #
    # Deduplication falls out naturally: the same content always produces the
    # same SHA-1 key. The LocalDiskStore (and any well-implemented backend)
    # skips the write if the object is already present.
    class ContentAddressableStore
      # Create a new CAS wrapping `store`.
      #
      # `store` must include the BlobStore module (or duck-type its interface).
      #
      #   store = CodingAdventures::ContentAddressableStorage::LocalDiskStore.new("/tmp/mystore")
      #   cas   = CodingAdventures::ContentAddressableStorage::ContentAddressableStore.new(store)
      def initialize(store)
        @store = store
      end

      # Hash `data` with SHA-1, store it in the backend, return the 20-byte key.
      #
      # Idempotent: if the same content is already stored, the existing key is
      # returned and no write is performed (the backend handles this).
      #
      #   key1 = cas.put("hello")
      #   key2 = cas.put("hello")   # no I/O — already exists
      #   key1 == key2              # → true
      #
      # The returned key is a 20-byte binary String (Encoding::BINARY).
      # Convert it to hex with CodingAdventures::ContentAddressableStorage.key_to_hex(key).
      #
      # We use the repo's own CodingAdventures::Sha1 implementation so the
      # entire stack is self-contained and every language port uses the same
      # hand-rolled algorithm — the educational point of this monorepo.
      def put(data)
        # Force binary encoding so hashing is byte-accurate regardless of how
        # the caller encoded the string (UTF-8, ASCII, etc.).
        raw = data.b
        key = CodingAdventures::Sha1.sha1(raw)
        @store.put(key, raw)
        key
      end

      # Retrieve the blob stored under `key` and verify its integrity.
      #
      # The returned bytes are guaranteed to hash to `key`. If the store returns
      # bytes that don't match, CasCorruptedError is raised — data is corrupt.
      #
      # Raises:
      #   CasNotFoundError   — if no blob is stored under `key`
      #   CasCorruptedError  — if the stored bytes don't hash to `key`
      def get(key)
        begin
          data = @store.get(key)
        rescue => e
          # Translate store-level "not found" errors (e.g., Errno::ENOENT from
          # LocalDiskStore) into the typed CasNotFoundError so callers don't need
          # to know backend-specific error classes.
          raise CasNotFoundError.new(key) if not_found_error?(e)

          raise
        end

        # Integrity check: re-hash the bytes that came back from the store.
        #
        # Why check on every read? If a disk sector flips, a file is truncated,
        # or a bug in the store returns the wrong object, we catch it here —
        # before the caller acts on corrupt data. The cost is one SHA-1 per read,
        # which is cheap compared to the value of detecting silent corruption.
        actual = CodingAdventures::Sha1.sha1(data)
        raise CasCorruptedError.new(key) unless actual == key

        data
      end

      # Return true if a blob is stored under `key`, false otherwise.
      #
      # Does not verify integrity — use get for that.
      def exists?(key)
        @store.exists?(key)
      end

      # Resolve an abbreviated hex string to a full 20-byte key.
      #
      # Accepts any non-empty hex string of 1–40 characters. Odd-length strings
      # are treated as nibble prefixes — "a3f" matches any key starting with
      # 0xa3 0xf0 (the trailing nibble is the high nibble of the next byte).
      #
      # This mirrors `git log --oneline` abbreviated hashes. Git uses 7 chars by
      # default; we accept any length from 1 to 40.
      #
      # Raises:
      #   CasInvalidPrefixError      — empty string or non-hex characters
      #   CasPrefixNotFoundError     — no keys match the prefix
      #   CasAmbiguousPrefixError    — two or more keys match (use a longer prefix)
      def find_by_prefix(hex_prefix)
        # Validate the prefix.
        raise CasInvalidPrefixError.new(hex_prefix) if hex_prefix.empty?
        raise CasInvalidPrefixError.new(hex_prefix) unless hex_prefix.match?(/\A[0-9a-fA-F]+\z/)

        # Odd-length hex prefix handling — the core of correct nibble matching.
        #
        # A 7-char prefix like "1bafb97" means: match any key whose hex starts
        # with the nibbles 1, b, a, f, b, 9, 7.  That is:
        #   - first 3 bytes exactly [0x1b, 0xaf, 0xb9]
        #   - high nibble of the 4th byte == 7  (i.e., 4th byte is 0x70..0x7f)
        #
        # Padding "1bafb97" → "1bafb970" and passing 4 bytes to keys_with_prefix
        # would only match keys starting with exactly [0x1b, 0xaf, 0xb9, 0x70],
        # missing "1bafb97a...", "1bafb97c...", etc.
        #
        # Correct approach:
        #   1. Pass only the COMPLETE bytes (floor(len/2)) to keys_with_prefix.
        #   2. Filter the returned candidates by checking the trailing nibble.
        is_odd = (hex_prefix.length % 2 == 1)
        trailing_nibble_val = nil
        complete_hex = hex_prefix

        if is_odd
          trailing_nibble_val = hex_prefix[-1].to_i(16)   # 0–15
          complete_hex = hex_prefix[0..-2]                 # all but last char
        end

        # Decode the complete hex pairs to bytes.
        # [complete_hex].pack("H*") returns "" for an empty string — that is fine.
        prefix_bytes = [complete_hex].pack("H*")

        if is_odd && prefix_bytes.empty?
          # 1-nibble prefix: scan all 16 possible first bytes (0xN0 through 0xNf).
          # For example, "a" should match keys in buckets a0/, a1/, …, af/.
          matches = []
          16.times do |lo|
            first_byte_val = (trailing_nibble_val << 4) | lo
            matches.concat(@store.keys_with_prefix([first_byte_val].pack("C")))
          end
        else
          matches = @store.keys_with_prefix(prefix_bytes)

          if is_odd
            # Filter: keep only keys where the nibble at position (2*n) in the
            # 40-char hex representation equals the trailing nibble.
            n = prefix_bytes.bytesize
            expected_nibble = trailing_nibble_val.to_s(16)
            matches.select! do |key|
              CodingAdventures::ContentAddressableStorage.key_to_hex(key)[n * 2] == expected_nibble
            end
          end
        end

        # Sort for deterministic behaviour (important for tests and stable UX).
        matches.sort!

        case matches.length
        when 0
          raise CasPrefixNotFoundError.new(hex_prefix)
        when 1
          matches[0]
        else
          raise CasAmbiguousPrefixError.new(hex_prefix)
        end
      end

      # Access the underlying BlobStore directly.
      #
      # Useful for backend-specific operations not exposed by the CAS interface,
      # such as listing all keys for garbage collection or querying storage stats.
      def inner
        @store
      end

      private

      # Heuristic: detect whether a backend exception means "key not found".
      #
      # LocalDiskStore raises Errno::ENOENT when the file doesn't exist. Other
      # backends might raise different errors. We check for the most common ones
      # here so that CasNotFoundError is raised consistently regardless of backend.
      #
      # This is intentionally conservative — if we can't classify the error,
      # we re-raise it unchanged so the caller sees the real backend error.
      def not_found_error?(err)
        err.is_a?(Errno::ENOENT) ||
          (err.respond_to?(:message) && err.message.include?("not found"))
      end
    end

    # ─── LocalDiskStore ────────────────────────────────────────────────────────
    #
    # Filesystem backend using the Git 2/38 fanout directory layout.
    #
    # Why 2/38? A repository with 100 000 objects stored flat as root/<40-hex>
    # would put 100 000 files in a single directory. Most filesystems degrade
    # dramatically at that scale (ext4 uses htree but still slows; HFS+ degrades).
    # Splitting on the first byte creates up to 256 sub-directories, keeping each
    # to ~390 entries for a 100 k object repo. Git has used this layout since day 1.
    #
    # Object path layout:
    #
    #   key = "\xa3\xf4\xb2…" (20 bytes)
    #   hex = "a3f4b2…"       (40 chars)
    #   dir  = root/a3/        (first 2 hex chars = first byte)
    #   file = root/a3/f4b2…   (remaining 38 hex chars)
    #
    # Atomic write protocol:
    #
    #   1. Check if the object already exists — skip if yes (idempotent).
    #   2. Create the fanout directory (e.g., root/a3/) if needed.
    #   3. Write data to a temp file in the same directory as the final path.
    #      The temp filename includes the PID and a high-resolution timestamp so
    #      it is unpredictable. A fixed .tmp name would be vulnerable to symlink
    #      attacks from local adversaries.
    #   4. File.rename(tmp_path, final_path) — atomic on POSIX (POSIX.1 guarantee).
    #      On Windows, rename may fail if the destination already exists; we treat
    #      that as success (another writer stored the same object concurrently).
    class LocalDiskStore
      include BlobStore

      # Create (or open) a LocalDiskStore rooted at `root`.
      #
      # The directory is created recursively if it does not exist.
      #
      #   store = CodingAdventures::ContentAddressableStorage::LocalDiskStore.new("/tmp/myobjects")
      def initialize(root)
        @root = root.to_s
        FileUtils.mkdir_p(@root)
      end

      # Persist `data` under `key` using an atomic write.
      #
      # Short-circuits (no-op) if the object is already present. Because the key
      # is the SHA-1 of the content, an existing file with the same name is
      # guaranteed to contain identical bytes — no need to overwrite.
      def put(key, data)
        final_path = object_path(key)

        # Short-circuit: object already stored → idempotent return.
        return if File.exist?(final_path)

        # Create the fanout directory (e.g., root/a3/) before writing.
        dir = File.dirname(final_path)
        FileUtils.mkdir_p(dir)

        # Build an unpredictable temp filename.
        #
        # Security rationale: a deterministic name like "f4b2...tmp" in a
        # world-writable temp directory lets a local attacker pre-place a symlink
        # at that path, redirecting our File.open to an arbitrary target. Mixing
        # the process PID with a nanosecond-resolution timestamp makes the name
        # effectively unguessable without privileged access to the process.
        ns = Process.clock_gettime(Process::CLOCK_REALTIME, :nanosecond)
        tmp_name = "#{File.basename(final_path)}.#{Process.pid}.#{ns}.tmp"
        tmp_path = File.join(dir, tmp_name)

        begin
          # Write to the temp file in the same directory as the final file so
          # that File.rename is on the same filesystem (cross-device renames fail
          # on POSIX with Errno::EXDEV).
          File.open(tmp_path, "wb") { |f| f.write(data.b) }

          # Atomic rename into place.
          begin
            File.rename(tmp_path, final_path)
          rescue Errno::EEXIST, Errno::ENOTEMPTY
            # Windows: rename fails if the destination already exists.
            # Another writer stored the same object concurrently — that is fine.
            File.unlink(tmp_path) if File.exist?(tmp_path)
          end
        rescue
          # Clean up the temp file to avoid leaving orphans on unexpected errors.
          File.unlink(tmp_path) if File.exist?(tmp_path)
          raise
        end
      end

      # Retrieve the bytes stored under `key`.
      #
      # Raises Errno::ENOENT if the key is not present. The CAS layer translates
      # this into CasNotFoundError so callers get a typed error.
      def get(key)
        File.binread(object_path(key))
      end

      # Return true if `key` is present in the store, false otherwise.
      def exists?(key)
        File.exist?(object_path(key))
      end

      # Return all stored keys whose first bytes match `prefix`.
      #
      # `prefix` is a binary String of 1 or more bytes. The first byte determines
      # which fanout bucket (e.g., "\xa3" → scan root/a3/). We then scan all 38-
      # char filenames in that bucket and filter to those whose full 40-char hex
      # starts with the hex encoding of `prefix`.
      #
      # Returns an Array of 20-byte binary key Strings.
      def keys_with_prefix(prefix)
        # A zero-byte prefix would match everything — not useful and potentially
        # dangerous (imagine scanning all 256 buckets for a large store).
        return [] if prefix.empty?

        # The first byte of the prefix encodes as a 2-char hex directory name.
        # "\xa3" → "a3", "\x0f" → "0f"
        first_byte_hex = prefix.unpack1("H2")
        bucket = File.join(@root, first_byte_hex)

        return [] unless Dir.exist?(bucket)

        # Convert the full prefix to hex for string-prefix matching.
        # "\xa3\xf0" → "a3f0"
        prefix_hex = prefix.unpack1("H*")

        keys = []
        Dir.each_child(bucket) do |name|
          # Each valid object filename is exactly 38 hex characters (the latter
          # 38 chars of the 40-char full hex hash). Skip anything else (temp files,
          # OS metadata, etc.).
          next unless name.length == 38 && name.match?(/\A[0-9a-f]+\z/)

          full_hex = "#{first_byte_hex}#{name}"

          # Does this key start with the requested prefix?
          next unless full_hex.start_with?(prefix_hex)

          # Reconstruct the 20-byte binary key from the hex.
          keys << [full_hex].pack("H*")
        end

        keys
      end

      private

      # Compute the filesystem path for a given key.
      #
      # key = "\xa3\xf4\xb2…" (20 bytes)
      # hex = "a3f4b2…"        (40 chars)
      # dir  = @root/a3         (first 2 chars)
      # file = @root/a3/f4b2…   (remaining 38 chars)
      def object_path(key)
        hex = key.unpack1("H*")
        dir_name  = hex[0, 2]
        file_name = hex[2, 38]
        File.join(@root, dir_name, file_name)
      end
    end

  end
end
