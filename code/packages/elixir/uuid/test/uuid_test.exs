defmodule CodingAdventures.UuidTest do
  use ExUnit.Case
  alias CodingAdventures.Uuid

  # ─── Module Loading ────────────────────────────────────────────────────────
  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.Uuid)
  end

  # ─── Nil and Max UUIDs ────────────────────────────────────────────────────
  test "nil_uuid is 16 zero bytes" do
    assert byte_size(Uuid.nil_uuid()) == 16
    assert Uuid.nil_uuid() == <<0::128>>
  end

  test "max_uuid is 16 0xFF bytes" do
    assert byte_size(Uuid.max_uuid()) == 16
    assert Uuid.max_uuid() == <<0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                                 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF>>
  end

  test "to_string of nil_uuid" do
    assert Uuid.to_string(Uuid.nil_uuid()) == "00000000-0000-0000-0000-000000000000"
  end

  test "to_string of max_uuid" do
    assert Uuid.to_string(Uuid.max_uuid()) == "ffffffff-ffff-ffff-ffff-ffffffffffff"
  end

  test "is_nil_uuid detects nil" do
    assert Uuid.is_nil_uuid(Uuid.nil_uuid()) == true
    assert Uuid.is_nil_uuid(Uuid.v4()) == false
  end

  test "is_max_uuid detects max" do
    assert Uuid.is_max_uuid(Uuid.max_uuid()) == true
    assert Uuid.is_max_uuid(Uuid.nil_uuid()) == false
  end

  # ─── Namespace Constants ──────────────────────────────────────────────────
  test "namespace_dns has correct value" do
    assert Uuid.to_string(Uuid.namespace_dns()) == "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
  end

  test "namespace_url has correct value" do
    assert Uuid.to_string(Uuid.namespace_url()) == "6ba7b811-9dad-11d1-80b4-00c04fd430c8"
  end

  test "namespace_oid has correct value" do
    assert Uuid.to_string(Uuid.namespace_oid()) == "6ba7b812-9dad-11d1-80b4-00c04fd430c8"
  end

  test "namespace_x500 has correct value" do
    assert Uuid.to_string(Uuid.namespace_x500()) == "6ba7b814-9dad-11d1-80b4-00c04fd430c8"
  end

  test "namespace constants are 16 bytes each" do
    assert byte_size(Uuid.namespace_dns())  == 16
    assert byte_size(Uuid.namespace_url())  == 16
    assert byte_size(Uuid.namespace_oid())  == 16
    assert byte_size(Uuid.namespace_x500()) == 16
  end

  # ─── Parse ────────────────────────────────────────────────────────────────
  test "parse standard hyphenated UUID" do
    {:ok, uuid} = Uuid.parse("550e8400-e29b-41d4-a716-446655440000")
    assert byte_size(uuid) == 16
  end

  test "parse round-trips with to_string" do
    s = "550e8400-e29b-41d4-a716-446655440000"
    {:ok, uuid} = Uuid.parse(s)
    assert Uuid.to_string(uuid) == s
  end

  test "parse is case insensitive" do
    {:ok, lower} = Uuid.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
    {:ok, upper} = Uuid.parse("6BA7B810-9DAD-11D1-80B4-00C04FD430C8")
    assert lower == upper
  end

  test "parse braces format" do
    {:ok, uuid} = Uuid.parse("{6ba7b810-9dad-11d1-80b4-00c04fd430c8}")
    assert Uuid.to_string(uuid) == "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
  end

  test "parse URN format" do
    {:ok, uuid} = Uuid.parse("urn:uuid:6ba7b810-9dad-11d1-80b4-00c04fd430c8")
    assert Uuid.to_string(uuid) == "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
  end

  test "parse compact format (no hyphens)" do
    {:ok, uuid} = Uuid.parse("6ba7b8109dad11d180b400c04fd430c8")
    assert Uuid.to_string(uuid) == "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
  end

  test "parse returns error on invalid input" do
    assert {:error, _} = Uuid.parse("not-a-uuid")
    assert {:error, _} = Uuid.parse("ZZZZZZZZ-ZZZZ-ZZZZ-ZZZZ-ZZZZZZZZZZZZ")
    assert {:error, _} = Uuid.parse("")
    assert {:error, _} = Uuid.parse("too-short")
  end

  # ─── Version and Variant ──────────────────────────────────────────────────
  test "version/1 extracts correct version from known UUID" do
    # 6ba7b810-9dad-11d1-80b4-00c04fd430c8 — version 1
    {:ok, uuid} = Uuid.parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
    assert Uuid.version(uuid) == 1
  end

  test "variant/1 returns rfc4122 for namespace_dns" do
    assert Uuid.variant(Uuid.namespace_dns()) == "rfc4122"
  end

  # ─── UUID v4 ──────────────────────────────────────────────────────────────
  test "v4 produces 16-byte binary" do
    uuid = Uuid.v4()
    assert byte_size(uuid) == 16
  end

  test "v4 version is 4" do
    assert Uuid.version(Uuid.v4()) == 4
  end

  test "v4 variant is rfc4122" do
    assert Uuid.variant(Uuid.v4()) == "rfc4122"
  end

  test "v4 produces unique UUIDs" do
    a = Uuid.v4()
    b = Uuid.v4()
    assert a != b
  end

  test "v4 formats as valid UUID string" do
    s = Uuid.to_string(Uuid.v4())
    assert String.match?(s, ~r/\A[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/)
  end

  # ─── UUID v5 — RFC Vectors ─────────────────────────────────────────────────
  test "v5 RFC vector: dns + python.org" do
    result = Uuid.to_string(Uuid.v5(Uuid.namespace_dns(), "python.org"))
    assert result == "886313e1-3b8a-5372-9b90-0c9aee199e5d"
  end

  test "v5 RFC vector: url + http://example.com/" do
    result = Uuid.to_string(Uuid.v5(Uuid.namespace_url(), "http://example.com/"))
    # Verified against Python uuid module: uuid.uuid5(uuid.NAMESPACE_URL, "http://example.com/")
    assert result == "0a300ee9-f9e4-5697-a51a-efc7fafaba67"
  end

  test "v5 version is 5" do
    assert Uuid.version(Uuid.v5(Uuid.namespace_dns(), "python.org")) == 5
  end

  test "v5 variant is rfc4122" do
    assert Uuid.variant(Uuid.v5(Uuid.namespace_dns(), "python.org")) == "rfc4122"
  end

  test "v5 is deterministic" do
    a = Uuid.v5(Uuid.namespace_dns(), "python.org")
    b = Uuid.v5(Uuid.namespace_dns(), "python.org")
    assert a == b
  end

  test "v5 differs by name" do
    a = Uuid.v5(Uuid.namespace_dns(), "python.org")
    b = Uuid.v5(Uuid.namespace_dns(), "python.com")
    assert a != b
  end

  test "v5 differs by namespace" do
    a = Uuid.v5(Uuid.namespace_dns(), "python.org")
    b = Uuid.v5(Uuid.namespace_url(), "python.org")
    assert a != b
  end

  # ─── UUID v3 — RFC Vectors ─────────────────────────────────────────────────
  test "v3 RFC vector: dns + python.org" do
    result = Uuid.to_string(Uuid.v3(Uuid.namespace_dns(), "python.org"))
    assert result == "6fa459ea-ee8a-3ca4-894e-db77e160355e"
  end

  test "v3 version is 3" do
    assert Uuid.version(Uuid.v3(Uuid.namespace_dns(), "python.org")) == 3
  end

  test "v3 variant is rfc4122" do
    assert Uuid.variant(Uuid.v3(Uuid.namespace_dns(), "python.org")) == "rfc4122"
  end

  test "v3 is deterministic" do
    a = Uuid.v3(Uuid.namespace_dns(), "python.org")
    b = Uuid.v3(Uuid.namespace_dns(), "python.org")
    assert a == b
  end

  # ─── UUID v1 ──────────────────────────────────────────────────────────────
  test "v1 produces 16-byte binary" do
    assert byte_size(Uuid.v1()) == 16
  end

  test "v1 version is 1" do
    assert Uuid.version(Uuid.v1()) == 1
  end

  test "v1 variant is rfc4122" do
    assert Uuid.variant(Uuid.v1()) == "rfc4122"
  end

  test "v1 produces unique UUIDs" do
    a = Uuid.v1()
    b = Uuid.v1()
    # Very unlikely to collide — different random clocks/nodes
    assert a != b
  end

  # ─── UUID v7 ──────────────────────────────────────────────────────────────
  test "v7 produces 16-byte binary" do
    assert byte_size(Uuid.v7()) == 16
  end

  test "v7 version is 7" do
    assert Uuid.version(Uuid.v7()) == 7
  end

  test "v7 variant is rfc4122" do
    assert Uuid.variant(Uuid.v7()) == "rfc4122"
  end

  test "v7 produces unique UUIDs" do
    a = Uuid.v7()
    b = Uuid.v7()
    assert a != b
  end

  test "v7 is time-ordered (later UUID sorts lexicographically after earlier)" do
    a = Uuid.v7()
    # Small sleep ensures different millisecond timestamp
    :timer.sleep(2)
    b = Uuid.v7()
    assert Uuid.to_string(a) < Uuid.to_string(b)
  end

  test "v7 timestamp matches current time within 5 seconds" do
    ts_before = System.os_time(:millisecond)
    uuid = Uuid.v7()
    ts_after = System.os_time(:millisecond)

    # Extract the 48-bit millisecond timestamp from bytes 0–5
    <<ts_ms::big-48, _::binary>> = uuid
    assert ts_ms >= ts_before
    assert ts_ms <= ts_after + 5000
  end
end
