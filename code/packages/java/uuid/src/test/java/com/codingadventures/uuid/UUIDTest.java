// ============================================================================
// UUIDTest.java — Unit Tests for UUID
// ============================================================================

package com.codingadventures.uuid;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

import java.util.HashSet;
import java.util.Set;

class UUIDTest {

    // =========================================================================
    // 1. Construction and parsing
    // =========================================================================

    @Test
    void fromStringStandard() {
        UUID u = UUID.fromString("550e8400-e29b-41d4-a716-446655440000");
        assertEquals("550e8400-e29b-41d4-a716-446655440000", u.toString());
    }

    @Test
    void fromStringUppercase() {
        UUID u = UUID.fromString("550E8400-E29B-41D4-A716-446655440000");
        assertEquals("550e8400-e29b-41d4-a716-446655440000", u.toString());
    }

    @Test
    void fromStringCompact() {
        UUID u = UUID.fromString("550e8400e29b41d4a716446655440000");
        assertEquals("550e8400-e29b-41d4-a716-446655440000", u.toString());
    }

    @Test
    void fromStringBraced() {
        UUID u = UUID.fromString("{550e8400-e29b-41d4-a716-446655440000}");
        assertEquals("550e8400-e29b-41d4-a716-446655440000", u.toString());
    }

    @Test
    void fromStringUrn() {
        UUID u = UUID.fromString("urn:uuid:550e8400-e29b-41d4-a716-446655440000");
        assertEquals("550e8400-e29b-41d4-a716-446655440000", u.toString());
    }

    @Test
    void fromStringRejectsInvalid() {
        assertThrows(UUIDException.class, () -> UUID.fromString("not-a-uuid"));
        assertThrows(UUIDException.class, () -> UUID.fromString("550e8400-e29b-41d4-a716-44665544000z"));
        assertThrows(UUIDException.class, () -> UUID.fromString(""));
        assertThrows(UUIDException.class, () -> UUID.fromString(null));
    }

    @Test
    void fromBytesRoundtrip() {
        UUID u = UUID.fromString("550e8400-e29b-41d4-a716-446655440000");
        byte[] b = u.toBytes();
        assertEquals(16, b.length);
        assertEquals(u, UUID.fromBytes(b));
    }

    @Test
    void fromBytesRejectsWrongLength() {
        assertThrows(UUIDException.class, () -> UUID.fromBytes(new byte[15]));
        assertThrows(UUIDException.class, () -> UUID.fromBytes(new byte[17]));
        assertThrows(UUIDException.class, () -> UUID.fromBytes(null));
    }

    // =========================================================================
    // 2. Properties
    // =========================================================================

    @Test
    void versionV4() {
        UUID u = UUID.fromString("550e8400-e29b-41d4-a716-446655440000");
        assertEquals(4, u.version());
    }

    @Test
    void versionV1() {
        UUID u = UUID.fromString("6ba7b810-9dad-11d1-80b4-00c04fd430c8");
        assertEquals(1, u.version());
    }

    @Test
    void versionV5() {
        UUID u = UUID.fromString("886313e1-3b8a-5372-9b90-0c9aee199e5d");
        assertEquals(5, u.version());
    }

    @Test
    void variantRfc4122() {
        // Variant nibble 8, 9, a, b → all RFC 4122
        for (String nibble : new String[]{"8", "9", "a", "b"}) {
            UUID u = UUID.fromString("550e8400-e29b-41d4-" + nibble + "716-446655440000");
            assertEquals("rfc4122", u.variant(),
                "Expected rfc4122 for nibble " + nibble);
        }
    }

    @Test
    void isNil() {
        assertTrue(UUID.NIL.isNil());
        assertFalse(UUID.v4().isNil());
    }

    @Test
    void isMax() {
        assertTrue(UUID.MAX.isMax());
        assertFalse(UUID.v4().isMax());
    }

    @Test
    void nilString() {
        assertEquals("00000000-0000-0000-0000-000000000000", UUID.NIL.toString());
    }

    @Test
    void maxString() {
        assertEquals("ffffffff-ffff-ffff-ffff-ffffffffffff", UUID.MAX.toString());
    }

    // =========================================================================
    // 3. Equality and hashing
    // =========================================================================

