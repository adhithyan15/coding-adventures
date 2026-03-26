const FEATURES = [
  {
    icon: "🧠",
    title: "SM-2 Spaced Repetition",
    desc: "The same algorithm that powers Anki. Intervals grow automatically — short at first, then days, weeks, months as you master each card.",
  },
  {
    icon: "📦",
    title: "50 starter cards included",
    desc: "Launches with a US State Capitals deck on first run. No setup required — open the app and start studying immediately.",
  },
  {
    icon: "📶",
    title: "Fully offline",
    desc: "All data lives in IndexedDB, local to your device. No account, no sync, no server. Works in the browser or as a desktop app.",
  },
  {
    icon: "🖥️",
    title: "Web + desktop",
    desc: "The same React app runs in your browser and as a native Electron desktop app on macOS, Windows, and Linux.",
  },
  {
    icon: "🎯",
    title: "Four-rating system",
    desc: "Again, Hard, Good, Easy. Rate your recall precisely so the algorithm can schedule each card at exactly the right interval.",
  },
  {
    icon: "📊",
    title: "Per-deck statistics",
    desc: "See how many cards are new, learning, or mastered. Track your correct percentage and average ease factor per deck.",
  },
];

export function Features() {
  return (
    <section className="features-section" id="features">
      <div className="section-container">
        <div className="section-header">
          <h2 className="section-title">Everything you need to study</h2>
          <p className="section-subtitle">
            Focused on the core study loop. No social features, no gamification —
            just the algorithm and your cards.
          </p>
        </div>
        <div className="features-grid">
          {FEATURES.map((f) => (
            <div key={f.title} className="feature-card">
              <div className="feature-icon">{f.icon}</div>
              <h3 className="feature-title">{f.title}</h3>
              <p className="feature-desc">{f.desc}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
