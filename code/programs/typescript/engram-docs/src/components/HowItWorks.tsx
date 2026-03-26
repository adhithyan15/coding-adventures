const RATINGS = [
  {
    label: "Again",
    score: 0,
    color: "var(--color-again)",
    bg: "var(--color-again-bg)",
    effect: "Resets interval to 1 day. Ease factor drops by 0.2 (min 1.3).",
    example: "Interval: any → 1 day",
  },
  {
    label: "Hard",
    score: 1,
    color: "var(--color-hard)",
    bg: "var(--color-hard-bg)",
    effect: "Interval grows slowly (×1.2). Ease factor drops by 0.15.",
    example: "Interval: 4 days → 5 days",
  },
  {
    label: "Good",
    score: 2,
    color: "var(--color-good)",
    bg: "var(--color-good-bg)",
    effect: "Interval multiplied by ease factor (normal growth).",
    example: "Interval: 4 days × 2.5 EF → 10 days",
  },
  {
    label: "Easy",
    score: 3,
    color: "var(--color-easy)",
    bg: "var(--color-easy-bg)",
    effect: "Interval multiplied by ease factor × 1.3 bonus. EF rises by 0.15.",
    example: "Interval: 4 days × 2.5 EF × 1.3 → 13 days",
  },
];

const EXAMPLE_TIMELINE = [
  { review: 1, rating: "Good", interval: "2 days" },
  { review: 2, rating: "Good", interval: "5 days" },
  { review: 3, rating: "Hard", interval: "6 days" },
  { review: 4, rating: "Good", interval: "15 days" },
  { review: 5, rating: "Easy", interval: "49 days" },
  { review: 6, rating: "Good", interval: "118 days" },
];

export function HowItWorks() {
  return (
    <section className="how-section" id="how-it-works">
      <div className="section-container">
        <div className="section-header">
          <h2 className="section-title">How SM-2 works</h2>
          <p className="section-subtitle">
            Every card tracks two numbers: its interval (days until next review)
            and ease factor (how aggressively the interval grows). Your rating
            after each review adjusts both.
          </p>
        </div>

        <div className="how-ratings">
          {RATINGS.map((r) => (
            <div
              key={r.label}
              className="how-rating-card"
              style={{ borderColor: r.color, background: r.bg }}
            >
              <div className="how-rating-label" style={{ color: r.color }}>
                {r.label}
              </div>
              <p className="how-rating-effect">{r.effect}</p>
              <code className="how-rating-example">{r.example}</code>
            </div>
          ))}
        </div>

        <div className="how-timeline-wrap">
          <h3 className="how-timeline-title">
            Example: one card over 6 reviews
          </h3>
          <p className="how-timeline-subtitle">
            Starting ease factor 2.5. A single card reviewed consistently
            reaches a 4-month interval after just 6 sessions.
          </p>
          <div className="how-timeline">
            {EXAMPLE_TIMELINE.map((t, i) => (
              <div key={i} className="how-timeline-row">
                <div className="how-timeline-num">#{t.review}</div>
                <div
                  className={`how-timeline-rating how-timeline-rating--${t.rating.toLowerCase()}`}
                >
                  {t.rating}
                </div>
                <div className="how-timeline-arrow">→</div>
                <div className="how-timeline-interval">{t.interval}</div>
              </div>
            ))}
          </div>
        </div>

        <div className="how-note">
          <span className="how-note-icon">💡</span>
          <p className="how-note-text">
            The ease factor starts at 2.5 and is clamped between 1.3 and 4.0.
            Cards you keep rating "again" converge toward a 1.3× multiplier
            (slow growth). Cards you ace converge toward a 4.0× multiplier
            (very fast growth). The algorithm self-calibrates to your memory.
          </p>
        </div>
      </div>
    </section>
  );
}