    @Test
    void equalityAndHash() {
        UUID a = UUID.fromString("550e8400-e29b-41d4-a716-446655440000");
        UUID b = UUID.fromString("550e8400-e29b-41d4-a716-446655440000");
        assertEquals(a, b);
        assertEquals(a.hashCode(), b.hashCode());
    }

    @Test
    void notEqual() {
        UUID a = UUID.fromString("550e8400-e29b-41d4-a716-446655440000");
        UUID b = UUID.fromString("6ba7b810-9dad-11d1-80b4-00c04fd430c8");
        assertNotEquals(a, b);
    }

    @Test
    void hashSetDeduplicates() {
        UUID a = UUID.fromString("550e8400-e29b-41d4-a716-446655440000");
        UUID b = UUID.fromString("550e8400-e29b-41d4-a716-446655440000");
        Set<UUID> set = new HashSet<>();
        set.add(a);
        set.add(b);
        assertEquals(1, set.size());
    }

    // =========================================================================
    // 4. compareTo
    // =========================================================================

    @Test
    void compareToOrdering() {
        UUID smaller = UUID.fromString("00000000-0000-0000-0000-000000000001");
        UUID larger  = UUID.fromString("00000000-0000-0000-0000-000000000002");
        assertTrue(smaller.compareTo(larger) < 0);
        assertTrue(larger.compareTo(smaller) > 0);
        assertEquals(0, smaller.compareTo(smaller));
    }

    // =========================================================================
    // 5. isValid
    // =========================================================================

    @Test
    void isValidTrue() {
        assertTrue(UUID.isValid("550e8400-e29b-41d4-a716-446655440000"));
        assertTrue(UUID.isValid("550e8400e29b41d4a716446655440000"));
        assertTrue(UUID.isValid("{550e8400-e29b-41d4-a716-446655440000}"));
        assertTrue(UUID.isValid("urn:uuid:550e8400-e29b-41d4-a716-446655440000"));
    }

    @Test
    void isValidFalse() {
        assertFalse(UUID.isValid("not-a-uuid"));
        assertFalse(UUID.isValid(""));
        assertFalse(UUID.isValid(null));
        assertFalse(UUID.isValid("550e8400-e29b-41d4-a716-44665544000z"));
    }

    // =========================================================================
    // 6. UUID v4 — Random
    // =========================================================================

    @Test
    void v4Version() {
        assertEquals(4, UUID.v4().version());
    }

    @Test
    void v4Variant() {
        assertEquals("rfc4122", UUID.v4().variant());
    }

    @Test
    void v4Uniqueness() {
        // Generate 1000 v4 UUIDs; expect no collisions (would be astronomically unlikely)
        Set<UUID> seen = new HashSet<>();
        for (int i = 0; i < 1000; i++) seen.add(UUID.v4());
        assertEquals(1000, seen.size());
    }

    @Test
    void v4Format() {
        // v4: position 13 (0-indexed) = '4', position 17 = 8/9/a/b
        String s = UUID.v4().toString();
        assertEquals('4', s.charAt(14)); // after 8+1+4+1+4+1 = "xxxxxxxx-xxxx-" → char at index 14
        char variant = s.charAt(19);    // after 8+1+4+1+4+1+4+1 = "xxxxxxxx-xxxx-xxxx-" → char at 19
        assertTrue(variant == '8' || variant == '9' || variant == 'a' || variant == 'b',
            "Expected variant nibble 8/9/a/b, got: " + variant);
    }

    // =========================================================================
    // 7. UUID v7 — Time-Ordered Random
    // =========================================================================

    @Test
    void v7Version() {
        assertEquals(7, UUID.v7().version());
    }

    @Test
    void v7Variant() {
        assertEquals("rfc4122", UUID.v7().variant());
    }

    @Test
    void v7Ordering() {
        // v7 UUIDs encode a millisecond timestamp in the high 48 bits.
        // The timestamp portion (first 6 bytes = high 48 bits of msb) must be
        // non-decreasing between two UUIDs generated in sequence. The random
        // suffix within the same millisecond is unordered by design.
        UUID u1 = UUID.v7();
        UUID u2 = UUID.v7();
        // Extract the 48-bit timestamp: high 48 bits of msb = msb >>> 16
        long ts1 = u1.msb >>> 16;
        long ts2 = u2.msb >>> 16;
        assertTrue(Long.compareUnsigned(ts1, ts2) <= 0,
            "v7 timestamps should be non-decreasing; ts1=" + ts1 + " ts2=" + ts2);
    }

