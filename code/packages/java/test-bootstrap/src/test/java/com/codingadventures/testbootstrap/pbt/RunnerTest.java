package com.codingadventures.testbootstrap.pbt;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.codingadventures.testbootstrap.corpusjson.Canon;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

/** Verdict honesty, CI redaction and the failure vocabulary (VER05e section 5). */
class RunnerTest {
  private static final long SECRET_SEED = 987654321987L;

  private static Options local() {
    return new Options(100, 1000, CiMode.OFF);
  }

  private static Options ci() {
    return new Options(100, 1000, CiMode.ON);
  }

  @Test
  void aHoldingPropertyPasses() {
    Verdict v = Runner.check(1, Gens.int64(0, 100), x -> x >= 0 && x <= 100, null);
    assertTrue(v.passed());
    assertFalse(v.failed());
    assertEquals(100, v.casesRun());
    assertNull(v.failureId());
    v.assertPassed();
  }

  @Test
  void aFalsifiablePropertyIsNeverAPass() {
    // x < 90 is false for roughly one case in ten; 100 cases must find it.
    for (long seed = 0; seed < 50; seed++) {
      Verdict v = Runner.check(seed, Gens.int64(0, 99), x -> x < 90, null);
      assertFalse(v.passed());
      assertEquals(FailureIds.COUNTEREXAMPLE, v.failureId());
      AssertionError e = assertThrows(AssertionError.class, v::assertPassed);
      assertTrue(e.getMessage().startsWith("[property-counterexample]"));
    }
  }

  @Test
  void zeroCasesIsNotAPass() {
    Verdict v = Runner.check(1, Gens.int64(0, 1), x -> true, new Options(0, 10, CiMode.ON));
    assertFalse(v.passed());
    assertEquals(FailureIds.EXHAUSTED, v.failureId());
    assertEquals(0, v.casesRun());
    assertTrue(v.message().contains("never ran"));
  }

  @Test
  void invalidOptionsProduceNoVerdictAtAll() {
    Gen<Long> g = Gens.int64(0, 1);
    assertThrows(IllegalArgumentException.class, () -> Runner.check(1, g, x -> true, new Options(-1, 10, CiMode.ON)));
    assertThrows(IllegalArgumentException.class, () -> Runner.check(1, g, x -> true, new Options(Limits.MAX_CASES + 1, 10, CiMode.ON)));
    assertThrows(IllegalArgumentException.class, () -> Runner.check(1, g, x -> true, new Options(1, -1, CiMode.ON)));
    assertThrows(IllegalArgumentException.class, () -> Runner.check(1, g, x -> true, new Options(1, Limits.MAX_SHRINK_STEPS + 1, CiMode.ON)));
    assertThrows(IllegalArgumentException.class, () -> Runner.check(1, g, null, null));
    assertThrows(IllegalArgumentException.class, () -> Runner.check(1, null, x -> true, null));
  }

  @Test
  void aPropertyThatThrowsIsAPanicVerdict() {
    Verdict v = Runner.check(1, Gens.int64(0, 9), x -> { throw new IllegalStateException("boom-" + SECRET_SEED); }, local());
    assertEquals(FailureIds.PANIC, v.failureId());
    assertTrue(v.message().contains("boom"));
    assertEquals(1, v.iteration());
    Verdict redacted = Runner.check(1, Gens.int64(0, 9), x -> { throw new IllegalStateException("boom-" + SECRET_SEED); }, ci());
    assertEquals(FailureIds.PANIC, redacted.failureId());
    assertFalse(redacted.message().contains("boom"));
    assertTrue(redacted.message().contains("IllegalStateException"));
    assertNull(redacted.seed());

    Verdict assertion = Runner.check(1, Gens.int64(0, 9), x -> { throw new AssertionError("nope"); }, local());
    assertEquals(FailureIds.PANIC, assertion.failureId());
    Verdict so = Runner.check(1, Gens.int64(0, 9), x -> { throw new StackOverflowError(); }, local());
    assertEquals(FailureIds.PANIC, so.failureId());
  }

