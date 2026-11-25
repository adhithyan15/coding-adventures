package com.codingadventures.testbootstrap.fuzz;

import com.codingadventures.testbootstrap.prng.SplitMix64;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

/**
 * Crash signatures, frame normalisation and the one public failure line (VER06
 * sections 5.2 and 6, VER06f sections 5 and 7).
 */
public final class Signature {
  private Signature() {}

  /** Caps the target id in a TAP line, in UTF-8 bytes. */
  static final int MAX_TARGET_LEN = 128;

  /** The innermost frames kept in a signature. */
  static final int MAX_FRAMES = 64;

  /** Caps a crash kind. */
  static final int MAX_KIND_LEN = 64;

  /**
   * The low 32 bits of {@code fnv1a64(kind ":" frames joined by ":")}, as 8 lowercase
   * hex digits.
   *
   * <p>Frames are NOT escaped (VER06 section 9.4 item 6): one frame {@code "x:y"}
   * and two frames {@code "x", "y"} hash alike. This lane never produces a frame
   * with a colon ({@link #normaliseFrame}), so the ambiguity cannot arise from
   * Java targets.
   */
  public static String crashSignature(String kind, List<String> frames) {
    String raw = kind + ":" + String.join(":", frames);
    return String.format("%08x", SplitMix64.fnv1a64(raw) & 0xFFFFFFFFL);
  }

  /** Whether two crash reports share one signature. */
  public static boolean isDuplicateCrash(String kindA, List<String> framesA, String kindB, List<String> framesB) {
    return crashSignature(kindA, framesA).equals(crashSignature(kindB, framesB));
  }

  // ---------------------------------------------------------------------------
  // TAP: the only public failure line
  // ---------------------------------------------------------------------------

  private static boolean isIdByte(int c) {
    return (c >= 'a' && c <= 'z')
        || (c >= 'A' && c <= 'Z')
        || (c >= '0' && c <= '9')
        || c == '.'
        || c == '_'
        || c == '/'
        || c == '-';
  }

