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
  private final java.util.concurrent.atomic.AtomicInteger propertyDepth =
      new java.util.concurrent.atomic.AtomicInteger();

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
   * no ambient budget, or when the ambient one is currently running a PROPERTY
   * (re-entrancy from inside a property: the inner shrink must not eat, or be
   * starved by, the outer one). Called from SHRINKER context (a custom
   * shrinker calling {@code Gen.shrink} on a component) it joins, so such a
   * shrinker cannot re-open nested amplification.
   */
  static ShrinkBudget enterTop(int maxSteps) {
    ShrinkBudget ambient = AMBIENT.get();
    boolean reentrant =
        ambient != null && !ambient.closed && ambient.root().propertyDepth.get() > 0;
    return enter(maxSteps, reentrant, defaultMaxWork);
  }

  /** Marks (+1) or unmarks (-1) property context on the root budget. */
  void markProperty(int delta) {
    root().propertyDepth.addAndGet(delta);
  }

  /**
   * Wraps a property predicate so that, while it runs, the root budget is
   * marked as being in property context (see {@link #enterTop}).
   */
  <V> java.util.function.Predicate<V> guard(java.util.function.Predicate<V> property) {
    final ShrinkBudget r = root();
    return v -> {
      r.propertyDepth.incrementAndGet();
      try {
        return property.test(v);
      } finally {
        r.propertyDepth.decrementAndGet();
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