  @Test
  void generatorFailuresAreExhausted() {
    Gen<Long> filtered = Gens.filter(Gens.int64(0, 5), x -> false, 3);
    Verdict v = Runner.check(1, filtered, x -> true, local());
    assertEquals(FailureIds.EXHAUSTED, v.failureId());
    assertTrue(v.message().contains("filter precondition"));
    assertEquals(0, v.casesRun());

    Gen<Long> boom = Gen.of(rng -> { throw new IllegalStateException("gen-secret"); }, null);
    Verdict plain = Runner.check(1, boom, x -> true, local());
    assertEquals(FailureIds.EXHAUSTED, plain.failureId());
    assertTrue(plain.message().contains("gen-secret"));
    Verdict redacted = Runner.check(1, boom, x -> true, ci());
    assertFalse(redacted.message().contains("gen-secret"));
    assertTrue(redacted.message().contains("IllegalStateException"));
  }

  @Test
  void shrinkingFindsTheMinimalWitness() {
    Verdict v = Runner.check(7, Gens.int64(0, 100000), x -> x < 500, local());
    assertEquals(FailureIds.COUNTEREXAMPLE, v.failureId());
    assertEquals(500L, v.minimal());
    assertNotNull(v.counterexample());
    assertEquals(7L, v.seed());
    assertTrue(v.shrinkSteps() > 0 || ((Long) v.counterexample()) == 500L);

    Verdict lists = Runner.check(3, Gens.list(Gens.integer(BigInteger.ZERO, BigInteger.valueOf(50)), 0, 10),
        l -> l.stream().reduce(BigInteger.ZERO, BigInteger::add).compareTo(BigInteger.valueOf(20)) <= 0, local());
    @SuppressWarnings("unchecked")
    List<BigInteger> m = (List<BigInteger>) lists.minimal();
    assertEquals(BigInteger.valueOf(21), m.stream().reduce(BigInteger.ZERO, BigInteger::add), "sum lands exactly on the threshold");
  }

  @Test
  void aLyingShrinkerCannotChangeTheVerdict() {
    Gen<Long> g = Gens.int64(0, 1000);
    // Returns a value that does NOT falsify: the original must be reported, 0 steps.
    Verdict lie = Runner.check(1, g, x -> x < 500, local(), (value, falsifies, max) -> new Shrunk<>(0L, 3));
    assertEquals(FailureIds.COUNTEREXAMPLE, lie.failureId());
    assertEquals(0, lie.shrinkSteps());
    assertEquals(lie.counterexample(), lie.minimal());
    // Out-of-range step count.
    Verdict steps = Runner.check(1, g, x -> x < 500, local(), (value, falsifies, max) -> new Shrunk<>(value, max + 1));
    assertEquals(0, steps.shrinkSteps());
    Verdict negSteps = Runner.check(1, g, x -> x < 500, local(), (value, falsifies, max) -> new Shrunk<>(value, -1));
    assertEquals(0, negSteps.shrinkSteps());
    // A shrinker that throws.
    Verdict boom = Runner.check(1, g, x -> x < 500, local(), (value, falsifies, max) -> { throw new IllegalStateException(); });
    assertEquals(FailureIds.COUNTEREXAMPLE, boom.failureId());
    assertEquals(0, boom.shrinkSteps());
    // An honest explicit shrinker is used.
    Verdict honest = Runner.check(1, g, x -> x < 500, local(), (value, falsifies, max) -> new Shrunk<>(value, 0));
    assertEquals(0, honest.shrinkSteps());
  }

  @Test
  void aPropertyThatThrowsOnTheShrunkValueReportsTheOriginal() {
    // Fails (returns false) for x >= 500 but throws for x == 500 exactly: a throwing
    // candidate is not "falsifying", so the walk must not accept it.
    Verdict v = Runner.check(7, Gens.int64(0, 100000), x -> {
      if (x == 500) {
        throw new IllegalStateException();
      }
      return x < 500;
    }, local());
    assertEquals(FailureIds.COUNTEREXAMPLE, v.failureId());
    assertTrue((Long) v.minimal() > 500);
  }

