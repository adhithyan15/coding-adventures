# frozen_string_literal: true

# ca_uuid.rb — UUID generation and parsing (RFC 4122 + RFC 9562) from scratch.
#
# What Is a UUID?
# ===============
# A UUID (Universally Unique Identifier) is a 128-bit label used to identify
# information without a central coordinator. Two independent computers can each
# generate a UUID and be virtually certain the values will never collide.
#
# Standard string representation — 32 hexadecimal digits in 5 groups separated
# by hyphens, totaling 36 characters:
#
#   550e8400-e29b-41d4-a716-446655440000
#   ^^^^^^^^ ^^^^ ^^^^ ^^^^ ^^^^^^^^^^^^
#   time_low  mid  hi   seq    node
#   (32 bit) (16) (16) (16)  (48 bit)
#
# The 128 bits carry two critical fields:
#   - Version (bits 76-79): tells which algorithm made this UUID (1, 3, 4, 5, 7)
#   - Variant (bits 62-63): tells the bit layout. "10xx" = RFC 4122 (IETF standard)
#
# UUID Versions
# =============
# v1  — time-based. Embeds a 60-bit Gregorian timestamp + clock sequence + node.
#        Sortable by time. May leak the host MAC address (or random node bits).
# v3  — name-based, MD5 hashed. Deterministic: same namespace+name → same UUID.
# v4  — random. 122 bits from a CSPRNG. The most common UUID in the wild.
# v5  — name-based, SHA-1 hashed. Preferred over v3 for new code.
# v7  — Unix-time-based (RFC 9562). Millisecond precision timestamp in the MSB,
#        random remainder. Sortable by creation time (better than v1 for databases).
#
# Internal Representation
# =======================
# A UUID is stored internally as @bytes: a 16-byte binary String
# (Encoding::ASCII_8BIT, also called BINARY). This makes pack/unpack fast and
# avoids encoding confusion. All public methods that return strings produce
# regular UTF-8 or ASCII output.
#
# Gregorian Epoch Offset (v1)
# ===========================
# RFC 4122 v1 counts 100-nanosecond intervals since 1582-10-15 (the adoption of
# the Gregorian calendar). Unix time counts seconds since 1970-01-01. The offset
# between those epochs in 100-ns units:
#
#   Days  between 1582-10-15 and 1970-01-01 = 122192928
#   x 86400 seconds/day = 10,542,252,748,800 seconds
#   x 10,000,000 (100-ns per second)         = 122,192,928,000,000,000
#
# GREGORIAN_OFFSET = 122_192_928_000_000_000
#
# Key RFC Test Vectors
# ====================
# v5(NAMESPACE_DNS, "python.org") => "886313e1-3b8a-5372-9b90-0c9aee199e5d"
# v3(NAMESPACE_DNS, "python.org") => "6fa459ea-ee8a-3ca4-894e-db77e160355e"

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby's require mechanism is order-dependent. If CodingAdventures::Sha1 or CodingAdventures::Md5 constants
# are needed when loading coding_adventures/uuid/version, they must already exist in the VM.
require "coding_adventures_sha1"
require "coding_adventures_md5"
require "securerandom"
require_relative "coding_adventures/uuid/version"

