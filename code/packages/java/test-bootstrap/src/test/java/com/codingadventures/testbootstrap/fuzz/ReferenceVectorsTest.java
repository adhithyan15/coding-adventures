package com.codingadventures.testbootstrap.fuzz;

import static com.codingadventures.testbootstrap.fuzz.Fx.bg;
import static com.codingadventures.testbootstrap.fuzz.Fx.endian;
import static com.codingadventures.testbootstrap.fuzz.Fx.hx;
import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;

import java.io.IOException;
import java.math.BigInteger;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import org.junit.jupiter.api.Test;

/**
 * The 820 differential vectors of the Go lane, computed by the independent Python
 * oracle {@code scripts/fuzz_conformance.py} (generator:
 * {@code go/test-bootstrap/fuzz/testdata/gen_reference_vectors.py}). The Go table is a
 * committed source file, so it is read from its committed path; there is no copy.
 *
 * <p>The fuzz-v1 corpus alone cannot pin these (VER06f section 9 item 9): its splice has
 * equal splits and its mutate_sequence case does not pin every constant.
 */
class ReferenceVectorsTest {
  record Vector(String op, List<String> args, String out) {}

  private static final Pattern ROW = Pattern.compile("^\\t\\{\"([a-z_]+)\", \\[\\]string\\{(.*)\\}, \"([0-9a-f]*)\"\\},$");
  private static final Pattern ARG = Pattern.compile("\"([^\"]*)\"");

  static List<Vector> load() throws IOException {
    Path file = Fx.repoRoot().resolve("code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go");
    List<String> lines = Files.readAllLines(file);
    int start = lines.indexOf("var referenceVectors = []refVector{");
    assertTrue(start >= 0, "the vector table moved");
    List<Vector> out = new ArrayList<>();
    for (int i = start + 1; i < lines.size() && !lines.get(i).equals("}"); i++) {
      Matcher m = ROW.matcher(lines.get(i));
      if (!m.matches()) {
        fail("unparseable vector row " + (i + 1) + ": " + lines.get(i));
      }
      List<String> args = new ArrayList<>();
      Matcher a = ARG.matcher(m.group(2));
      while (a.find()) {
        args.add(a.group(1));
      }
      out.add(new Vector(m.group(1), args, m.group(3)));
    }
    return out;
  }

  private static boolean fitsLong(String... decimals) {
    for (String d : decimals) {
      if (bg(d).bitLength() > 63) {
        return false;
      }
    }
    return true;
  }

  @Test
  void allVectorsMatchBothTheBigAndTheLongEntryPoints() throws IOException {
    List<Vector> vectors = load();
    assertEquals(820, vectors.size(), "the table must hold all 820 oracle vectors");
    Map<String, Integer> counts = new TreeMap<>();
    int i = -1;
    for (Vector v : vectors) {
      i++;
      List<String> a = v.args();
      byte[] want = hx(v.out());
      byte[] got;
      byte[] small = null;
      counts.merge(v.op(), 1, Integer::sum);
      switch (v.op()) {
        case "bit_flip" -> {
          got = Mutate.bitFlipBig(hx(a.get(0)), bg(a.get(1)), bg(a.get(2)));
          if (fitsLong(a.get(1), a.get(2))) {
            small = Mutate.bitFlip(hx(a.get(0)), Long.parseLong(a.get(1)), Long.parseLong(a.get(2)));
          }
        }
        case "byte_flip" -> {
          got = Mutate.byteFlipBig(hx(a.get(0)), bg(a.get(1)), bg(a.get(2)));
          if (fitsLong(a.get(1), a.get(2))) {
            small = Mutate.byteFlip(hx(a.get(0)), Long.parseLong(a.get(1)), Long.parseLong(a.get(2)));
          }
        }
        case "arithmetic" -> {
          int w = Integer.parseInt(a.get(3));
          got = Mutate.byteArithmeticBig(hx(a.get(0)), bg(a.get(1)), bg(a.get(2)), w, endian(a.get(4)));
          if (fitsLong(a.get(1), a.get(2))) {
            small = Mutate.byteArithmetic(hx(a.get(0)), Long.parseLong(a.get(1)), Long.parseLong(a.get(2)), w, endian(a.get(4)));
          }
        }
        case "boundary" -> {
          int w = Integer.parseInt(a.get(2));
          got = Mutate.boundaryInsertBig(hx(a.get(0)), bg(a.get(1)), w, bg(a.get(3)), endian(a.get(4)));
          BigInteger val = bg(a.get(3));
          if (fitsLong(a.get(1)) && val.signum() >= 0 && val.bitLength() <= 64) {
            small = Mutate.boundaryInsert(hx(a.get(0)), Long.parseLong(a.get(1)), w, val.longValue(), endian(a.get(4)));
          }
        }
        case "block_delete" -> {
          got = Mutate.blockDeleteBig(hx(a.get(0)), bg(a.get(1)), bg(a.get(2)));
          if (fitsLong(a.get(1), a.get(2))) {
            small = Mutate.blockDelete(hx(a.get(0)), Long.parseLong(a.get(1)), Long.parseLong(a.get(2)));
          }
        }
        case "block_insert" -> {
          got = Mutate.blockInsertBig(hx(a.get(0)), bg(a.get(1)), hx(a.get(2)));
          if (fitsLong(a.get(1))) {
            small = Mutate.blockInsert(hx(a.get(0)), Long.parseLong(a.get(1)), hx(a.get(2)));
          }
        }
        case "block_duplicate" -> {
          got = Mutate.blockDuplicateBig(hx(a.get(0)), bg(a.get(1)), bg(a.get(2)), bg(a.get(3)));
          if (fitsLong(a.get(1), a.get(2), a.get(3))) {
            small = Mutate.blockDuplicate(hx(a.get(0)), Long.parseLong(a.get(1)), Long.parseLong(a.get(2)), Long.parseLong(a.get(3)));
          }
        }
        case "splice" -> {
          got = Mutate.spliceBig(hx(a.get(0)), hx(a.get(1)), bg(a.get(2)), bg(a.get(3)));
          if (fitsLong(a.get(2), a.get(3))) {
            small = Mutate.splice(hx(a.get(0)), hx(a.get(1)), Long.parseLong(a.get(2)), Long.parseLong(a.get(3)));
          }
        }
        case "sequence" ->
            got = Mutate.mutateSequence(hx(a.get(0)), bg(a.get(1)).longValue(), Integer.parseInt(a.get(2)), Integer.parseInt(a.get(3)));
        default -> {
          fail("unknown vector op " + v.op());
          return;
        }
      }
      assertArrayEquals(want, got, "vector " + i + " " + v.op() + " " + a);
      if (small != null) {
        assertArrayEquals(want, small, "vector " + i + " " + v.op() + " " + a + " (long path)");
      }
    }
    for (String op : List.of("bit_flip", "byte_flip", "arithmetic", "boundary", "block_delete", "block_insert", "block_duplicate", "splice", "sequence")) {
      assertTrue(counts.getOrDefault(op, 0) >= 50, "only " + counts.get(op) + " reference vectors for " + op);
    }
  }
}
