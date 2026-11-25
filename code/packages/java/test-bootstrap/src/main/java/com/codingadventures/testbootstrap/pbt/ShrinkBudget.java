package com.codingadventures.testbootstrap.pbt;

/**
 * The one budget of one whole shrink (VER05e section 4.1, ported from the
 * VER05g security review).
 *
 * <p>A shrink of one falsified case may involve many shrinkers: a tuple's
 * components, a list of lists, a custom shrinker. If each kept its own budget
 * the work would multiply, so the budget is <em>ambient</em>: the outermost
 * entry point opens a scope and every nested shrinker joins it. Two things are
 * charged, thread-safely:
 *
 * <ul>
 *   <li><b>attempts</b>, one per property evaluation, up to
 *       {@code 16 * maxSteps + 64};
 *   <li><b>work</b>, the number of list elements copied, up to
 *       {@link Limits#MAX_SHRINK_WORK}.
 * </ul>
 *
 * Once either is spent every further charge is refused, so nothing more is
 * accepted, every loop ends, and the best value found so far is kept.
 *
 * <p>Public top-level entry points ({@link Runner#check}, {@link Gen#shrink},
 * {@link Shrink#shrinkList}, {@link Shrink#shrinkBytes}) open an
 * <em>isolated</em> scope: a property that itself runs a verdict or a shrink
 * gets a fresh budget instead of eating (and being starved by) the outer one,
 * and the outer budget is restored in a {@code finally}. Combinators that
 * shrink their components use {@link Gen#shrinkNested}, which joins.
 *
 * <p>The ambient slot is an {@link InheritableThreadLocal}: a thread created
 * inside a scope (a worker the property or shrinker spawns) starts with the
 * same budget and so charges the same counters. A thread that already existed
 * (a pooled executor thread) does not inherit; see VER05e section 4.1.
 */
final class ShrinkBudget implements AutoCloseable {
  private static final InheritableThreadLocal<ShrinkBudget> AMBIENT = new InheritableThreadLocal<>();

  /**
   * Property context, per THREAD (not per budget): cell[0] > 0 while this thread
   * is inside a guarded property. A thread spawned from inside a property
   * inherits "in property" (1); one spawned from shrinker context starts at 0,
   * so a parallel custom shrinker's workers do not make each other isolate.
   */
  static final InheritableThreadLocal<int[]> PROP_DEPTH =
      new InheritableThreadLocal<>() {
        @Override
        protected int[] initialValue() {
          return new int[1];
        }

        @Override
        protected int[] childValue(int[] parent) {
          return new int[] {parent[0] > 0 ? 1 : 0};
        }
      };

  /** Overridable by tests (see {@code Hooks.reset}): the work allowance of a new budget. */
  static volatile long defaultMaxWork = Limits.MAX_SHRINK_WORK;

  /** Work spent by the most recent owning scope that ended (for tests). */
  static volatile long lastWork;

  /** Attempts spent by the most recent owning scope that ended (for tests). */
  static volatile long lastAttempts;

  private final Object lock = new Object();
  private final ShrinkBudget joined; // non-null for a joining scope: charges go there
  private final ShrinkBudget previous; // ambient value to restore when an owner ends
  private final long maxAttempts;
  private final long maxWork;
  private long attempts;
  private long work;
  private volatile boolean closed; // set when an owning scope ends

  private ShrinkBudget(int maxSteps, long maxWork, ShrinkBudget joined, ShrinkBudget previous) {
    this.maxAttempts = Limits.attemptBudget(maxSteps);
    this.maxWork = maxWork;
    this.joined = joined;
    this.previous = previous;
  }

  /** The ambient budget of this thread, or null outside any shrink. */
  static ShrinkBudget current() {
    return AMBIENT.get();
  }

  private ShrinkBudget root() {
    return joined != null ? joined : this;
  }

  /** Attempts charged so far. */
  long attempts() {
    ShrinkBudget r = root();
    synchronized (r.lock) {
      return r.attempts;
    }
  }

  /** Elements charged so far. */
  long work() {
    ShrinkBudget r = root();
    synchronized (r.lock) {
      return r.work;
    }
  }

  /** Joins the ambient budget when there is one, else opens a fresh one. Close the result. */
  static ShrinkBudget enter(int maxSteps) {
    return enter(maxSteps, false, defaultMaxWork);
  }

  static ShrinkBudget enter(int maxSteps, boolean isolated, long maxWork) {
    ShrinkBudget ambient = AMBIENT.get();
    if (ambient != null && ambient.closed) {
      // An inheriting thread that outlived its scope: the budget is dead, not ours.
      AMBIENT.remove();
      ambient = null;
    }
    if (!isolated && ambient != null) {
      return new ShrinkBudget(0, 0, ambient.root(), null);
    }
    ShrinkBudget fresh = new ShrinkBudget(maxSteps, maxWork, null, ambient);
    AMBIENT.set(fresh);
    return fresh;
  }

  /**
   * The scope of a public top-level entry point. It is isolated when there is
   * no ambient budget, or when THIS THREAD is currently running a PROPERTY
   * (re-entrancy from inside a property: the inner shrink must not eat, or be
   * starved by, the outer one). Called from SHRINKER context (a custom
   * shrinker calling {@code Gen.shrink} on a component) it joins, so such a
   * shrinker cannot re-open nested amplification.
   */
  static ShrinkBudget enterTop(int maxSteps) {
    ShrinkBudget ambient = AMBIENT.get();
    return enter(maxSteps, PROP_DEPTH.get()[0] > 0, defaultMaxWork);
  }

  /**
   * Wraps a property predicate so that, while it runs on a thread, that thread is
   * in property context (see {@link #enterTop}). The depth is saved and restored
   * by value, with no method call in the {@code finally}, so a StackOverflowError
   * cannot leave the cell raised.
   */
  static <V> java.util.function.Predicate<V> guarded(java.util.function.Predicate<V> property) {
    return v -> {
      int[] c = PROP_DEPTH.get();
      int saved = c[0];
      try {
        c[0] = saved + 1;
        return property.test(v);
      } finally {
        c[0] = saved;
      }
    };
  }

  /**
   * Charges one attempt over a candidate of {@code elements} elements (at least
   * 1). Returns false, charging nothing, when either budget cannot afford it:
   * callers must check this BEFORE building the candidate.
   */
  boolean charge(long elements) {
    return root().take(elements, 1);
  }

  /**
   * Charges a copy that is not itself a property evaluation (an initial copy, a
   * list-to-array conversion): work only, so attempt counts are unchanged.
   * Refused once spent.
   */
  boolean chargeCopy(long elements) {
    return root().take(elements, 0);
  }

  private boolean take(long elements, int attemptCost) {
    long cost = Math.max(1, elements);
    synchronized (lock) {
      if (attempts >= maxAttempts || work + cost > maxWork) {
        return false;
      }
      attempts += attemptCost;
      work += cost;
      return true;
    }
  }

  /** True while at least one more attempt of one element could be afforded. */
  boolean hasRoom() {
    ShrinkBudget r = root();
    synchronized (r.lock) {
      return r.attempts < r.maxAttempts && r.work < r.maxWork;
    }
  }

  @Override
  public void close() {
    if (joined != null) {
      return;
    }
    try {
      // Restore the ambient budget FIRST, so nothing below can leave a stale one.
      if (previous == null) {
        AMBIENT.remove();
      } else {
        AMBIENT.set(previous);
      }
    } finally {
      closed = true;
      synchronized (lock) {
        lastWork = work;
        lastAttempts = attempts;
      }
    }
  }
}
