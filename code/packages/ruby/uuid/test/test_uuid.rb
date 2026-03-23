# frozen_string_literal: true

# test_uuid.rb — Comprehensive tests for ca_uuid.
#
# Test Strategy
# =============
# We test at three levels:
#   1. Unit tests for parsing and string formatting (deterministic, no randomness)
#   2. RFC test vectors for v3 and v5 (bit-exact deterministic outputs)
#   3. Property tests for v1, v4, v7 (check structure, not value)
#
# RFC 4122 Test Vectors (Appendix B)
# ===================================
# v5(NAMESPACE_DNS, "python.org") => "886313e1-3b8a-5372-9b90-0c9aee199e5d"
# v3(NAMESPACE_DNS, "python.org") => "6fa459ea-ee8a-3ca4-894e-db77e160355e"

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
end

require "minitest/autorun"
require "ca_uuid"

# Bring the UUID class and module-level constants into scope
UUID          = Ca::Uuid::UUID
UUIDError     = Ca::Uuid::UUIDError
NAMESPACE_DNS = Ca::Uuid::NAMESPACE_DNS
NAMESPACE_URL = Ca::Uuid::NAMESPACE_URL
NAMESPACE_OID = Ca::Uuid::NAMESPACE_OID
NAMESPACE_X500 = Ca::Uuid::NAMESPACE_X500
NIL_UUID      = Ca::Uuid::NIL
MAX_UUID      = Ca::Uuid::MAX

class TestCaUuidVersion < Minitest::Test
  # ---- Version constant -------------------------------------------------------

  def test_version_exists
    refute_nil Ca::Uuid::VERSION
  end

  def test_version_is_string
    assert_kind_of String, Ca::Uuid::VERSION
  end

  def test_version_format
    # Should match semver X.Y.Z
    assert_match(/\A\d+\.\d+\.\d+\z/, Ca::Uuid::VERSION)
  end
end

