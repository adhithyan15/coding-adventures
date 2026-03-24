const LATTICE_SAMPLE = `$primary: #4a90d9;
$radius: 6px;
$spacing: 16px;

@mixin card($bg: white) {
  background: $bg;
  border-radius: $radius;
  padding: $spacing;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.hero {
  @include card(#f8f9fa);
  color: $primary;
  font-size: 1.25rem;
}

@for $i from 1 through 3 {
  .col-$i {
    width: calc($i * 33.33%);
  }
}`;

const CSS_SAMPLE = `.hero {
  background: #f8f9fa;
  border-radius: 6px;
  padding: 16px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  color: #4a90d9;
  font-size: 1.25rem;
}

.col-1 {
  width: calc(1 * 33.33%);
}

.col-2 {
  width: calc(2 * 33.33%);
}

.col-3 {
  width: calc(3 * 33.33%);
}`;

function CodeBlock({ code, lang }: { code: string; lang: string }) {
  return (
    <div className="hero-code-block">
      <div className="code-block-header">
        <div className="code-block-dots">
          <span /><span /><span />
        </div>
        <span className="code-block-lang">{lang}</span>
      </div>
      <pre className="code-block-content"><code>{code}</code></pre>
    </div>
  );
}

export function Hero() {
  const scrollToPlayground = () => {
    document.getElementById("playground")?.scrollIntoView({ behavior: "smooth" });
  };

  return (
    <section className="hero-section">
      <div className="hero-content">
        <div className="hero-badge">CSS Superset Transpiler</div>
        <h1 className="hero-title">
          CSS with{" "}
          <span className="hero-title-accent">superpowers</span>
        </h1>
        <p className="hero-subtitle">
          Lattice extends CSS with variables, mixins, control flow, functions,
          and modules — all compiled away to plain CSS at build time.
          Zero runtime overhead. Implemented in 6 languages.
        </p>
        <div className="hero-cta">
          <button className="btn btn-primary" onClick={scrollToPlayground}>
            Try it live →
          </button>
          <a
            className="btn btn-secondary"
            href="https://github.com/adhithyan15/coding-adventures"
            target="_blank"
            rel="noopener noreferrer"
          >
            View on GitHub
          </a>
        </div>
      </div>

      <div className="hero-code-comparison">
        <CodeBlock code={LATTICE_SAMPLE} lang="Lattice" />
        <div className="hero-arrow">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M5 12h14M12 5l7 7-7 7" />
          </svg>
        </div>
        <CodeBlock code={CSS_SAMPLE} lang="CSS output" />
      </div>
    </section>
  );
}
