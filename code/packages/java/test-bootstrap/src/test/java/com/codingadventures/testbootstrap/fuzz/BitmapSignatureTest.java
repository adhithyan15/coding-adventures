package com.codingadventures.testbootstrap.fuzz;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.codingadventures.testbootstrap.prng.SplitMix64;
import java.util.ArrayList;
import java.util.List;
import org.junit.jupiter.api.Test;

/** Bitmap, admission, energy, failure ids, signatures and the TAP line. */
class BitmapSignatureTest {
  // ---- bitmap ---------------------------------------------------------------------

  @Test
  void bitmapRecordsMasksAndCounts() {
    Bitmap b = new Bitmap();
    assertEquals(0, b.count());
    b.record(3);
    b.record(3);
    b.record(65_536 + 3); // masked to 3
    b.record(-1L); // unsigned 2^64-1 mod 65536 = 65535
    assertTrue(b.has(3));
    assertTrue(b.has(65_535));
    assertFalse(b.has(4));
    assertEquals(2, b.count());
    assertArrayEquals(new int[] {3, 65_535}, b.edges());
    byte[] raw = b.raw();
    assertEquals(Bitmap.COVERAGE_BYTES, raw.length);
    assertEquals(0b1000, raw[0]);
    assertEquals((byte) 0x80, raw[8191]);
    raw[0] = 0; // a copy
    assertEquals(0b1000, b.raw()[0]);
    b.reset();
    assertEquals(0, b.count());
    assertFalse(b.has(3));
    assertEquals(0, b.edges().length);
  }

  @Test
  void bitmapGrowsPastItsInitialEdgeBuffer() {
    Bitmap b = new Bitmap();
    for (int i = 0; i < 1000; i += 3) {
      b.record(i);
    }
    assertEquals(334, b.count());
    assertEquals(334, b.trace().length);
    Bitmap c = new Bitmap();
    c.record(5000);
    c.merge(b);
    assertEquals(335, c.count());
  }

  @Test
  void admitCountsUniqueNovelEdges() {
    Bitmap b = new Bitmap();
    Bitmap.Admission a = b.admit(new int[] {1, 2, 2, 3});
    assertTrue(a.hasNew());
    assertEquals(3, a.countNew());
    a = b.admit(new int[] {1, 2});
    assertFalse(a.hasNew());
    assertEquals(0, a.countNew());
    a = b.admit(new int[] {3, 9, 9});
    assertEquals(1, a.countNew());
  }

  @Test
  void corpusAdmitCountsUniqueNovelEdgesOverFullWords() {
    Bitmap.Novelty n = Bitmap.corpusAdmit(new long[] {1, 2, 3}, new long[] {2, 4});
    assertTrue(n.admitted());
    assertEquals(1, n.newEdges());
    n = Bitmap.corpusAdmit(new long[] {1, 2, 3}, new long[] {1, 2});
    assertFalse(n.admitted());
    assertEquals(0, n.newEdges());
    n = Bitmap.corpusAdmit(new long[] {1}, new long[] {7, 7, 7, 1});
    assertEquals(1, n.newEdges()); // a repeated novel edge counts once
    n = Bitmap.corpusAdmit(new long[] {1}, new long[] {1L + 65_536, -1L});
    assertEquals(2, n.newEdges(), "admission compares full words, it does not reduce mod 65536");
    assertFalse(Bitmap.corpusAdmit(new long[0], new long[0]).admitted());
  }

  @Test
  void energySchedule() {
    assertEquals(8, Bitmap.calculateEnergy(10, 4, 3, 6)); // 7.5 rounds up
    assertEquals(3, Bitmap.calculateEnergy(5, 4, 0, 0)); // 2.5 rounds away from zero, not to even
    assertEquals(10, Bitmap.calculateEnergy(10, 1, 5, 0)); // no edges: bonus 1.0
    assertEquals(1, Bitmap.calculateEnergy(0, 1_000_000, 1, 1));
    assertEquals(20, Bitmap.calculateEnergy(10, 0, 1, 1)); // zero length counts as 1
    assertEquals(Mutate.MAX_ITERATIONS, Bitmap.calculateEnergy(Integer.MAX_VALUE, 1, 1, 1));
    assertTrue(Bitmap.calculateEnergy(10, 4, 6, 6) > Bitmap.calculateEnergy(10, 4, 0, 6));
    assertTrue(Bitmap.calculateEnergy(10, 100, 1, 2) < Bitmap.calculateEnergy(10, 1, 1, 2));
  }

