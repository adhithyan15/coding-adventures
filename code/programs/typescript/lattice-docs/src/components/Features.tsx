const FEATURES = [
  {
    icon: "◆",
    title: "Variables",
    color: "var(--color-feature-vars)",
    description:
      "Define reusable values once. Change a color or size in one place and it ripples everywhere.",
    code: `$brand: #4a90d9;
$gap: 1rem;

.card {
  color: $brand;
  margin: $gap;
}`,
  },
  {
    icon: "⬡",
    title: "Mixins",
    color: "var(--color-feature-mixins)",
    description:
      "Package reusable blocks of CSS into named mixins with optional parameters and default values.",
    code: `@mixin flex-center($dir: row) {
  display: flex;
  justify-content: center;
  align-items: center;
  flex-direction: $dir;
}

.hero {
  @include flex-center(column);
}`,
  },
  {
    icon: "⬢",
    title: "Control Flow",
    color: "var(--color-feature-control)",
    description:
      "Generate CSS programmatically with @if, @else, @for loops, and @each list iteration.",
    code: `@for $i from 1 through 5 {
  .opacity-$i {
    opacity: calc($i * 0.2);
  }
}

@if $dark-mode {
  body { background: #111; }
} @else {
  body { background: #fff; }
}`,
  },
  {
    icon: "◉",
    title: "Functions",
    color: "var(--color-feature-fns)",
    description:
      "Write compile-time functions that compute CSS values from parameters and return them.",
    code: `@function spacing($n) {
  @return $n * 8px;
}

@function rem($px) {
  @return $px / 16px * 1rem;
}

.button {
  padding: spacing(1) spacing(2);
  font-size: rem(14px);
}`,
  },
  {
    icon: "◎",
    title: "Modules",
    color: "var(--color-feature-modules)",
    description:
      "Split your stylesheets into focused files and compose them with @use.",
    code: `// _colors.lattice
$primary: #4a90d9;
$danger: #e74c3c;

// main.lattice
@use "colors";
@use "utils/mixins" as m;

.alert {
  color: $primary;
}`,
  },
];

export function Features() {
  return (
    <section className="features-section" id="features">
      <div className="section-container">
        <div className="section-header">
          <h2 className="section-title">Everything you need</h2>
          <p className="section-subtitle">
            Lattice adds exactly five features to CSS — each solving a specific
            authoring pain point, none adding runtime overhead.
          </p>
        </div>
        <div className="features-grid">
          {FEATURES.map((f) => (
            <div key={f.title} className="feature-card">
              <div className="feature-icon" style={{ color: f.color }}>
                {f.icon}
              </div>
              <h3 className="feature-title">{f.title}</h3>
              <p className="feature-desc">{f.description}</p>
              <pre className="feature-code"><code>{f.code}</code></pre>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