    @Test
    void v7Uniqueness() {
        Set<UUID> seen = new HashSet<>();
        for (int i = 0; i < 100; i++) seen.add(UUID.v7());
        assertEquals(100, seen.size());
    }

    // =========================================================================
    // 8. UUID v1 — Time-Based
    // =========================================================================

    @Test
    void v1Version() {
        assertEquals(1, UUID.v1().version());
    }

    @Test
    void v1Variant() {
        assertEquals("rfc4122", UUID.v1().variant());
    }

    @Test
    void v1Uniqueness() {
        Set<UUID> seen = new HashSet<>();
        for (int i = 0; i < 100; i++) seen.add(UUID.v1());
        assertEquals(100, seen.size());
    }

    // =========================================================================
    // 9. UUID v5 — Name-Based (SHA-1)
    // =========================================================================

    @Test
    void v5RfcTestVector() {
        // RFC 4122 test vector: v5(NAMESPACE_DNS, "python.org")
        assertEquals("886313e1-3b8a-5372-9b90-0c9aee199e5d",
            UUID.v5(UUID.NAMESPACE_DNS, "python.org").toString());
    }

    @Test
    void v5Deterministic() {
        UUID a = UUID.v5(UUID.NAMESPACE_DNS, "example.com");
        UUID b = UUID.v5(UUID.NAMESPACE_DNS, "example.com");
        assertEquals(a, b);
    }

    @Test
    void v5DifferentNames() {
        UUID a = UUID.v5(UUID.NAMESPACE_DNS, "example.com");
        UUID b = UUID.v5(UUID.NAMESPACE_DNS, "example.org");
        assertNotEquals(a, b);
    }

    @Test
    void v5DifferentNamespaces() {
        UUID a = UUID.v5(UUID.NAMESPACE_DNS, "example.com");
        UUID b = UUID.v5(UUID.NAMESPACE_URL, "example.com");
        assertNotEquals(a, b);
    }

    @Test
    void v5Version() {
        assertEquals(5, UUID.v5(UUID.NAMESPACE_DNS, "test").version());
    }

    @Test
    void v5Variant() {
        assertEquals("rfc4122", UUID.v5(UUID.NAMESPACE_DNS, "test").variant());
    }

    // =========================================================================
    // 10. UUID v3 — Name-Based (MD5)
    // =========================================================================

    @Test
    void v3RfcTestVector() {
        // RFC 4122 test vector: v3(NAMESPACE_DNS, "python.org")
        assertEquals("6fa459ea-ee8a-3ca4-894e-db77e160355e",
            UUID.v3(UUID.NAMESPACE_DNS, "python.org").toString());
    }

    @Test
    void v3Deterministic() {
        UUID a = UUID.v3(UUID.NAMESPACE_DNS, "example.com");
        UUID b = UUID.v3(UUID.NAMESPACE_DNS, "example.com");
        assertEquals(a, b);
    }

    @Test
    void v3DifferentFromV5() {
        UUID a = UUID.v3(UUID.NAMESPACE_DNS, "python.org");
        UUID b = UUID.v5(UUID.NAMESPACE_DNS, "python.org");
        assertNotEquals(a, b);
    }

    @Test
    void v3Version() {
        assertEquals(3, UUID.v3(UUID.NAMESPACE_DNS, "test").version());
    }

    // =========================================================================
    // 11. Namespace constants
    // =========================================================================

    @Test
    void namespaceDns() {
        assertEquals("6ba7b810-9dad-11d1-80b4-00c04fd430c8",
            UUID.NAMESPACE_DNS.toString());
    }

    @Test
    void namespaceUrl() {
        assertEquals("6ba7b811-9dad-11d1-80b4-00c04fd430c8",
            UUID.NAMESPACE_URL.toString());
    }

    @Test
    void namespaceOid() {
        assertEquals("6ba7b812-9dad-11d1-80b4-00c04fd430c8",
            UUID.NAMESPACE_OID.toString());
    }

    @Test
    void namespaceX500() {
        assertEquals("6ba7b814-9dad-11d1-80b4-00c04fd430c8",
            UUID.NAMESPACE_X500.toString());
    }
}
