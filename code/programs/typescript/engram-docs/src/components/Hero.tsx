const APP_URL =
  "https://adhithyan15.github.io/coding-adventures/engram/";
const RELEASES_URL =
  "https://github.com/adhithyan15/coding-adventures/releases?q=engram-v&expanded=true";

export function Hero() {
  const scrollToDownload = () =>
    document.getElementById("download")?.scrollIntoView({ behavior: "smooth" });

  return (
    <section className="hero-section">
      <div className="hero-content">
        <div className="hero-badge">Spaced Repetition</div>
        <h1 className="hero-title">
          Study smarter,{" "}
          <span className="hero-title-accent">not harder</span>
        </h1>
        <p className="hero-subtitle">
          Engram is an open-source flashcard app powered by the SM-2 algorithm.
          See each card at the exact moment you're about to forget it — and
          never waste time on cards you already know.
        </p>
        <div className="hero-cta">
          <a
            className="btn btn-primary"
            href={APP_URL}
            target="_blank"
            rel="noopener noreferrer"
          >
            Try in browser →
          </a>
          <button className="btn btn-secondary" onClick={scrollToDownload}>
            Download desktop app
          </button>
        </div>
      </div>

      <div className="hero-visual">
        <div className="hero-card-stack">
          <div className="hero-card hero-card--back">
            <div className="hero-card__label">Next up</div>
            <div className="hero-card__front">What is the capital of Texas?</div>
          </div>
          <div className="hero-card hero-card--front">
            <div className="hero-card__label">Answer revealed</div>
            <div className="hero-card__front">What is the capital of California?</div>
            <div className="hero-card__divider" />
            <div className="hero-card__back">Sacramento</div>
            <div className="hero-card__ratings">
              <span className="rating-pill rating-pill--again">Again</span>
              <span className="rating-pill rating-pill--hard">Hard</span>
              <span className="rating-pill rating-pill--good">Good</span>
              <span className="rating-pill rating-pill--easy">Easy</span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
