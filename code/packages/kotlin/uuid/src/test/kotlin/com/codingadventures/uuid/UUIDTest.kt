// ============================================================================
// UUIDTest.kt — Unit Tests for UUID
// ============================================================================

package com.codingadventures.uuid

import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class UUIDTest {

    // =========================================================================
    // 1. Construction and parsing
    // =========================================================================

    @Test
    fun fromStringStandard() {
        val u = UUID.fromString("550e8400-e29b-41d4-a716-446655440000")
        assertEquals("550e8400-e29b-41d4-a716-446655440000", u.toString())
    }

    @Test
    fun fromStringUppercase() {
        val u = UUID.fromString("550E8400-E29B-41D4-A716-446655440000")
        assertEquals("550e8400-e29b-41d4-a716-446655440000", u.toString())
    }

    @Test
    fun fromStringCompact() {
        val u = UUID.fromString("550e8400e29b41d4a716446655440000")
        assertEquals("550e8400-e29b-41d4-a716-446655440000", u.toString())
    }

    @Test
    fun fromStringBraced() {
        val u = UUID.fromString("{550e8400-e29b-41d4-a716-446655440000}")
        assertEquals("550e8400-e29b-41d4-a716-446655440000", u.toString())
    }

    @Test
    fun fromStringUrn() {
        val u = UUID.fromString("urn:uuid:550e8400-e29b-41d4-a716-446655440000")
        assertEquals("550e8400-e29b-41d4-a716-446655440000", u.toString())
    }

    @Test
    fun fromStringRejectsInvalid() {
        assertFailsWith<UUIDException> { UUID.fromString("not-a-uuid") }
        assertFailsWith<UUIDException> { UUID.fromString("550e8400-e29b-41d4-a716-44665544000z") }
        assertFailsWith<UUIDException> { UUID.fromString("") }
        assertFailsWith<UUIDException> { UUID.fromString(null) }
    }

    @Test
    fun fromBytesRoundtrip() {
        val u = UUID.fromString("550e8400-e29b-41d4-a716-446655440000")
        val b = u.toBytes()
        assertEquals(16, b.size)
        assertEquals(u, UUID.fromBytes(b))
    }

    @Test
    fun fromBytesRejectsWrongLength() {
        assertFailsWith<UUIDException> { UUID.fromBytes(ByteArray(15)) }
        assertFailsWith<UUIDException> { UUID.fromBytes(ByteArray(17)) }
        assertFailsWith<UUIDException> { UUID.fromBytes(null) }
    }

    // =========================================================================
    // 2. Properties
    // =========================================================================

    @Test
    fun versionV4() {
        assertEquals(4, UUID.fromString("550e8400-e29b-41d4-a716-446655440000").version)
    }

    @Test
    fun versionV1() {
        assertEquals(1, UUID.fromString("6ba7b810-9dad-11d1-80b4-00c04fd430c8").version)
    }

    @Test
    fun versionV5() {
        assertEquals(5, UUID.fromString("886313e1-3b8a-5372-9b90-0c9aee199e5d").version)
    }

    @Test
    fun variantRfc4122() {
        for (nibble in listOf("8", "9", "a", "b")) {
            val u = UUID.fromString("550e8400-e29b-41d4-${nibble}716-446655440000")
            assertEquals("rfc4122", u.variant, "Expected rfc4122 for nibble $nibble")
        }
    }

    @Test
    fun isNil() {
        assertTrue(UUID.NIL.isNil)
        assertFalse(UUID.v4().isNil)
    }

    @Test
    fun isMax() {
        assertTrue(UUID.MAX.isMax)
        assertFalse(UUID.v4().isMax)
    }

    @Test
    fun nilString() {
        assertEquals("00000000-0000-0000-0000-000000000000", UUID.NIL.toString())
    }

    @Test
    fun maxString() {
        assertEquals("ffffffff-ffff-ffff-ffff-ffffffffffff", UUID.MAX.toString())
    }

    // =========================================================================
    // 3. Equality and hashing (data class)
    // =========================================================================

    @Test
    fun equalityAndHash() {
        val a = UUID.fromString("550e8400-e29b-41d4-a716-446655440000")
        val b = UUID.fromString("550e8400-e29b-41d4-a716-446655440000")
        assertEquals(a, b)
        assertEquals(a.hashCode(), b.hashCode())
    }

    @Test
    fun notEqual() {
        val a = UUID.fromString("550e8400-e29b-41d4-a716-446655440000")
        val b = UUID.fromString("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
        assertNotEquals(a, b)
    }

    @Test
    fun hashSetDeduplicates() {
        val a = UUID.fromString("550e8400-e29b-41d4-a716-446655440000")
        val b = UUID.fromString("550e8400-e29b-41d4-a716-446655440000")
        val set = setOf(a, b)
        assertEquals(1, set.size)
    }

    // =========================================================================
    // 4. compareTo
    // =========================================================================

    @Test
    fun compareToOrdering() {
        val smaller = UUID.fromString("00000000-0000-0000-0000-000000000001")
        val larger  = UUID.fromString("00000000-0000-0000-0000-000000000002")
        assertTrue(smaller < larger)
        assertTrue(larger > smaller)
        assertEquals(0, smaller.compareTo(smaller))
    }

    // =========================================================================
    // 5. isValid
    // =========================================================================

    @Test
    fun isValidTrue() {
        assertTrue(UUID.isValid("550e8400-e29b-41d4-a716-446655440000"))
        assertTrue(UUID.isValid("550e8400e29b41d4a716446655440000"))
        assertTrue(UUID.isValid("{550e8400-e29b-41d4-a716-446655440000}"))
        assertTrue(UUID.isValid("urn:uuid:550e8400-e29b-41d4-a716-446655440000"))
    }

    @Test
    fun isValidFalse() {
        assertFalse(UUID.isValid("not-a-uuid"))
        assertFalse(UUID.isValid(""))
        assertFalse(UUID.isValid(null))
        assertFalse(UUID.isValid("550e8400-e29b-41d4-a716-44665544000z"))
    }

    // =========================================================================
    // 6. UUID v4 — Random
    // =========================================================================

    @Test
    fun v4Version() {
        assertEquals(4, UUID.v4().version)
    }

    @Test
    fun v4Variant() {
        assertEquals("rfc4122", UUID.v4().variant)
    }

    @Test
    fun v4Uniqueness() {
        val seen = (1..1000).map { UUID.v4() }.toSet()
        assertEquals(1000, seen.size)
    }

    @Test
    fun v4Format() {
        val s = UUID.v4().toString()
        assertEquals('4', s[14])
        val variantChar = s[19]
        assertTrue(variantChar in setOf('8', '9', 'a', 'b'),
            "Expected variant nibble 8/9/a/b, got: $variantChar")
    }

    // =========================================================================
    // 7. UUID v7 — Time-Ordered Random
    // =========================================================================

    @Test
    fun v7Version() {
        assertEquals(7, UUID.v7().version)
    }

    @Test
    fun v7Variant() {
        assertEquals("rfc4122", UUID.v7().variant)
    }

    @Test
    fun v7Ordering() {
        // The 48-bit timestamp (high 48 bits of msb = msb ushr 16) must be
        // non-decreasing. Within the same millisecond, random suffix can differ.
        val u1 = UUID.v7()
        val u2 = UUID.v7()
        val ts1 = u1.msb ushr 16
        val ts2 = u2.msb ushr 16
        assertTrue(java.lang.Long.compareUnsigned(ts1, ts2) <= 0,
            "v7 timestamps should be non-decreasing; ts1=$ts1 ts2=$ts2")
    }

    @Test
    fun v7Uniqueness() {
        val seen = (1..100).map { UUID.v7() }.toSet()
        assertEquals(100, seen.size)
    }

    // =========================================================================
    // 8. UUID v1 — Time-Based
    // =========================================================================

    @Test
    fun v1Version() {
        assertEquals(1, UUID.v1().version)
    }

    @Test
    fun v1Variant() {
        assertEquals("rfc4122", UUID.v1().variant)
    }

    @Test
    fun v1Uniqueness() {
        val seen = (1..100).map { UUID.v1() }.toSet()
        assertEquals(100, seen.size)
    }

    // =========================================================================
    // 9. UUID v5 — Name-Based (SHA-1)
    // =========================================================================

    @Test
    fun v5RfcTestVector() {
        assertEquals(
            "886313e1-3b8a-5372-9b90-0c9aee199e5d",
            UUID.v5(UUID.NAMESPACE_DNS, "python.org").toString(),
        )
    }

    @Test
    fun v5Deterministic() {
        val a = UUID.v5(UUID.NAMESPACE_DNS, "example.com")
        val b = UUID.v5(UUID.NAMESPACE_DNS, "example.com")
        assertEquals(a, b)
    }

    @Test
    fun v5DifferentNames() {
        assertNotEquals(
            UUID.v5(UUID.NAMESPACE_DNS, "example.com"),
            UUID.v5(UUID.NAMESPACE_DNS, "example.org"),
        )
    }

    @Test
    fun v5DifferentNamespaces() {
        assertNotEquals(
            UUID.v5(UUID.NAMESPACE_DNS, "example.com"),
            UUID.v5(UUID.NAMESPACE_URL, "example.com"),
        )
    }

    @Test
    fun v5Version() {
        assertEquals(5, UUID.v5(UUID.NAMESPACE_DNS, "test").version)
    }

    @Test
    fun v5Variant() {
        assertEquals("rfc4122", UUID.v5(UUID.NAMESPACE_DNS, "test").variant)
    }

    // =========================================================================
    // 10. UUID v3 — Name-Based (MD5)
    // =========================================================================

    @Test
    fun v3RfcTestVector() {
        assertEquals(
            "6fa459ea-ee8a-3ca4-894e-db77e160355e",
            UUID.v3(UUID.NAMESPACE_DNS, "python.org").toString(),
        )
    }

    @Test
    fun v3Deterministic() {
        val a = UUID.v3(UUID.NAMESPACE_DNS, "example.com")
        val b = UUID.v3(UUID.NAMESPACE_DNS, "example.com")
        assertEquals(a, b)
    }

    @Test
    fun v3DifferentFromV5() {
        assertNotEquals(
            UUID.v3(UUID.NAMESPACE_DNS, "python.org"),
            UUID.v5(UUID.NAMESPACE_DNS, "python.org"),
        )
    }

    @Test
    fun v3Version() {
        assertEquals(3, UUID.v3(UUID.NAMESPACE_DNS, "test").version)
    }

    // =========================================================================
    // 11. Namespace constants
    // =========================================================================

    @Test
    fun namespaceDns() {
        assertEquals("6ba7b810-9dad-11d1-80b4-00c04fd430c8", UUID.NAMESPACE_DNS.toString())
    }

    @Test
    fun namespaceUrl() {
        assertEquals("6ba7b811-9dad-11d1-80b4-00c04fd430c8", UUID.NAMESPACE_URL.toString())
    }

    @Test
    fun namespaceOid() {
        assertEquals("6ba7b812-9dad-11d1-80b4-00c04fd430c8", UUID.NAMESPACE_OID.toString())
    }

    @Test
    fun namespaceX500() {
        assertEquals("6ba7b814-9dad-11d1-80b4-00c04fd430c8", UUID.NAMESPACE_X500.toString())
    }

    // =========================================================================
    // 12. data class behaviour
    // =========================================================================

    @Test
    fun dataClassCopy() {
        val u = UUID.fromString("550e8400-e29b-41d4-a716-446655440000")
        val copy = u.copy()
        assertEquals(u, copy)
    }

    @Test
    fun dataClassDestructure() {
        val u = UUID.fromString("550e8400-e29b-41d4-a716-446655440000")
        val (m, l) = u
        assertEquals(u.msb, m)
        assertEquals(u.lsb, l)
    }
}
