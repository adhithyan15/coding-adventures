const PASSES = [
  {
    number: "01",
    title: "Module Resolution",
    subtitle: "Pass 1",
    color: "var(--color-pass-1)",
    description:
      "Walk the AST and collect all @use directives. Load each referenced file recursively, detecting cycles with a visited-set. The loaded stylesheet's nodes are merged in before the main file's nodes.",
    details: [
      "Detects circular @use references (A uses B uses A)",
      "Namespaces: @use \"colors\" as c; → prefix c.$var",
      "Loaded files are also processed through all 3 passes",
    ],
    input: `@use "tokens";
@use "utils/mixins";

.btn { color: $primary; }`,
    output: `// tokens merged in:
$primary: #4a90d9;

// utils/mixins merged in:
@mixin flex-center { ... }

.btn { color: $primary; }`,
  },
  {
    number: "02",
    title: "Symbol Collection",
    subtitle: "Pass 2",
    color: "var(--color-pass-2)",
    description:
      "Walk the merged AST and extract all variable declarations, mixin definitions, and function definitions into registries. Remove these definition nodes from the tree — they are not emitted as CSS.",
    details: [
      "Variables → key/value map keyed by $name",
      "Mixins → map of name → { params, defaults, body }",
      "Functions → map of name → { params, defaults, body }",
      "@return inside non-function body raises an error",
    ],
    input: `$brand: #4a90d9;

@mixin card($bg: white) {
  background: $bg;
  border-radius: 8px;
}

.hero { @include card; }`,
    output: `// Registries (not in output tree):
// vars:  { $brand: #4a90d9 }
// mixins: { card: { params: [$bg],
//             defaults: { $bg: white },
//             body: ... } }

// Tree after Pass 2:
.hero { @include card; }`,
  },
  {
    number: "03",
    title: "Expansion",
    subtitle: "Pass 3",
    color: "var(--color-pass-3)",
    description:
      "Walk the tree and expand every Lattice node: substitute $variables, inline @include, evaluate @if/@for/@each, and call @function bodies. The result is a pure CSS AST with no Lattice nodes.",
    details: [
      "$variable → look up in scope chain, replace with value token",
      "@include mixin → clone body, bind args to params, expand recursively",
      "@if → evaluate condition, keep matching branch only",
      "@for → generate N copies of body with loop variable bound",
      "@each → iterate list, one copy per item",
      "@function call → execute body, capture @return value",
    ],
    input: `// After Pass 2
$brand: #4a90d9;    // in registry
// card mixin in registry

.hero { @include card; }`,
    output: `.hero {
  background: white;
  border-radius: 8px;
}`,
  },
];

export function HowItWorks() {
  return (
    <section className="how-section" id="how-it-works">
      <div className="section-container">
        <div className="section-header">
          <h2 className="section-title">How it works</h2>
          <p className="section-subtitle">
            Lattice compiles in three passes. Each pass has a single
            responsibility — the clean separation makes the compiler easy to
            understand, test, and extend.
          </p>
        </div>

        <div className="passes-pipeline">
          {PASSES.map((pass, i) => (
            <div key={pass.number} className="pass-group">
              <div className="pass-card">
                <div className="pass-number" style={{ color: pass.color }}>
                  {pass.number}
                </div>
                <div className="pass-badge" style={{ background: pass.color }}>
                  {pass.subtitle}
                </div>
                <h3 className="pass-title">{pass.title}</h3>
                <p className="pass-desc">{pass.description}</p>
                <ul className="pass-details">
                  {pass.details.map((d) => (
                    <li key={d}>{d}</li>
                  ))}
                </ul>
              </div>

              <div className="pass-io">
                <div className="pass-io-block">
                  <div className="pass-io-label">Input</div>
                  <pre className="pass-code"><code>{pass.input}</code></pre>
                </div>
                <div className="pass-io-arrow">→</div>
                <div className="pass-io-block">
                  <div className="pass-io-label">Output</div>
                  <pre className="pass-code"><code>{pass.output}</code></pre>
                </div>
              </div>

              {i < PASSES.length - 1 && (
                <div className="pass-connector">
                  <div className="pass-connector-line" />
                  <div className="pass-connector-arrow">↓</div>
                </div>
              )}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