class TestUuidParsing < Minitest::Test
  # ---- UUID.parse: valid inputs -----------------------------------------------

  CANONICAL = "6ba7b810-9dad-11d1-80b4-00c04fd430c8"

  def test_parse_canonical_lowercase
    u = UUID.parse(CANONICAL)
    assert_equal CANONICAL, u.to_s
  end

  def test_parse_canonical_uppercase
    u = UUID.parse(CANONICAL.upcase)
    assert_equal CANONICAL, u.to_s
  end

  def test_parse_compact_no_hyphens
    compact = CANONICAL.delete("-")
    u = UUID.parse(compact)
    assert_equal CANONICAL, u.to_s
  end

  def test_parse_braces
    u = UUID.parse("{#{CANONICAL}}")
    assert_equal CANONICAL, u.to_s
  end

  def test_parse_urn
    u = UUID.parse("urn:uuid:#{CANONICAL}")
    assert_equal CANONICAL, u.to_s
  end

  def test_parse_with_leading_trailing_whitespace
    u = UUID.parse("  #{CANONICAL}  ")
    assert_equal CANONICAL, u.to_s
  end

  # ---- UUID.parse: invalid inputs ---------------------------------------------

  def test_parse_raises_on_empty_string
    assert_raises(UUIDError) { UUID.parse("") }
  end

  def test_parse_raises_on_garbage
    assert_raises(UUIDError) { UUID.parse("not-a-uuid") }
  end

  def test_parse_raises_on_too_short
    assert_raises(UUIDError) { UUID.parse("6ba7b810-9dad-11d1-80b4-00c04fd430") }
  end

  def test_parse_raises_on_wrong_grouping
    # Groups must be 8-4-4-4-12
    assert_raises(UUIDError) { UUID.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8-extra") }
  end

  # ---- UUID.valid? ------------------------------------------------------------

  def test_valid_canonical
    assert UUID.valid?(CANONICAL)
  end

  def test_valid_compact
    assert UUID.valid?(CANONICAL.delete("-"))
  end

  def test_invalid_garbage
    refute UUID.valid?("not-a-uuid")
  end

  def test_invalid_empty
    refute UUID.valid?("")
  end
end

class TestUuidToS < Minitest::Test
  # ---- to_s format -----------------------------------------------------------

  def test_to_s_is_36_chars
    assert_equal 36, UUID.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8").to_s.length
  end

  def test_to_s_has_four_hyphens
    s = UUID.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8").to_s
    assert_equal 4, s.count("-")
  end

  def test_to_s_is_lowercase
    u = UUID.parse("6BA7B810-9DAD-11D1-80B4-00C04FD430C8")
    assert_equal u.to_s, u.to_s.downcase
  end

  def test_to_s_roundtrip
    str = "550e8400-e29b-41d4-a716-446655440000"
    assert_equal str, UUID.parse(str).to_s
  end
end

class TestUuidVersion < Minitest::Test
  # ---- UUID#version -----------------------------------------------------------

  DNS_UUID = "6ba7b810-9dad-11d1-80b4-00c04fd430c8"

  def test_version_of_dns_namespace
    # 6ba7b810-9dad-11d1-... => byte 6 = 0x11 => high nibble = 0x1 => version 1
    assert_equal 1, UUID.parse(DNS_UUID).version
  end

  def test_v4_version
    assert_equal 4, Ca::Uuid.v4.version
  end

  def test_v1_version
    assert_equal 1, Ca::Uuid.v1.version
  end

  def test_v5_version
    u = Ca::Uuid.v5(NAMESPACE_DNS, "python.org")
    assert_equal 5, u.version
  end

  def test_v3_version
    u = Ca::Uuid.v3(NAMESPACE_DNS, "python.org")
    assert_equal 3, u.version
  end

  def test_v7_version
    assert_equal 7, Ca::Uuid.v7.version
  end
end

class TestUuidVariant < Minitest::Test
  # ---- UUID#variant -----------------------------------------------------------

  def test_v4_variant
    assert_equal "rfc4122", Ca::Uuid.v4.variant
  end

  def test_v1_variant
    assert_equal "rfc4122", Ca::Uuid.v1.variant
  end

  def test_v5_variant
    u = Ca::Uuid.v5(NAMESPACE_DNS, "example.com")
    assert_equal "rfc4122", u.variant
  end

  def test_v3_variant
    u = Ca::Uuid.v3(NAMESPACE_DNS, "example.com")
    assert_equal "rfc4122", u.variant
  end

  def test_v7_variant
    assert_equal "rfc4122", Ca::Uuid.v7.variant
  end

  def test_nil_uuid_variant
    # NIL UUID has byte 8 = 0x00 => top bit 0 => "ncs" variant
    assert_equal "ncs", NIL_UUID.variant
  end
end

class TestUuidNilMax < Minitest::Test
  # ---- UUID#nil? and UUID#max? ------------------------------------------------

  def test_nil_uuid_is_nil
    assert NIL_UUID.nil?
  end

  def test_max_uuid_is_max
    assert MAX_UUID.max?
  end

  def test_v4_is_not_nil
    refute Ca::Uuid.v4.nil?
  end

  def test_v4_is_not_max
    refute Ca::Uuid.v4.max?
  end

  def test_nil_to_s
    assert_equal "00000000-0000-0000-0000-000000000000", NIL_UUID.to_s
  end

  def test_max_to_s
    assert_equal "ffffffff-ffff-ffff-ffff-ffffffffffff", MAX_UUID.to_s
  end
end

class TestUuidEquality < Minitest::Test
  # ---- UUID#== ---------------------------------------------------------------

  CANONICAL = "6ba7b810-9dad-11d1-80b4-00c04fd430c8"

  def test_equal_same_string
    a = UUID.parse(CANONICAL)
    b = UUID.parse(CANONICAL)
    assert_equal a, b
  end

  def test_equal_compact_and_canonical
    a = UUID.parse(CANONICAL)
    b = UUID.parse(CANONICAL.delete("-"))
    assert_equal a, b
  end

  def test_not_equal_different_uuids
    a = UUID.parse(CANONICAL)
    b = UUID.parse("550e8400-e29b-41d4-a716-446655440000")
    refute_equal a, b
  end

  def test_not_equal_non_uuid
    a = UUID.parse(CANONICAL)
    refute_equal a, "not a uuid"
  end
end

class TestUuidComparison < Minitest::Test
  # ---- UUID#<=> and Comparable -----------------------------------------------

  def test_spaceship_equal
    a = UUID.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
    b = UUID.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
    assert_equal 0, a <=> b
  end

  def test_spaceship_less_than
    a = UUID.parse("00000000-0000-0000-0000-000000000000")
    b = UUID.parse("ffffffff-ffff-ffff-ffff-ffffffffffff")
    assert_equal(-1, a <=> b)
  end

  def test_spaceship_greater_than
    a = UUID.parse("ffffffff-ffff-ffff-ffff-ffffffffffff")
    b = UUID.parse("00000000-0000-0000-0000-000000000000")
    assert_equal 1, a <=> b
  end

  def test_spaceship_nil_for_non_uuid
    a = UUID.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
    assert_nil a <=> "string"
  end

  def test_sort_collection
    uuids = [
      UUID.parse("ffffffff-ffff-ffff-ffff-ffffffffffff"),
      UUID.parse("00000000-0000-0000-0000-000000000000"),
      UUID.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8"),
    ]
    sorted = uuids.sort
    assert_equal "00000000-0000-0000-0000-000000000000", sorted[0].to_s
    assert_equal "ffffffff-ffff-ffff-ffff-ffffffffffff", sorted[2].to_s
  end
end

class TestNamespaceConstants < Minitest::Test
  # ---- Namespace constants ----------------------------------------------------

  def test_namespace_dns_string
    assert_equal "6ba7b810-9dad-11d1-80b4-00c04fd430c8", NAMESPACE_DNS.to_s
  end

  def test_namespace_url_string
    assert_equal "6ba7b811-9dad-11d1-80b4-00c04fd430c8", NAMESPACE_URL.to_s
  end

  def test_namespace_oid_string
    assert_equal "6ba7b812-9dad-11d1-80b4-00c04fd430c8", NAMESPACE_OID.to_s
  end

  def test_namespace_x500_string
    assert_equal "6ba7b814-9dad-11d1-80b4-00c04fd430c8", NAMESPACE_X500.to_s
  end

  def test_namespace_dns_is_uuid
    assert_kind_of UUID, NAMESPACE_DNS
  end
end

class TestUuidV4 < Minitest::Test
  # ---- UUID.v4 ----------------------------------------------------------------

  def test_v4_returns_uuid
    assert_kind_of UUID, Ca::Uuid.v4
  end

  def test_v4_version_is_4
    assert_equal 4, Ca::Uuid.v4.version
  end

  def test_v4_variant_is_rfc4122
    assert_equal "rfc4122", Ca::Uuid.v4.variant
  end

  def test_v4_uniqueness
    # Two independently generated v4 UUIDs should be different.
    # The probability they're equal is 1/2^122 — effectively impossible.
    a = Ca::Uuid.v4
    b = Ca::Uuid.v4
    refute_equal a, b
  end

  def test_v4_to_s_format
    s = Ca::Uuid.v4.to_s
    assert_match(/\A[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/, s)
  end
end

class TestUuidV5 < Minitest::Test
  # ---- UUID.v5: RFC 4122 Appendix B test vector --------------------------------

  def test_v5_rfc_vector_python_org
    # RFC 4122 Appendix B:
    # v5(NAMESPACE_DNS, "python.org") => "886313e1-3b8a-5372-9b90-0c9aee199e5d"
    u = Ca::Uuid.v5(NAMESPACE_DNS, "python.org")
    assert_equal "886313e1-3b8a-5372-9b90-0c9aee199e5d", u.to_s
  end

  def test_v5_deterministic
    a = Ca::Uuid.v5(NAMESPACE_DNS, "example.com")
    b = Ca::Uuid.v5(NAMESPACE_DNS, "example.com")
    assert_equal a, b
  end

  def test_v5_different_names_differ
    a = Ca::Uuid.v5(NAMESPACE_DNS, "example.com")
    b = Ca::Uuid.v5(NAMESPACE_DNS, "example.org")
    refute_equal a, b
  end

  def test_v5_different_namespaces_differ
    a = Ca::Uuid.v5(NAMESPACE_DNS, "example.com")
    b = Ca::Uuid.v5(NAMESPACE_URL, "example.com")
    refute_equal a, b
  end

  def test_v5_version_is_5
    u = Ca::Uuid.v5(NAMESPACE_DNS, "test")
    assert_equal 5, u.version
  end

  def test_v5_variant_is_rfc4122
    u = Ca::Uuid.v5(NAMESPACE_DNS, "test")
    assert_equal "rfc4122", u.variant
  end
end

class TestUuidV3 < Minitest::Test
  # ---- UUID.v3: RFC 4122 test vector ------------------------------------------

  def test_v3_rfc_vector_python_org
    # RFC 4122 Appendix B:
    # v3(NAMESPACE_DNS, "python.org") => "6fa459ea-ee8a-3ca4-894e-db77e160355e"
    u = Ca::Uuid.v3(NAMESPACE_DNS, "python.org")
    assert_equal "6fa459ea-ee8a-3ca4-894e-db77e160355e", u.to_s
  end

  def test_v3_deterministic
    a = Ca::Uuid.v3(NAMESPACE_DNS, "example.com")
    b = Ca::Uuid.v3(NAMESPACE_DNS, "example.com")
    assert_equal a, b
  end

  def test_v3_different_names_differ
    a = Ca::Uuid.v3(NAMESPACE_DNS, "example.com")
    b = Ca::Uuid.v3(NAMESPACE_DNS, "example.org")
    refute_equal a, b
  end

  def test_v3_version_is_3
    u = Ca::Uuid.v3(NAMESPACE_DNS, "test")
    assert_equal 3, u.version
  end

  def test_v3_variant_is_rfc4122
    u = Ca::Uuid.v3(NAMESPACE_DNS, "test")
    assert_equal "rfc4122", u.variant
  end
end

class TestUuidV1 < Minitest::Test
  # ---- UUID.v1 ----------------------------------------------------------------

  def test_v1_returns_uuid
    assert_kind_of UUID, Ca::Uuid.v1
  end

  def test_v1_version_is_1
    assert_equal 1, Ca::Uuid.v1.version
  end

  def test_v1_variant_is_rfc4122
    assert_equal "rfc4122", Ca::Uuid.v1.variant
  end

  def test_v1_uniqueness
    a = Ca::Uuid.v1
    b = Ca::Uuid.v1
    refute_equal a, b
  end
end

class TestUuidV7 < Minitest::Test
  # ---- UUID.v7 ----------------------------------------------------------------

  def test_v7_returns_uuid
    assert_kind_of UUID, Ca::Uuid.v7
  end

  def test_v7_version_is_7
    assert_equal 7, Ca::Uuid.v7.version
  end

  def test_v7_variant_is_rfc4122
    assert_equal "rfc4122", Ca::Uuid.v7.variant
  end

  def test_v7_uniqueness
    a = Ca::Uuid.v7
    b = Ca::Uuid.v7
    refute_equal a, b
  end

  def test_v7_ordering
    # Because v7 embeds a millisecond timestamp in the high bits, UUIDs
    # generated later should sort after those generated earlier.
    # We generate two UUIDs with a small sleep to ensure different timestamps.
    a = Ca::Uuid.v7
    sleep(0.002)  # 2 ms — enough to guarantee a different ms timestamp
    b = Ca::Uuid.v7
    assert a < b, "v7 UUID generated later should sort after earlier one"
  end

  def test_v7_to_s_format
    s = Ca::Uuid.v7.to_s
    # Version nibble must be 7 (third group starts with '7')
    assert_match(/\A[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/, s)
  end
end

class TestUuidModuleMethods < Minitest::Test
  # ---- Ca::Uuid module-level convenience wrappers ----------------------------

  def test_module_parse
    u = Ca::Uuid.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
    assert_kind_of UUID, u
  end

  def test_module_valid_true
    assert Ca::Uuid.valid?("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
  end

  def test_module_valid_false
    refute Ca::Uuid.valid?("garbage")
  end

  def test_module_v4
    assert_kind_of UUID, Ca::Uuid.v4
  end

  def test_module_v1
    assert_kind_of UUID, Ca::Uuid.v1
  end

  def test_module_v7
    assert_kind_of UUID, Ca::Uuid.v7
  end
end

class TestUuidBytes < Minitest::Test
  # ---- UUID bytes methods ----------------------------------------------------

  def test_bytes_returns_array_of_16
    u = UUID.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
    b = u.bytes
    assert_kind_of Array, b
    assert_equal 16, b.length
  end

  def test_bytes_values_in_range
    u = UUID.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
    u.bytes.each do |byte|
      assert byte >= 0 && byte <= 255
    end
  end

  def test_to_i_returns_integer
    u = UUID.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
    assert_kind_of Integer, u.to_i
  end

  def test_to_i_nil_uuid
    assert_equal 0, NIL_UUID.to_i
  end

  def test_to_i_max_uuid
    assert_equal (1 << 128) - 1, MAX_UUID.to_i
  end
end
