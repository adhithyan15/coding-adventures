package com.codingadventures.testbootstrap.tfv;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class TfvTest {
  private static String doc(String record) {
    return "tfv 1\n" + record + "\n";
  }

  @Test
  void acceptsWellFormedDocuments() {
    String[] good = {
      "tfv 1\n",
      "tfv 1",
      "tfv 1\n# a comment\n\n",
      doc("a.b/c-d_e R0"),
      doc("r R1 Kop S706274"),
      doc("r R2 Kn N Kt T"),
      doc("r R1 Kf F"),
      doc("r R1 Ki I0"),
      doc("r R1 Ki I-7"),
      doc("r R1 Ki I" + "9".repeat(1000)),
      doc("r R1 Kd D3ff0000000000000"),
      doc("r R1 Kd D7ff8000000000000"),
      doc("r R1 Kd D7ff0000000000000"),
      doc("r R1 Kx X"),
      doc("r R1 Kx X00ff"),
      doc("r R1 Ks S68c3a9"),
      doc("r R1 Kl L0"),
      doc("r R1 Kl L2 I1 L1 N"),
      doc("r R1 Km M0"),
      doc("r R1 Km M2 I1 I2 I2 I3"),
      "tfv 1\nr R0\ns R0\n",
    };
    for (String g : good) {
      Tfv.validate(g);
      assertTrue(Tfv.isValid(g));
    }
  }

  @Test
  void rejectsMalformedDocuments() {
    String d200 = "L1 ".repeat(100);
    String deepOk = "r R1 Kx " + "L1 ".repeat(63) + "N";
    String deepBad = "r R1 Kx " + "L1 ".repeat(65) + "N";
    String[] bad = {
      "", "\n", "bad header\n", "tfv 2\n", "tfv  1\n", "TFV 1\n", " tfv 1\n",
      doc("r R1 Kop S706274 Kdata XFF"), // the corpus case: uppercase / odd hex
      doc("r"), doc(" r R0"), doc("r R0 "), doc("r  R0"), doc("r R1"), doc("r R2 Ka N"), doc("r R0 N"),
      doc("r R01"), doc("r RX"), doc("r R"), doc("r R9999999999"), doc("r S1"), doc("bad!id R0"), doc("R0"),
      doc("r R2 Ka N Ka N"), doc("r R1 Ba N"), doc("r R1 K N"), doc("r R1 KA N"), doc("r R1 K1a N"),
      doc("r R1 Ki I"), doc("r R1 Ki I-"), doc("r R1 Ki I-0"), doc("r R1 Ki I007"), doc("r R1 Ki I1a"), doc("r R1 Ki I+1"),
      doc("r R1 Ki I" + "9".repeat(1001)),
      doc("r R1 Kd D3ff000000000000"), doc("r R1 Kd D3FF0000000000000"), doc("r R1 Kd Dzz"),
      doc("r R1 Kn Nx"), doc("r R1 Kn Tx"), doc("r R1 Kn Fx"),
      doc("r R1 Kx Xf"), doc("r R1 Kx XFF"), doc("r R1 Kx Xgg"),
      doc("r R1 Ks Sf"), doc("r R1 Ks Sff"), doc("r R1 Ks Sc3"), doc("r R1 Ks SZZ"),
      doc("r R1 Kl L"), doc("r R1 Kl L01"), doc("r R1 Kl L2 N"), doc("r R1 Kl Lx"),
      doc("r R1 Km M1 N"), doc("r R1 Km M2 I1 I2 I1 I3"), doc("r R1 Km M1 I1"),
      doc("r R1 Kz Q"),
      doc("r R1 Ka N extra"),
      doc("r R0\r"),
      doc("r R1 Ka N\r"),
      "tfv 1\nr R0\nr R0\n",
      doc("r R1 Kx " + d200 + "N"),
      doc(deepBad),
      doc("r R1 Km M1 L1 N N") , // a (list) key is fine, but this one is incomplete? covered below
      "tfv 1\n\uD800\n",
      "x".repeat(Tfv.MAX_INPUT + 1),
    };
    int rejected = 0;
    for (String b : bad) {
      if (b.equals(doc("r R1 Km M1 L1 N N"))) {
        continue; // well-formed: map with list key [N] -> N
      }
      assertFalse(Tfv.isValid(b), "accepted: " + b.substring(0, Math.min(60, b.length())));
      assertThrows(Tfv.InvalidTfvException.class, () -> Tfv.validate(b));
      rejected++;
    }
    assertTrue(rejected > 60);
    assertTrue(Tfv.isValid(doc(deepOk)));
    assertTrue(Tfv.isValid(doc("r R1 Km M1 L1 N N")));
  }

  @Test
  void errorsNameTheLineAndClipTokens() {
    String huge = "I" + "9".repeat(5000);
    Tfv.InvalidTfvException e = assertThrows(Tfv.InvalidTfvException.class, () -> Tfv.validate("tfv 1\nr R1 Ki " + huge + "\n"));
    assertTrue(e.getMessage().contains("line 2"));
    assertTrue(e.getMessage().length() < 300);
    Tfv.InvalidTfvException tok = assertThrows(Tfv.InvalidTfvException.class, () -> Tfv.validate("tfv 1\nr R1 Kx X" + "G".repeat(5000) + "\n"));
    assertTrue(tok.getMessage().length() < 300);
    Tfv.InvalidTfvException dup = assertThrows(Tfv.InvalidTfvException.class, () -> Tfv.validate("tfv 1\nr R0\nr R0\n"));
    assertTrue(dup.getMessage().contains("duplicate record id"));
    assertTrue(assertThrows(Tfv.InvalidTfvException.class, () -> Tfv.validate("bad header\n")).getMessage().contains("first line"));
  }

  @Test
  void countsDoNotReserveMemory() {
    // A count of 999,999,999 with no tokens present fails fast ("ends early").
    assertFalse(Tfv.isValid(doc("r R1 Kl L999999999")));
    assertFalse(Tfv.isValid(doc("r R1 Km M999999999")));
  }
}