  // ---- signatures -------------------------------------------------------------------

  @Test
  void signaturesArePinnedAgainstTheReference() {
    assertEquals("4eca7ae8", Signature.crashSignature("panic", List.of("eval", "step")));
    assertEquals("4e61ea76", Signature.crashSignature("timeout", List.of()));
    assertEquals("7146a410", Signature.crashSignature("oom", List.of()));
    assertEquals("557f548e", Signature.crashSignature("x", List.of("a:b")));
    assertEquals("557f548e", Signature.crashSignature("x", List.of("a", "b"))); // the documented, unescaped ambiguity
    // The definition: low 32 bits of fnv1a64(kind ":" frames).
    assertEquals(
        String.format("%08x", SplitMix64.fnv1a64("panic:eval:step") & 0xFFFFFFFFL), Signature.crashSignature("panic", List.of("eval", "step")));
  }

  @Test
  void dedupSeparatesKindsFramesAndDepth() {
    assertTrue(Signature.isDuplicateCrash("panic", List.of("eval", "step"), "panic", List.of("eval", "step")));
    assertFalse(Signature.isDuplicateCrash("panic", List.of("eval", "step"), "segv", List.of("alloc", "write")));
    assertFalse(Signature.isDuplicateCrash("panic", List.of("a"), "panic", List.of("a", "b")));
    assertFalse(Signature.isDuplicateCrash("panic", List.of("a", "b"), "panic", List.of("b", "a")));
  }

  @Test
  void failureVocabularyIsClosed() {
    assertEquals(List.of("fuzz-crash", "fuzz-timeout", "fuzz-oom", "fuzz-leak", "fuzz-panic"), FailureIds.ALL);
    for (String id : FailureIds.ALL) {
      assertTrue(FailureIds.isFuzzFailureId(id));
    }
    for (String bad : new String[] {"fuzz-arbitrary-error", "", "FUZZ-CRASH", "fuzz-crash ", "property-panic", "fuzz-crash\n"}) {
      assertFalse(FailureIds.isFuzzFailureId(bad), bad);
    }
    assertFalse(FailureIds.isFuzzFailureId(null));
  }

  // ---- TAP ----------------------------------------------------------------------------

  @Test
  void tapLineIsOpaqueAndExact() {
    assertEquals("not ok 1 - ct_compare [fuzz-crash:3a8f19b2]", Signature.formatTapFailure(1, "ct_compare", "fuzz-crash", "3a8f19b2"));
    assertEquals("not ok 42 - a/b.c-d_e [fuzz-oom:00000000]", Signature.formatTapFailure(42, "a/b.c-d_e", "fuzz-oom", "00000000"));
  }

  @Test
  void tapLineRefusesUnregisteredIdsAndBadHashes() {
    assertThrows(IllegalArgumentException.class, () -> Signature.formatTapFailure(1, "t", "fuzz-bogus", "3a8f19b2"));
    assertThrows(IllegalArgumentException.class, () -> Signature.formatTapFailure(1, "t", null, "3a8f19b2"));
    for (String bad : new String[] {"3A8F19B2", "3a8f19b", "3a8f19b22", "", "3a8f19b\n", "zzzzzzzz", "3a8f19b2\n"}) {
      assertThrows(IllegalArgumentException.class, () -> Signature.formatTapFailure(1, "t", "fuzz-crash", bad), bad);
    }
    assertThrows(IllegalArgumentException.class, () -> Signature.formatTapFailure(1, "t", "fuzz-crash", null));
  }

  @Test
  void tapTargetCannotForgeALine() {
    String line = Signature.formatTapFailure(1, "evil\nok 2 - forged [x]\r\n", "fuzz-crash", "3a8f19b2");
    assertFalse(line.contains("\n"));
    assertFalse(line.contains("\r"));
    assertTrue(line.startsWith("not ok 1 - evil_ok_2_-_forged__x___"));
    assertEquals(1, line.lines().count());
    // Multi-byte UTF-8 is replaced byte for byte.
    assertEquals("not ok 1 - a__b [fuzz-crash:3a8f19b2]", Signature.formatTapFailure(1, "aéb", "fuzz-crash", "3a8f19b2"));
    // Truncation is in bytes: 128 of them.
    String longLine = Signature.formatTapFailure(1, "x".repeat(500), "fuzz-crash", "3a8f19b2");
    assertEquals("not ok 1 - " + "x".repeat(128) + " [fuzz-crash:3a8f19b2]", longLine);
    String multi = Signature.formatTapFailure(1, "é".repeat(100), "fuzz-crash", "3a8f19b2");
    assertEquals("not ok 1 - " + "_".repeat(128) + " [fuzz-crash:3a8f19b2]", multi);
    assertEquals("not ok 1 -  [fuzz-crash:3a8f19b2]", Signature.formatTapFailure(1, "", "fuzz-crash", "3a8f19b2"));
  }