  @Test
  void ciMessagesCarryNoWitnessAndNoSeed() {
    Gen<Long> g = Gens.int64(0, 1_000_000);
    Verdict v = Runner.check(SECRET_SEED, g, x -> x < 777_777, ci());
    assertEquals(FailureIds.COUNTEREXAMPLE, v.failureId());
    assertFalse(v.message().contains("minimal="));
    assertFalse(v.message().contains("initial="));
    assertFalse(v.message().contains("seed="));
    assertFalse(v.message().contains(Long.toString(SECRET_SEED)));
    assertNull(v.seed());
    assertNull(v.counterexample());
    assertNull(v.minimal());
    assertNotNull(v.digest());
    assertEquals(16, v.digest().length());
    assertTrue(v.message().contains("repro-fnv: " + v.digest()));
    // The default (unset) is CI too: fail closed.
    Verdict unset = Runner.check(SECRET_SEED, g, x -> x < 777_777, new Options(100, 1000, CiMode.UNSET));
    assertNull(unset.minimal());
    assertEquals(v.digest(), unset.digest());
    Verdict defaults = Runner.check(SECRET_SEED, g, x -> x < 777_777, null);
    assertNull(defaults.seed());
    // The local message does carry them.
    Verdict l = Runner.check(SECRET_SEED, g, x -> x < 777_777, local());
    assertTrue(l.message().contains("minimal=" + l.minimal()));
    assertTrue(l.message().contains("initial=" + l.counterexample()));
    assertTrue(l.message().contains("seed="));
    assertTrue(l.message().contains(Long.toString(SECRET_SEED)));
    assertEquals(SECRET_SEED, l.seed());
  }

  @Test
  void ciFromLookup() {
    for (String on : new String[] {"1", "true", "yes", "TRUE", " 1 ", "anything"}) {
      assertTrue(Options.ciFromLookup(k -> on), on);
    }
    for (String off : new String[] {"", "0", "false", "FALSE", "  ", " False "}) {
      assertFalse(Options.ciFromLookup(k -> off), off);
    }
    assertFalse(Options.ciFromLookup(k -> null));
    Map<String, String> env = new HashMap<>(Map.of("CI", "1"));
    assertTrue(Options.ciFromLookup(env::get));
    assertEquals(CiMode.OFF, Options.defaults().withCi(CiMode.OFF).ci());
    assertEquals(CiMode.UNSET, Options.defaults().ci());
  }

  @Test
  void failureVocabularyIsClosed() {
    assertEquals(
        List.of("property-counterexample", "property-exhausted", "property-timeout", "property-panic"), FailureIds.ALL);
    for (String id : FailureIds.ALL) {
      assertTrue(FailureIds.isPbtFailureId(id));
    }
    for (String no : new String[] {"", "unregistered-id", "Property-Panic", "property-panic ", "property-"}) {
      assertFalse(FailureIds.isPbtFailureId(no), no);
    }
    assertFalse(FailureIds.isPbtFailureId(null));
  }

  @Test
  void theVerdictIsDeterministicForASeed() {
    for (long seed = 0; seed < 20; seed++) {
      Verdict a = Runner.check(seed, Gens.int64(0, 1000), x -> x < 400, local());
      Verdict b = Runner.check(seed, Gens.int64(0, 1000), x -> x < 400, local());
      assertEquals(a, b);
    }
  }

  // ---- Canon -----------------------------------------------------------------

