import { useState, useCallback, useEffect } from "react";
import { transpileLatticeBrowser } from "../browser-transpiler";

const EXAMPLES: Record<string, { label: string; code: string }> = {
  variables: {
    label: "Variables",
    code: `// ── Variables ──────────────────────────────────
// Define a value once, use it everywhere.
// Change $brand in one place — the whole site updates.

$brand: #4a90d9;
$font-base: 16px;
$radius: 4px;
$gap: 1rem;

.button {
  background: $brand;
  border-radius: $radius;
  font-size: $font-base;
  padding: $gap calc($gap * 2);
  color: white;
}

.link {
  color: $brand;
  text-decoration: none;
}`,
  },

  mixins: {
    label: "Mixins",
    code: `// ── Mixins ──────────────────────────────────────
// A mixin is a named, reusable block of CSS.
// Parameters let you customize it per call site.

@mixin flex-center($direction: row) {
  display: flex;
  justify-content: center;
  align-items: center;
  flex-direction: $direction;
}

@mixin card($bg: #fff, $shadow: true) {
  background: $bg;
  border-radius: 8px;
  padding: 1.5rem;
}

.hero {
  @include flex-center(column);
  min-height: 100vh;
}

.card-primary {
  @include card(#f0f4ff);
}

.card-secondary {
  @include card(#f9fafb);
}`,
  },

  control_flow: {
    label: "Control Flow",
    code: `// ── Control Flow ────────────────────────────────
// Generate CSS programmatically with loops and conditionals.

$dark-mode: false;

// @if/@else — conditional CSS
@if $dark-mode {
  body {
    background: #111;
    color: #eee;
  }
} @else {
  body {
    background: #fff;
    color: #111;
  }
}

// @for — numeric loop (through = inclusive, to = exclusive)
@for $i from 1 through 4 {
  .col-$i {
    flex: 0 0 calc($i * 25%);
  }
}

// @each — iterate over a list
$sizes: sm, md, lg, xl;
@each $size in $sizes {
  .text-$size {
    font-size: 1rem;
  }
}`,
  },

  functions: {
    label: "Functions",
    code: `// ── Functions ───────────────────────────────────
// Functions compute CSS values at compile time.
// Use them to enforce design systems.

// 8-point spacing scale (multiply by 8px)
@function spacing($n) {
  @return $n * 8px;
}

// Convert px to rem (assumes 16px base)
@function rem($px) {
  @return $px / 16 * 1rem;
}

// Clamp a value between min and max
@function clamp-between($val, $min, $max) {
  @if $val < $min {
    @return $min;
  } @else if $val > $max {
    @return $max;
  } @else {
    @return $val;
  }
}

.card {
  padding: spacing(2) spacing(3);
  font-size: rem(14);
  border-radius: clamp-between(8px, 4px, 16px);
}

.hero {
  padding: spacing(4) spacing(8);
  font-size: rem(32);
}`,
  },

  full_example: {
    label: "Full Example",
    code: `// ── Full Design System ──────────────────────────
// A complete example using all Lattice features.

// Design tokens
$primary: #6366f1;
$secondary: #8b5cf6;
$success: #10b981;
$danger: #ef4444;
$gray-100: #f3f4f6;
$gray-900: #111827;
$radius-sm: 4px;
$radius-md: 8px;
$radius-lg: 16px;

// Utilities
@function space($n) {
  @return $n * 0.25rem;
}

// Flex helper
@mixin flex($justify: flex-start, $align: stretch) {
  display: flex;
  justify-content: $justify;
  align-items: $align;
}

// Card mixin
@mixin card($elevation: 1) {
  background: white;
  border-radius: $radius-md;
  padding: space(6);
  box-shadow: 0 calc($elevation * 1px) calc($elevation * 4px) rgba(0,0,0,0.1);
}

// Base styles
body {
  font-family: system-ui, sans-serif;
  background: $gray-100;
  color: $gray-900;
}

// Component variants via @each
$variants: primary, secondary, success, danger;
$colors: $primary, $secondary, $success, $danger;

@each $variant in $variants {
  .btn-$variant {
    padding: space(2) space(4);
    border-radius: $radius-sm;
    border: none;
    cursor: pointer;
    font-weight: 500;
  }
}

// Spacing scale
@for $i from 1 through 8 {
  .p-$i  { padding: space($i); }
  .m-$i  { margin: space($i); }
  .px-$i { padding-left: space($i); padding-right: space($i); }
  .py-$i { padding-top: space($i); padding-bottom: space($i); }
}

// Cards
.card { @include card(1); }
.card-raised { @include card(3); }

// Layout
.container {
  @include flex(space-between, center);
  max-width: 1200px;
  margin: 0 auto;
  padding: space(4) space(6);
}`,
  },
};

export function Playground() {
  const [selectedExample, setSelectedExample] = useState("variables");
  const [input, setInput] = useState(EXAMPLES.variables.code);
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [minified, setMinified] = useState(false);

  const compile = useCallback(
    (source: string) => {
      const result = transpileLatticeBrowser(source, { minified });
      if (result.success) {
        setOutput(result.css);
        setError(null);
      } else {
        setOutput("");
        setError(result.error);
      }
    },
    [minified]
  );

  // Compile on every change
  useEffect(() => {
    compile(input);
  }, [input, compile]);

  const loadExample = (key: string) => {
    setSelectedExample(key);
    setInput(EXAMPLES[key].code);
  };

  return (
    <section className="playground-section" id="playground">
      <div className="section-container">
        <div className="section-header">
          <h2 className="section-title">Live playground</h2>
          <p className="section-subtitle">
            Edit Lattice on the left — compiled CSS appears instantly on the right.
          </p>
        </div>

        <div className="playground-toolbar">
          <div className="example-tabs">
            {Object.entries(EXAMPLES).map(([key, ex]) => (
              <button
                key={key}
                className={`example-tab ${selectedExample === key ? "active" : ""}`}
                onClick={() => loadExample(key)}
              >
                {ex.label}
              </button>
            ))}
          </div>
          <label className="minified-toggle">
            <input
              type="checkbox"
              checked={minified}
              onChange={(e) => setMinified(e.target.checked)}
            />
            Minified
          </label>
        </div>

        <div className="playground-panels">
          <div className="playground-panel">
            <div className="panel-header">
              <span className="panel-label lattice-label">Lattice</span>
              <span className="panel-hint">← edit me</span>
            </div>
            <textarea
              className="panel-editor"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              spellCheck={false}
              autoComplete="off"
              autoCorrect="off"
              autoCapitalize="off"
            />
          </div>

          <div className="playground-divider">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M5 12h14M12 5l7 7-7 7" />
            </svg>
          </div>

          <div className="playground-panel">
            <div className="panel-header">
              <span className="panel-label css-label">CSS output</span>
              {!error && output && (
                <button
                  className="copy-btn"
                  onClick={() => navigator.clipboard.writeText(output)}
                >
                  Copy
                </button>
              )}
            </div>
            {error ? (
              <div className="panel-error">
                <div className="error-icon">⚠</div>
                <pre className="error-message">{error}</pre>
              </div>
            ) : (
              <pre className="panel-output"><code>{output}</code></pre>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}
