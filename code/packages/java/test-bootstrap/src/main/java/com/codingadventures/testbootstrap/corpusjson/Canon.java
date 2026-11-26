package com.codingadventures.testbootstrap.corpusjson;

import java.lang.reflect.Array;
import java.lang.reflect.RecordComponent;
import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import java.util.Map;

/**
 * Renders a witness as stable text, for failure messages and the CI digest.
 *
 * <p>It is <em>total</em>: it never throws, never recurses without bound and
 * never reads the whole of a hostile value. Nesting deeper than 16 renders as
 * {@code ...}, a witness with more than 10,000 nodes truncates with {@code ...},
 * and a value whose {@code toString} throws renders as its class name. This
 * matters because redacted mode computes the digest outside any catch: a custom
 * generator can build arbitrarily deep values, and a runner that crashes on one
 * never returns a verdict.
 *
 * <pre>
 *   render(true)                      -> true
 *   render(BigInteger.valueOf(-12))   -> -12
 *   render(new byte[] {0, -1})        -> 0x00ff
 *   render(List.of(1, 2))             -> [1,2]
 *   render(new Pair(1, 2))            -> {1,2}          (records render as {fields})
 *   render(null)                      -> nil
 * </pre>
 */
public final class Canon {
  /** Container nesting bound. */
  public static final int MAX_DEPTH = 16;

  /** Total values rendered. */
  public static final int MAX_NODES = 10_000;

  /** Longest rendered text of an arbitrary object's toString. */
  private static final int MAX_TO_STRING = 200;

  private Canon() {}

  /** What a witness renders as when rendering itself throws. */
  public static final String UNRENDERABLE = "<unrenderable>";

  /** Stable text for {@code v}. Never throws. */
  public static String render(Object v) {
    StringBuilder b = new StringBuilder();
    int[] nodes = {0};
    try {
      render(b, v, 0, nodes);
    } catch (RuntimeException | StackOverflowError | AssertionError | LinkageError e) {
      // A hostile iterator, entrySet, CharSequence or array accessor: the
      // contract is totality, so the whole witness degrades to a constant.
      return UNRENDERABLE;
    }
    return b.toString();
  }

  private static void render(StringBuilder b, Object v, int depth, int[] nodes) {
    nodes[0]++;
    if (nodes[0] > MAX_NODES) {
      b.append("...");
      return;
    }
    if (v == null) {
      b.append("nil");
    } else if (v instanceof Boolean
        || v instanceof Integer
        || v instanceof Long
        || v instanceof Short
        || v instanceof Byte
        || v instanceof java.math.BigInteger
        || v instanceof Double
        || v instanceof Float) {
      b.append(v);
    } else if (v instanceof CharSequence cs) {
      quote(b, cs.toString());
    } else if (v instanceof byte[] raw) {
      b.append("0x");
      for (byte x : raw) {
        b.append(Character.forDigit((x >> 4) & 0xf, 16)).append(Character.forDigit(x & 0xf, 16));
        if (b.length() > 4 * MAX_NODES) {
          b.append("...");
          break;
        }
      }
    } else if (v.getClass().isArray()) {
      sequence(b, arrayView(v), depth, nodes);
    } else if (v instanceof Collection<?> c) {
      sequence(b, c, depth, nodes);
    } else if (v instanceof Map<?, ?> m) {
      map(b, m, depth, nodes);
    } else if (v instanceof Record r) {
      record(b, r, depth, nodes);
    } else {
      other(b, v);
    }
  }

  private static List<Object> arrayView(Object array) {
    int n = Math.min(Array.getLength(array), MAX_NODES + 1);
    List<Object> out = new ArrayList<>(n);
    for (int i = 0; i < n; i++) {
      out.add(Array.get(array, i));
    }
    return out;
  }

  private static void sequence(StringBuilder b, Collection<?> c, int depth, int[] nodes) {
    if (depth >= MAX_DEPTH) {
      b.append("...");
      return;
    }
    b.append('[');
    boolean first = true;
    for (Object item : c) {
      if (!first) {
        b.append(',');
      }
      first = false;
      if (nodes[0] >= MAX_NODES) { // out of budget: stop, do not walk the rest
        b.append("...");
        break;
      }
      render(b, item, depth + 1, nodes);
    }
    b.append(']');
  }

  private static void map(StringBuilder b, Map<?, ?> m, int depth, int[] nodes) {
    if (depth >= MAX_DEPTH) {
      b.append("...");
      return;
    }
    List<String[]> entries = new ArrayList<>();
    for (Map.Entry<?, ?> e : m.entrySet()) {
      StringBuilder kb = new StringBuilder();
      StringBuilder vb = new StringBuilder();
      render(kb, e.getKey(), depth + 1, nodes);
      render(vb, e.getValue(), depth + 1, nodes);
      entries.add(new String[] {kb.toString(), vb.toString()});
      if (nodes[0] >= MAX_NODES) {
        break;
      }
    }
    entries.sort((x, y) -> x[0].compareTo(y[0]));
    b.append('{');
    for (int i = 0; i < entries.size(); i++) {
      if (i > 0) {
        b.append(',');
      }
      b.append(entries.get(i)[0]).append(':').append(entries.get(i)[1]);
    }
    b.append('}');
  }

  private static void record(StringBuilder b, Record r, int depth, int[] nodes) {
    if (depth >= MAX_DEPTH) {
      b.append("...");
      return;
    }
    b.append('{');
    boolean first = true;
    try {
      for (RecordComponent rc : r.getClass().getRecordComponents()) {
        if (!first) {
          b.append(',');
        }
        first = false;
        if (nodes[0] >= MAX_NODES) {
          b.append("...");
          break;
        }
        Object field;
        try {
          var accessor = rc.getAccessor();
          accessor.setAccessible(true);
          field = accessor.invoke(r);
        } catch (ReflectiveOperationException | RuntimeException | AssertionError | LinkageError e) {
          field = "<" + rc.getName() + ">";
        }
        render(b, field, depth + 1, nodes);
      }
    } catch (RuntimeException | AssertionError | LinkageError e) {
      b.append("<").append(r.getClass().getSimpleName()).append(">");
    }
    b.append('}');
  }

  private static void other(StringBuilder b, Object v) {
    String text;
    try {
      text = String.valueOf(v);
    } catch (RuntimeException | StackOverflowError | AssertionError | LinkageError e) {
      text = "<" + v.getClass().getName() + ">";
    }
    quote(b, Clip.clip(text, MAX_TO_STRING));
  }

  private static void quote(StringBuilder b, String s) {
    b.append('"');
    for (int i = 0; i < s.length(); i++) {
      char c = s.charAt(i);
      switch (c) {
        case '"' -> b.append("\\\"");
        case '\\' -> b.append("\\\\");
        case '\n' -> b.append("\\n");
        case '\r' -> b.append("\\r");
        case '\t' -> b.append("\\t");
        default -> {
          if (c < 0x20 || c == 0x7f) {
            b.append(String.format("\\u%04x", (int) c));
          } else {
            b.append(c);
          }
        }
      }
    }
    b.append('"');
  }
}