  @Test
  void canonRendersCommonShapes() {
    assertEquals("true", Canon.render(true));
    assertEquals("-12", Canon.render(BigInteger.valueOf(-12)));
    assertEquals("7", Canon.render(7));
    assertEquals("0x00ff", Canon.render(new byte[] {0, -1}));
    assertEquals("0x", Canon.render(new byte[0]));
    assertEquals("[1,2]", Canon.render(List.of(1, 2)));
    assertEquals("[1,2]", Canon.render(new int[] {1, 2}));
    assertEquals("[\"a\",nil]", Canon.render(new Object[] {"a", null}));
    assertEquals("nil", Canon.render(null));
    assertEquals("{1,[2]}", Canon.render(new Gens.Pair<>(1, List.of(2))));
    assertEquals("\"a\\\"b\\n\\\\\\t\\r\\u0001\"", Canon.render("a\"b\n\\\t\r\u0001"));
    Map<String, Integer> m = new java.util.LinkedHashMap<>();
    m.put("b", 2);
    m.put("a", 1);
    assertEquals("{\"a\":1,\"b\":2}", Canon.render(m), "map keys render sorted");
    assertEquals("1.5", Canon.render(1.5));
    assertTrue(Canon.render(new Object() {
      @Override public String toString() { return "custom"; }
    }).contains("custom"));
  }

  @Test
  void canonIsTotalForHostileWitnesses() {
    // Depth.
    Object deep = new ArrayList<>();
    Object cur = deep;
    for (int i = 0; i < 10_000; i++) {
      List<Object> next = new ArrayList<>();
      @SuppressWarnings("unchecked")
      List<Object> c = (List<Object>) cur;
      c.add(next);
      cur = next;
    }
    String text = Canon.render(deep);
    assertTrue(text.contains("..."));
    assertTrue(text.length() < 200);
    // Width: more than the node budget truncates.
    List<Integer> wide = new ArrayList<>();
    for (int i = 0; i < 50_000; i++) {
      wide.add(i);
    }
    String w = Canon.render(wide);
    assertTrue(w.endsWith("...]"));
    assertTrue(w.length() < 100_000);
    // A hostile toString.
    Object evil = new Object() {
      @Override public String toString() { throw new IllegalStateException("evil"); }
    };
    assertTrue(Canon.render(evil).contains("<"));
    Object so = new Object() {
      @Override public String toString() { throw new StackOverflowError(); }
    };
    assertTrue(Canon.render(so).contains("<"));
    // Maps, records and arrays at depth, and a huge byte array.
    Map<Object, Object> cyc = new HashMap<>();
    cyc.put("self", cyc);
    assertTrue(Canon.render(cyc).contains("..."));
    assertTrue(Canon.render(new byte[50_000]).endsWith("..."));
    Object[] arr = new Object[1];
    arr[0] = arr;
    assertTrue(Canon.render(arr).contains("..."));
    Gens.Pair<Object, Object> nested = new Gens.Pair<>(null, null);
    for (int i = 0; i < 100; i++) {
      nested = new Gens.Pair<>(nested, nested);
    }
    assertTrue(Canon.render(nested).length() < 400_000);
    Map<Object, Object> bigMap = new HashMap<>();
    for (int i = 0; i < 20_000; i++) {
      bigMap.put(i, i);
    }
    assertTrue(Canon.render(bigMap).length() < 400_000);
  }

  @Test
  void checkReturnsAVerdictForAHostileWitness() {
    Gen<Object> deep = Gen.of(rng -> {
      List<Object> top = new ArrayList<>();
      List<Object> cur = top;
      for (int i = 0; i < 100_000; i++) {
        List<Object> next = new ArrayList<>();
        cur.add(next);
        cur = next;
      }
      return top;
    }, null);
    Verdict v = Runner.check(1, deep, x -> false, ci());
    assertEquals(FailureIds.COUNTEREXAMPLE, v.failureId());
    assertNotNull(v.digest());
    Gen<Object> evil = Gen.of(rng -> new Object() {
      @Override public String toString() { throw new IllegalStateException(); }
    }, null);
    Verdict e = Runner.check(1, evil, x -> false, local());
    assertEquals(FailureIds.COUNTEREXAMPLE, e.failureId());
    assertEquals("a", Runner.safeString("a"));
    assertTrue(Runner.safeString(new Object() {
      @Override public String toString() { throw new IllegalStateException(); }
    }).length() > 0);
  }