  @Test
  void tapFailuresRenderEveryBucketWithoutPayload() {
    CrashRecord a = new CrashRecord("aaaaaaaa", "k", List.of("f"), new byte[] {(byte) 0xde, (byte) 0xad}, true, false);
    CrashRecord b = new CrashRecord("bbbbbbbb", "k", List.of("g"), new byte[] {1}, true, false);
    Report r = new Report(5, 1, 1, List.of(a, b), 2, 3, 0, 0, Report.TIER_REPORTED, 7, List.of(new byte[] {1}));
    List<String> lines = Signature.tapFailures(r, "tgt", 10);
    assertEquals(
        List.of(
            "not ok 10 - tgt [fuzz-crash:aaaaaaaa]",
            "not ok 11 - tgt [fuzz-crash:bbbbbbbb]",
            "not ok 12 - tgt [fuzz-timeout:" + Signature.crashSignature("timeout", List.of()) + "]",
            "not ok 13 - tgt [fuzz-oom:" + Signature.crashSignature("oom", List.of()) + "]"),
        lines);
    for (String l : lines) {
      assertFalse(l.contains("dead"));
    }
    Report clean = new Report(5, 1, 1, List.of(), 0, 0, 0, 0, Report.TIER_REPORTED, 7, List.of());
    assertEquals(List.of(), Signature.tapFailures(clean, "tgt", 1));
  }

  @Test
  void tapFailuresAlwaysEmitANotOkLineEvenForAMalformedSignature() {
    CrashRecord bad = new CrashRecord("NOT-HEX!", "k", List.of("f"), new byte[0], true, false);
    Report r = new Report(1, 1, 1, List.of(bad), 0, 0, 0, 0, Report.TIER_BLACK_BOX, 1, List.of());
    List<String> lines = Signature.tapFailures(r, "t", 1);
    assertEquals(1, lines.size());
    assertEquals("not ok 1 - t [fuzz-crash:" + Signature.crashSignature("invalid", List.of()) + "]", lines.get(0));
  }

  // ---- frame normalisation ---------------------------------------------------------------

  @Test
  void normaliseFrameKeepsClassAndMethodOnly() {
    assertEquals("Parser$Inner.read", Signature.normaliseFrame("com.acme.Parser$Inner", "read"));
    assertEquals("App.run", Signature.normaliseFrame("com.acme.App$$Lambda/0x0000000800c03000", "run"));
    assertEquals("$Proxy.call", Signature.normaliseFrame("jdk.proxy2.$Proxy12", "call"));
    assertEquals("Main.<init>", Signature.normaliseFrame("Main", "<init>"));
    assertEquals("Main.lambda$main$0", Signature.normaliseFrame("a.b.Main", "lambda$main$0"));
    assertEquals("Odd_Class.m_x", Signature.normaliseFrame("p.Odd:Class", "m:x"));
    assertFalse(Signature.normaliseFrame("a:b.C", "d:e").contains(":"));
    assertEquals("A__.m", Signature.normaliseFrame("p.Aé", "m"));
  }

  @Test
  void normaliseKindAndCollapse() {
    assertEquals("java.lang.IllegalStateException", Signature.normaliseKind("java.lang.IllegalStateException"));
    assertEquals("a_b", Signature.normaliseKind("a b"));
    assertEquals(64, Signature.normaliseKind("x".repeat(200)).length());
    assertEquals(List.of("a", "b", "a"), Signature.collapse(List.of("a", "a", "b", "b", "b", "a")));
    List<String> many = new ArrayList<>();
    for (int i = 0; i < 100; i++) {
      many.add("f" + i);
    }
    List<String> kept = Signature.collapse(many);
    assertEquals(64, kept.size());
    assertEquals("f0", kept.get(0));
    assertEquals("f63", kept.get(63));
  }
}