  private static boolean isHexByte(int c) {
    return (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f');
  }

  /**
   * Reduces text to the characters its grammar allows; every other UTF-8 BYTE becomes
   * {@code _}. Without this a target id holding a newline would let a caller forge a
   * second TAP line inside "an opaque token". Multi-byte characters are replaced
   * byte for byte, so the result is pure ASCII.
   */
  private static String token(byte[] bytes, int limit, boolean extra) {
    int n = Math.min(bytes.length, limit);
    StringBuilder b = new StringBuilder(n);
    for (int i = 0; i < n; i++) {
      int c = bytes[i] & 0xFF;
      boolean ok = isIdByte(c) || (extra && (c == '<' || c == '>' || c == '$'));
      b.append(ok ? (char) c : '_');
    }
    return b.toString();
  }

  /**
   * The only public failure line:
   *
   * <pre>
   *   not ok N - target [failure-id:hex8]
   * </pre>
   *
   * It carries no input bytes, seed, message or stack trace (VER00 7.3).
   * {@code failureId} must be one of the five registered ids and {@code sigHash}
   * exactly 8 lowercase hex digits, else {@link IllegalArgumentException}. The target
   * is reduced to {@code [A-Za-z0-9._/-]} (every other byte becoming {@code _}) and
   * truncated to 128 bytes. Well-formed arguments pass through unchanged.
   */
  public static String formatTapFailure(int testNum, String target, String failureId, String sigHash) {
    if (!FailureIds.isFuzzFailureId(failureId)) {
      throw new IllegalArgumentException("failure id is not one of the registered fuzz failure ids");
    }
    if (sigHash == null || sigHash.length() != 8) {
      throw new IllegalArgumentException("sig hash must be exactly 8 lowercase hex digits");
    }
    for (int i = 0; i < 8; i++) {
      if (!isHexByte(sigHash.charAt(i))) {
        throw new IllegalArgumentException("sig hash must be exactly 8 lowercase hex digits");
      }
    }
    String safeTarget = token(target.getBytes(StandardCharsets.UTF_8), MAX_TARGET_LEN, false);
    return "not ok " + testNum + " - " + safeTarget + " [" + failureId + ":" + sigHash + "]";
  }

  /**
   * Renders a report as public, opaque failure lines (none for a clean one): one per
   * distinct crash signature, then one for timeouts and one for OOMs. A timeout or
   * OOM has no stack to hash, so its sig hash is the signature of the bare kind.
   */
  public static List<String> tapFailures(Report r, String target, int firstTest) {
    List<String> lines = new ArrayList<>();
    // A hand-built record cannot exist (CrashRecord's constructor is package-private),
    // but the fail-closed fallback stays: a malformed signature still yields a line.
    String invalidSig = crashSignature("invalid", List.of());
    for (CrashRecord rec : r.crashes()) {
      lines.add(tapLine(firstTest + lines.size(), target, FailureIds.CRASH, rec.signature(), invalidSig));
    }
    if (r.timeouts() > 0) {
      lines.add(tapLine(firstTest + lines.size(), target, FailureIds.TIMEOUT, crashSignature("timeout", List.of()), invalidSig));
    }
    if (r.ooms() > 0) {
      lines.add(tapLine(firstTest + lines.size(), target, FailureIds.OOM, crashSignature("oom", List.of()), invalidSig));
    }
    return lines;
  }

  private static String tapLine(int n, String target, String id, String sig, String fallbackSig) {
    try {
      return formatTapFailure(n, target, id, sig);
    } catch (IllegalArgumentException e) {
      return formatTapFailure(n, target, id, fallbackSig);
    }
  }

  // ---------------------------------------------------------------------------
  // Frame normalisation (VER06f section 5)
  // ---------------------------------------------------------------------------

  private static String safeName(String s) {
    return token(s.getBytes(StandardCharsets.UTF_8), Integer.MAX_VALUE, true);
  }

  /**
   * Turns one stack element into {@code Class.method}:
   *
   * <pre>
   *   com.acme.Parser$Inner.read       -&gt; Parser$Inner.read
   *   com.acme.App$$Lambda/0x0123.run  -&gt; App.run
   *   jdk.proxy2.$Proxy12.call         -&gt; $Proxy.call
   * </pre>
   *
   * The package is dropped; a hidden-class suffix ({@code $$Lambda...}, anything
   * after a {@code /}) is removed; a trailing proxy counter is removed. The result
   * contains no {@code :} (every character outside {@code [A-Za-z0-9._<>$/-]}
   * becomes {@code _}).
   */
  static String normaliseFrame(String className, String method) {
    String cls = className;
    int slash = cls.indexOf('/');
    if (slash >= 0) {
      cls = cls.substring(0, slash);
    }
    int lambda = cls.indexOf("$$Lambda");
    if (lambda >= 0) {
      cls = cls.substring(0, lambda);
    }
    int dot = cls.lastIndexOf('.');
    if (dot >= 0) {
      cls = cls.substring(dot + 1);
    }
    int proxy = cls.indexOf("$Proxy");
    if (proxy >= 0) {
      cls = cls.substring(0, proxy + "$Proxy".length());
    }
    return safeName(cls + "." + method);
  }

  /** Turns a throwable's class name into a signature kind: sanitised, at most 64 characters. */
  static String normaliseKind(String className) {
    String k = safeName(className);
    return k.length() > MAX_KIND_LEN ? k.substring(0, MAX_KIND_LEN) : k;
  }

  /**
   * Removes consecutive duplicate frames, so a recursive crash hashes alike at any
   * depth, and keeps the innermost {@link #MAX_FRAMES}.
   */
  static List<String> collapse(List<String> frames) {
    List<String> out = new ArrayList<>();
    for (String f : frames) {
      if (out.isEmpty() || !out.get(out.size() - 1).equals(f)) {
        out.add(f);
        if (out.size() >= MAX_FRAMES) {
          break;
        }
      }
    }
    return out;
  }
}