  // Security regression (review item 3): Canon documents totality but a hostile
  // iterator / entrySet / CharSequence used to escape it, so redacted Runner.check threw.
  @Test
  void canonAndRunnerSurviveThrowingContainers() {
    List<Integer> badIterator = new java.util.AbstractList<>() {
      @Override public Integer get(int i) { throw new IllegalStateException("get"); }
      @Override public int size() { return 3; }
      @Override public java.util.Iterator<Integer> iterator() { throw new IllegalStateException("iter"); }
    };
    Map<String, String> badMap = new java.util.AbstractMap<>() {
      @Override public java.util.Set<Map.Entry<String, String>> entrySet() { throw new IllegalStateException("entries"); }
    };
    CharSequence badSeq = new CharSequence() {
      @Override public int length() { return 1; }
      @Override public char charAt(int i) { return 'x'; }
      @Override public CharSequence subSequence(int a, int b) { return this; }
      @Override public String toString() { throw new IllegalStateException("seq"); }
    };
    List<Object> overflow = new java.util.AbstractList<>() {
      @Override public Object get(int i) { return 0; }
      @Override public int size() { return 1; }
      @Override public java.util.Iterator<Object> iterator() { throw new StackOverflowError(); }
    };
    for (Object hostile : new Object[] {badIterator, badMap, badSeq, overflow, List.of(badIterator)}) {
      assertEquals(Canon.UNRENDERABLE, Canon.render(hostile));
      for (CiMode mode : new CiMode[] {CiMode.ON, CiMode.OFF}) {
        Gen<Object> g = Gen.of(rng -> hostile, null);
        Verdict v = Runner.check(1, g, x -> false, new Options(5, 100, mode));
        assertEquals(FailureIds.COUNTEREXAMPLE, v.failureId());
        if (mode == CiMode.ON) {
          assertNotNull(v.digest());
        }
      }
    }
    // Digest of an unrenderable witness is the constant-preimage digest and is deterministic.
    Gen<Object> g = Gen.of(rng -> badIterator, null);
    assertEquals(
        Runner.check(1, g, x -> false, ci()).digest(), Runner.check(9, g, x -> false, ci()).digest());
  }

  // Re-review 3: AssertionError / LinkageError from a hostile toString or iterator used to
  // escape Canon and make Runner.check throw.
  @Test
  void canonAndRunnerSurviveErrorsOtherThanStackOverflow() {
    Object assertionToString = new Object() {
      @Override public String toString() { throw new AssertionError("evil"); }
    };
    Object linkageToString = new Object() {
      @Override public String toString() { throw new LinkageError("evil"); }
    };
    List<Integer> initError = new java.util.AbstractList<>() {
      @Override public Integer get(int i) { return 0; }
      @Override public int size() { return 1; }
      @Override public java.util.Iterator<Integer> iterator() { throw new ExceptionInInitializerError("evil"); }
    };
    Map<String, String> assertionMap = new java.util.AbstractMap<>() {
      @Override public java.util.Set<Map.Entry<String, String>> entrySet() { throw new AssertionError("evil"); }
    };
    for (Object hostile : new Object[] {assertionToString, linkageToString, initError, assertionMap}) {
      String text = Canon.render(hostile);
      assertTrue(text.contains("<"), text);
      for (CiMode mode : new CiMode[] {CiMode.ON, CiMode.OFF}) {
        Gen<Object> g = Gen.of(rng -> hostile, null);
        Verdict v = Runner.check(1, g, x -> false, new Options(3, 50, mode));
        assertEquals(FailureIds.COUNTEREXAMPLE, v.failureId());
      }
    }
    assertTrue(Runner.safeString(assertionToString).length() > 0);
    // A shrinker that throws an Error-family AssertionError cannot make check throw either.
    Verdict v = Runner.check(1, Gens.int64(0, 9), x -> false, local(), (value, falsifies, max) -> { throw new AssertionError("bad shrinker"); });
    assertEquals(FailureIds.COUNTEREXAMPLE, v.failureId());
    assertEquals(0, v.shrinkSteps());
  }
}
