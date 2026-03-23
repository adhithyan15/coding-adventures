const LANGUAGES = [
  {
    name: "Python",
    emoji: "🐍",
    packages: ["lattice-lexer", "lattice-parser", "lattice-ast-to-css", "lattice-transpiler"],
    install: "pip install coding-adventures-lattice-transpiler",
    status: "complete",
  },
  {
    name: "Go",
    emoji: "🐹",
    packages: ["lattice-lexer", "lattice-parser", "lattice-ast-to-css", "lattice-transpiler"],
    install: 'go get github.com/adhithyan15/coding-adventures/code/packages/go/lattice-transpiler',
    status: "complete",
  },
  {
    name: "Ruby",
    emoji: "💎",
    packages: ["lattice_lexer", "lattice_parser", "lattice_ast_to_css", "lattice_transpiler"],
    install: "gem install coding_adventures_lattice_transpiler",
    status: "complete",
  },
  {
    name: "TypeScript",
    emoji: "🔷",
    packages: ["lattice-lexer", "lattice-parser", "lattice-ast-to-css", "lattice-transpiler"],
    install: "npm install @coding-adventures/lattice-transpiler",
    status: "complete",
  },
  {
    name: "Rust",
    emoji: "🦀",
    packages: ["lattice-lexer", "lattice-parser", "lattice-ast-to-css", "lattice-transpiler"],
    install: 'cargo add coding-adventures-lattice-transpiler',
    status: "complete",
  },
  {
    name: "Elixir",
    emoji: "⚗️",
    packages: ["lattice_lexer", "lattice_parser", "lattice_ast_to_css", "lattice_transpiler"],
    install: '{:coding_adventures_lattice_transpiler, path: "..."}',
    status: "complete",
  },
];

export function LanguageSupport() {
  return (
    <section className="languages-section" id="languages">
      <div className="section-container">
        <div className="section-header">
          <h2 className="section-title">6 languages, one compiler</h2>
          <p className="section-subtitle">
            Every implementation is semantically identical — same grammar files,
            same 3-pass algorithm, same output for the same input.
          </p>
        </div>

        <div className="languages-grid">
          {LANGUAGES.map((lang) => (
            <div key={lang.name} className="language-card">
              <div className="lang-header">
                <span className="lang-emoji">{lang.emoji}</span>
                <span className="lang-name">{lang.name}</span>
                <span className="lang-status complete">✓ Complete</span>
              </div>
              <div className="lang-packages">
                {lang.packages.map((pkg) => (
                  <span key={pkg} className="lang-pkg">{pkg}</span>
                ))}
              </div>
              <pre className="lang-install"><code>{lang.install}</code></pre>
            </div>
          ))}
        </div>

        <div className="language-note">
          <div className="note-icon">◈</div>
          <div className="note-text">
            All 6 implementations share the same{" "}
            <code>lattice.tokens</code> and <code>lattice.grammar</code> files.
            Change the grammar once — all languages pick it up automatically.
          </div>
        </div>
      </div>
    </section>
  );
}
