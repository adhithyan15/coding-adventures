import { useState } from "react";

const INSTALL_TABS = [
  {
    id: "python",
    label: "Python",
    emoji: "🐍",
    install: `pip install coding-adventures-lattice-transpiler`,
    usage: `from lattice_transpiler import transpile_lattice

# Read your Lattice source file
source = open("styles.lattice").read()

# Transpile to CSS
css = transpile_lattice(source)
print(css)

# Minified output
css_min = transpile_lattice(source, minified=True)`,
  },
  {
    id: "go",
    label: "Go",
    emoji: "🐹",
    install: `go get github.com/adhithyan15/coding-adventures/code/packages/go/lattice-transpiler`,
    usage: `package main

import (
    "fmt"
    "os"

    latticetranspiler "github.com/adhithyan15/coding-adventures/code/packages/go/lattice-transpiler"
)

func main() {
    source, _ := os.ReadFile("styles.lattice")

    // Transpile to CSS
    css, err := latticetranspiler.TranspileLattice(string(source))
    if err != nil {
        fmt.Fprintf(os.Stderr, "Error: %v\\n", err)
        os.Exit(1)
    }

    fmt.Println(css)
}`,
  },
  {
    id: "ruby",
    label: "Ruby",
    emoji: "💎",
    install: `gem install coding_adventures_lattice_transpiler`,
    usage: `require "coding_adventures_lattice_transpiler"

# Read your Lattice source file
source = File.read("styles.lattice")

# Transpile to CSS
css = CodingAdventures::LatticeTranspiler.transpile(source)
puts css

# Minified output
css_min = CodingAdventures::LatticeTranspiler.transpile(
  source, minified: true
)`,
  },
  {
    id: "typescript",
    label: "TypeScript",
    emoji: "🔷",
    install: `npm install @coding-adventures/lattice-transpiler`,
    usage: `import { transpileLattice } from "@coding-adventures/lattice-transpiler";
import { readFileSync } from "fs";

// Read your Lattice source file
const source = readFileSync("styles.lattice", "utf-8");

// Transpile to CSS
const css = transpileLattice(source);
console.log(css);

// With options
const cssMin = transpileLattice(source, { minified: true });
const cssPretty = transpileLattice(source, { indent: "    " });`,
  },
  {
    id: "rust",
    label: "Rust",
    emoji: "🦀",
    install: `# In your Cargo.toml:
[dependencies]
coding-adventures-lattice-transpiler = { path = "..." }`,
    usage: `use coding_adventures_lattice_transpiler::transpile_lattice;
use std::fs;

fn main() {
    let source = fs::read_to_string("styles.lattice")
        .expect("Could not read styles.lattice");

    match transpile_lattice(&source) {
        Ok(css) => println!("{}", css),
        Err(e) => eprintln!("Lattice error: {}", e),
    }
}`,
  },
  {
    id: "elixir",
    label: "Elixir",
    emoji: "⚗️",
    install: `# In mix.exs:
{:coding_adventures_lattice_transpiler,
  path: "../lattice_transpiler"}`,
    usage: `alias CodingAdventures.LatticeTranspiler

# Read your Lattice source file
source = File.read!("styles.lattice")

# Transpile to CSS
case LatticeTranspiler.transpile(source) do
  {:ok, css} ->
    IO.puts(css)

  {:error, message} ->
    IO.puts(:stderr, "Error: \#{message}")
end

# With options
{:ok, css_min} = LatticeTranspiler.transpile(
  source, minified: true
)`,
  },
];

export function InstallGuide() {
  const [activeTab, setActiveTab] = useState("python");
  const [copied, setCopied] = useState<string | null>(null);
  const active = INSTALL_TABS.find((t) => t.id === activeTab)!;

  const copy = (text: string, key: string) => {
    navigator.clipboard.writeText(text);
    setCopied(key);
    setTimeout(() => setCopied(null), 2000);
  };

  return (
    <section className="install-section" id="install">
      <div className="section-container">
        <div className="section-header">
          <h2 className="section-title">Get started</h2>
          <p className="section-subtitle">
            Install the Lattice transpiler in your preferred language and start
            writing CSS with superpowers.
          </p>
        </div>

        <div className="install-tabs">
          {INSTALL_TABS.map((t) => (
            <button
              key={t.id}
              className={`install-tab ${activeTab === t.id ? "active" : ""}`}
              onClick={() => setActiveTab(t.id)}
            >
              <span>{t.emoji}</span>
              <span>{t.label}</span>
            </button>
          ))}
        </div>

        <div className="install-content">
          <div className="install-block">
            <div className="install-block-header">
              <span className="install-block-label">Install</span>
              <button
                className="copy-btn"
                onClick={() => copy(active.install, "install")}
              >
                {copied === "install" ? "Copied!" : "Copy"}
              </button>
            </div>
            <pre className="install-code"><code>{active.install}</code></pre>
          </div>

          <div className="install-block">
            <div className="install-block-header">
              <span className="install-block-label">Usage</span>
              <button
                className="copy-btn"
                onClick={() => copy(active.usage, "usage")}
              >
                {copied === "usage" ? "Copied!" : "Copy"}
              </button>
            </div>
            <pre className="install-code"><code>{active.usage}</code></pre>
          </div>
        </div>
      </div>
    </section>
  );
}