module CodingAdventures
  # UUID — Universally Unique Identifier library.
  #
  # Implements UUID v1, v3, v4, v5, v7 from scratch using only:
  #   - ca_sha1 (our SHA-1 implementation) for v5
  #   - ca_md5  (our MD5  implementation) for v3
  #   - Ruby's SecureRandom for cryptographic random bytes
  #   - Process.clock_gettime for high-resolution wall-clock time
  #
  # Quick start:
  #   CodingAdventures::Uuid.v4.to_s        # random v4
  #   CodingAdventures::Uuid.v7.to_s        # time-sortable UUID
  #   CodingAdventures::Uuid.v5(CodingAdventures::Uuid::NAMESPACE_DNS, "example.com").to_s
  #   CodingAdventures::Uuid.parse("550e8400-e29b-41d4-a716-446655440000")
  module Uuid
    # ---- Error Class -----------------------------------------------------------

    # Raised when a string cannot be parsed as a valid UUID, or when bytes
    # passed to from_bytes are not exactly 16.
    class UUIDError < StandardError; end

    # ---- UUID Class ------------------------------------------------------------

    # Represents a single 128-bit UUID.
    #
    # Internally, the UUID is stored as @bytes: a 16-byte String in
    # Encoding::ASCII_8BIT (BINARY). This encoding is Ruby's way to say
    # "these bytes have no character meaning — treat them as raw octets."
    #
    # Using binary encoding avoids a common Ruby pitfall: if you concatenate
    # UTF-8 and ASCII strings unexpectedly, Ruby raises Encoding::CompatibilityError.
    # Binary strings accept any byte value without complaint.
    class UUID
      # Regex that matches any canonically or compactly written UUID string.
      #
      # Accepted forms:
      #   "6ba7b810-9dad-11d1-80b4-00c04fd430c8"   (canonical, lowercase)
      #   "6BA7B810-9DAD-11D1-80B4-00C04FD430C8"   (canonical, uppercase)
      #   "6ba7b8109dad11d180b400c04fd430c8"        (compact, no hyphens)
      #   "{6ba7b810-9dad-11d1-80b4-00c04fd430c8}" (braces)
      #   "urn:uuid:6ba7b810-9dad-11d1-80b4-00c04fd430c8" (URN)
      #
      # The five capture groups match the five UUID fields (without hyphens).
      HEX_RE = /\A\s*(?:urn:uuid:)?\{?([0-9a-f]{8})-?([0-9a-f]{4})-?([0-9a-f]{4})-?([0-9a-f]{4})-?([0-9a-f]{12})\}?\s*\z/i

      # ---- Construction --------------------------------------------------------

      # Create a UUID from a string representation.
      #
      # Accepts canonical ("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"), compact
      # (32 hex chars), URN, or brace-wrapped forms.
      # Raises UUIDError for any unrecognized format.
      #
      # For internal construction from raw bytes, use UUID.from_bytes.
      def initialize(str)
        @bytes = parse_str(str)
      end

      # Parse a string into 16 binary bytes. Internal helper.
      #
      # Strategy: strip optional whitespace/braces/URN prefix, apply HEX_RE,
      # join the five hex groups, then pack into binary with ["hex"].pack("H*").
      #
      # ["6ba7b810"].pack("H*") => "\x6b\xa7\xb8\x10" (4 binary bytes)
      # So packing the 32-char join gives exactly 16 bytes.
      private def parse_str(str)
        m = HEX_RE.match(str.to_s)
        raise UUIDError, "Invalid UUID string: #{str.inspect}" unless m
        [m[1] + m[2] + m[3] + m[4] + m[5]].pack("H*")
      end

      # ---- Class Methods: Factories --------------------------------------------

      # Parse a UUID string. Raises UUIDError for invalid input.
      #
      #   UUID.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
      def self.parse(str)
        new(str)
      end

      # Test whether a string is a valid UUID without raising.
      #
      #   UUID.valid?("6ba7b810-9dad-11d1-80b4-00c04fd430c8")  # => true
      #   UUID.valid?("not-a-uuid")                             # => false
      def self.valid?(str)
        HEX_RE.match?(str.to_s)
      end

      # Construct a UUID from a 16-byte binary string.
      #
      # This is the internal factory used by the version generators. It avoids
      # the overhead of string parsing when we already have raw bytes.
      # The .b call ensures the bytes are in BINARY encoding.
      #
      # Raises UUIDError unless bytes.bytesize == 16.
      def self.from_bytes(bytes)
        raise UUIDError, "UUID bytes must be 16, got #{bytes.bytesize}" unless bytes.bytesize == 16
        u = allocate
        u.instance_variable_set(:@bytes, bytes.b)
        u
      end

      # ---- Version Generators -------------------------------------------------

      # UUID v1: time-based (RFC 4122 ss 4.2)
      #
      # Layout of the 16 bytes:
      #
      #   Byte range  Field                  Width
      #   ----------  ---------------------  -----
      #   0-3         time_low               32 bits  (low 32 bits of 60-bit timestamp)
      #   4-5         time_mid               16 bits  (bits 32-47 of timestamp)
      #   6-7         time_hi_and_version    16 bits  (bits 48-59 of timestamp + version=1)
      #   8           clock_seq_hi_and_res    8 bits  (variant + high 6 bits of clock seq)
      #   9           clock_seq_low           8 bits  (low 8 bits of clock seq)
      #   10-15       node                   48 bits  (MAC or random)
      #
      # The timestamp is the number of 100-nanosecond intervals since 1582-10-15.
      # We convert from the POSIX nanosecond clock, adding GREGORIAN_OFFSET.
      #
      # Because we cannot guarantee a monotonic clock sequence between calls,
      # we use random bytes for clock_seq (per RFC 4122 ss 4.1.5, random node).
      # We also set the multicast bit on node[0] to signal "random node" (not MAC).
      def self.v1
        # Number of 100-ns intervals since Gregorian epoch
        t100ns = Process.clock_gettime(Process::CLOCK_REALTIME, :nanosecond) / 100 +
                 GREGORIAN_OFFSET

        # Split the 60-bit timestamp into three RFC 4122 fields
        time_low            = t100ns & 0xFFFFFFFF                    # bits 0-31
        time_mid            = (t100ns >> 32) & 0xFFFF               # bits 32-47
        time_hi             = (t100ns >> 48) & 0x0FFF               # bits 48-59
        time_hi_and_version = 0x1000 | time_hi                      # set version = 1

        # Clock sequence: 14 random bits (we cannot track state between calls)
        clock_seq     = SecureRandom.random_bytes(2).unpack1("n") & 0x3FFF
        clock_seq_hi  = 0x80 | (clock_seq >> 8)   # variant bits 10xx + high 6 of seq
        clock_seq_low = clock_seq & 0xFF

        # Node: 6 random bytes with multicast bit set to advertise "random node"
        # RFC 4122 ss 4.5: if no MAC, generate random node with multicast bit set
        node = SecureRandom.random_bytes(6)
        node.setbyte(0, node.getbyte(0) | 0x01)  # set multicast (bit 0) = random

        # Pack: N=uint32 big-endian, n=uint16 big-endian, C=uint8
        raw = [time_low, time_mid, time_hi_and_version, clock_seq_hi, clock_seq_low].pack("N n n C C") + node
        from_bytes(raw)
      end

      # UUID v3: name-based, MD5 hashed (RFC 4122 ss 4.3)
      #
      # Algorithm:
      #   1. Concatenate namespace UUID bytes + name (UTF-8 encoded)
      #   2. Compute MD5 => 16 bytes
      #   3. Set version bits (byte 6, high nibble = 0x3)
      #   4. Set variant bits (byte 8, top 2 bits = 10)
      #
      # v3 is deterministic: the same namespace and name always produce
      # the same UUID. Two parties who agree on the namespace can compute
      # the same UUID independently.
      #
      # namespace — a UUID object (typically one of the NAMESPACE_* constants)
      # name      — an arbitrary string identifier
      def self.v3(namespace, name)
        # namespace.bytes_string returns the 16 raw bytes; name must be UTF-8 binary
        data = namespace.bytes_string + name.encode("UTF-8").b
        digest = CodingAdventures::Md5.md5(data)   # => 16-byte binary string
        raw = digest.bytes
        raw[6] = (raw[6] & 0x0F) | 0x30   # version = 3
        raw[8] = (raw[8] & 0x3F) | 0x80   # variant = 10xx
        from_bytes(raw.pack("C*"))
      end

      # UUID v4: randomly generated (RFC 4122 ss 4.4)
      #
      # Generate 16 random bytes from the OS CSPRNG (SecureRandom). Then:
      #   - Force bits 4-7 of byte 6 to 0100 (version = 4)
      #   - Force bits 6-7 of byte 8 to 10   (variant = RFC 4122)
      #
      # The remaining 122 bits are random. The probability of a v4 collision
      # is so low that for practical purposes, every v4 is unique.
      def self.v4
        raw = SecureRandom.random_bytes(16).bytes
        raw[6] = (raw[6] & 0x0F) | 0x40   # set version = 4
        raw[8] = (raw[8] & 0x3F) | 0x80   # set variant = 10xx
        from_bytes(raw.pack("C*"))
      end

      # UUID v5: name-based, SHA-1 hashed (RFC 4122 ss 4.3)
      #
      # Like v3 but uses SHA-1 instead of MD5. SHA-1 produces 20 bytes;
      # we use only the first 16, then overwrite the version/variant bits.
      # Preferred over v3 for new code.
      #
      # RFC 4122 Appendix B test vector:
      #   v5(NAMESPACE_DNS, "python.org") => "886313e1-3b8a-5372-9b90-0c9aee199e5d"
      def self.v5(namespace, name)
        data = namespace.bytes_string + name.encode("UTF-8").b
        digest = CodingAdventures::Sha1.sha1(data)   # => 20-byte binary string
        raw = digest[0, 16].bytes      # take first 16 bytes
        raw[6] = (raw[6] & 0x0F) | 0x50   # version = 5
        raw[8] = (raw[8] & 0x3F) | 0x80   # variant = 10xx
        from_bytes(raw.pack("C*"))
      end

      # UUID v7: Unix-time-based (RFC 9562 ss 5.7)
      #
      # Layout (128 bits):
      #
      #   Bit range   Field               Value
      #   ---------   -----------------   -----------------------------------
      #   0-47        unix_ts_ms          milliseconds since Unix epoch
      #   48-51       ver                 0b0111 (7)
      #   52-63       rand_a              12 random bits
      #   64-65       var                 0b10   (RFC 4122 variant)
      #   66-127      rand_b              62 random bits
      #
      # The big advantage over v1: the timestamp is in the most-significant bits
      # in natural byte order. This means v7 UUIDs sort chronologically with a
      # simple bytewise (lexicographic) comparison — ideal for database primary
      # keys where you want B-tree index locality.
      def self.v7
        ts_ms = Process.clock_gettime(Process::CLOCK_REALTIME, :millisecond)
        rand_bytes = SecureRandom.random_bytes(10).bytes

        raw = Array.new(16, 0)

        # Bytes 0-5: 48-bit millisecond timestamp, big-endian
        raw[0] = (ts_ms >> 40) & 0xFF
        raw[1] = (ts_ms >> 32) & 0xFF
        raw[2] = (ts_ms >> 24) & 0xFF
        raw[3] = (ts_ms >> 16) & 0xFF
        raw[4] = (ts_ms >> 8)  & 0xFF
        raw[5] =  ts_ms        & 0xFF

        # Byte 6: version nibble (0x70) OR lower nibble of first random byte
        raw[6] = 0x70 | (rand_bytes[0] & 0x0F)

        # Byte 7: rand_a lower byte
        raw[7] = rand_bytes[1]

        # Byte 8: variant (0x80) OR 6 random bits
        raw[8] = 0x80 | (rand_bytes[2] & 0x3F)

        # Bytes 9-15: 7 more random bytes (56 bits of rand_b)
        raw[9, 7] = rand_bytes[3, 7]

        from_bytes(raw.pack("C*"))
      end

      # ---- Instance Methods ----------------------------------------------------

      # Return the canonical UUID string: "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
      #
      # We unpack @bytes to 32 hex chars, then insert hyphens at the five field
      # boundaries defined by RFC 4122:
      #   chars  0- 7  => time_low       (4 bytes)
      #   chars  8-11  => time_mid       (2 bytes)
      #   chars 12-15  => time_hi+ver    (2 bytes)
      #   chars 16-19  => clock_seq      (2 bytes)
      #   chars 20-31  => node           (6 bytes)
      def to_s
        h = @bytes.unpack1("H*")   # 32 lowercase hex chars
        "#{h[0, 8]}-#{h[8, 4]}-#{h[12, 4]}-#{h[16, 4]}-#{h[20, 12]}"
      end

      alias inspect to_s

      # Return the UUID version number (1-8), or the raw nibble for nil/max UUIDs.
      #
      # The version is stored in bits 76-79 (the high nibble of byte 6).
      # We mask with 0xF0 to isolate the high nibble, then shift right by 4.
      #
      # Example: byte 6 = 0x47 => 0x47 & 0xF0 => 0x40 => 0x40 >> 4 => 4 (v4)
      def version
        (@bytes.getbyte(6) & 0xF0) >> 4
      end

      # Return the variant string.
      #
      # The variant is encoded in the top bits of byte 8:
      #   10xxxxxx => "rfc4122"   (all modern UUIDs)
      #   110xxxxx => "microsoft" (legacy Windows GUIDs)
      #   0xxxxxxx => "ncs"       (very old NCS UUIDs)
      #
      # We check the top two bits: if (byte & 0xC0) == 0x80, it's RFC 4122.
      def variant
        b = @bytes.getbyte(8)
        if (b & 0x80) == 0x00
          "ncs"
        elsif (b & 0xC0) == 0x80
          "rfc4122"
        elsif (b & 0xE0) == 0xC0
          "microsoft"
        else
          "future"
        end
      end

      # Return true if this is the nil UUID (all 128 bits are zero).
      # The nil UUID is used as a sentinel "no UUID" value.
      def nil?
        @bytes == "\x00".b * 16
      end

      # Return true if this is the max UUID (all 128 bits are one).
      # RFC 9562 ss 5.10: the max UUID is "ffffffff-ffff-ffff-ffff-ffffffffffff".
      def max?
        @bytes == "\xff".b * 16
      end

      # Return the raw 16 bytes as a binary String (Encoding::ASCII_8BIT).
      # Used internally by v3/v5 to build the hash input data.
      def bytes_string
        @bytes.dup
      end

      # Return an Array of 16 integers (0-255), one per byte.
      def bytes
        @bytes.bytes
      end

      # Return the UUID as a 128-bit Integer.
      #
      # We unpack to a 32-char hex string, then parse as base-16.
      # Ruby integers are arbitrary precision, so 128-bit values are fine.
      def to_i
        @bytes.unpack1("H*").to_i(16)
      end

      # Equality: two UUIDs are equal if their bytes are identical.
      def ==(other)
        return false unless other.is_a?(UUID)
        @bytes == other.instance_variable_get(:@bytes)
      end

      alias eql? ==

      def hash
        @bytes.hash
      end

      # Spaceship operator for ordering.
      #
      # UUID ordering is defined bytewise (lexicographic on the binary representation).
      # This is meaningful for v7 (time-sortable) and useful for sorting collections.
      def <=>(other)
        return nil unless other.is_a?(UUID)
        @bytes <=> other.instance_variable_get(:@bytes)
      end

      include Comparable
    end

    # ---- Gregorian Epoch Offset ------------------------------------------------
    #
    # Number of 100-nanosecond intervals between the Gregorian epoch (1582-10-15)
    # and the Unix epoch (1970-01-01 00:00:00 UTC).
    #
    # Derivation:
    #   Days  = (1970-01-01) minus (1582-10-15) = 122,192,928 days
    #   Secs  = 122,192,928 x 86,400 = 10,542,252,748,800 seconds
    #   100ns = 10,542,252,748,800 x 10,000,000 = 122,192,928,000,000,000
    GREGORIAN_OFFSET = 122_192_928_000_000_000

    # ---- Namespace UUIDs -------------------------------------------------------
    #
    # RFC 4122 Appendix C defines four pre-assigned namespace UUIDs. They are used
    # with v3 and v5 to generate deterministic UUIDs for names in well-known spaces.
    #
    # Usage:
    #   CodingAdventures::Uuid.v5(CodingAdventures::Uuid::NAMESPACE_DNS, "example.com")
    #   CodingAdventures::Uuid.v3(CodingAdventures::Uuid::NAMESPACE_URL, "https://example.com/page")

    # For names that are DNS domain names ("example.com", "python.org").
    NAMESPACE_DNS  = UUID.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8")

    # For names that are URLs ("https://example.com").
    NAMESPACE_URL  = UUID.parse("6ba7b811-9dad-11d1-80b4-00c04fd430c8")

    # For names that are ISO OID strings ("2.25.12345...").
    NAMESPACE_OID  = UUID.parse("6ba7b812-9dad-11d1-80b4-00c04fd430c8")

    # For names that are X.500 DN strings ("/CN=example/O=Example Corp").
    NAMESPACE_X500 = UUID.parse("6ba7b814-9dad-11d1-80b4-00c04fd430c8")

    # The nil UUID: all 128 bits zero. Used as a sentinel "no UUID" value.
    # RFC 4122 ss 4.1.7 and RFC 9562 ss 5.9.
    NIL = UUID.parse("00000000-0000-0000-0000-000000000000")

    # The max UUID: all 128 bits one. RFC 9562 ss 5.10.
    # Useful as an "infinity" sentinel in range queries.
    MAX = UUID.parse("ffffffff-ffff-ffff-ffff-ffffffffffff")

    # ---- Module-Level Convenience Methods -------------------------------------
    #
    # These delegate to the UUID class methods so callers can write
    #   CodingAdventures::Uuid.v4   instead of   CodingAdventures::Uuid::UUID.v4

    # Parse a UUID string. Raises UUIDError for invalid input.
    def self.parse(s) = UUID.parse(s)

    # Return true if s is a valid UUID string.
    def self.valid?(s) = UUID.valid?(s)

    # Generate a new UUID v1 (time-based).
    def self.v1 = UUID.v1

    # Generate a UUID v3 (name-based, MD5).
    def self.v3(ns, name) = UUID.v3(ns, name)

    # Generate a new UUID v4 (random).
    def self.v4 = UUID.v4

    # Generate a UUID v5 (name-based, SHA-1).
    def self.v5(ns, name) = UUID.v5(ns, name)

    # Generate a new UUID v7 (Unix-time-based, sortable).
    def self.v7 = UUID.v7
  end
end
