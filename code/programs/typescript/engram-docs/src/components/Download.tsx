const RELEASES_URL =
  "https://github.com/adhithyan15/coding-adventures/releases?q=engram-v&expanded=true";

const PLATFORMS = [
  {
    icon: "🍎",
    name: "macOS",
    formats: [".dmg (installer)", ".zip (portable)"],
    arch: "Apple Silicon & Intel (universal)",
    note: "Open the .dmg, drag Engram to Applications.",
  },
  {
    icon: "🪟",
    name: "Windows",
    formats: [".exe (NSIS installer)", "portable .exe"],
    arch: "x64",
    note: "Run the installer. Windows may show a SmartScreen prompt — click More info → Run anyway.",
  },
  {
    icon: "🐧",
    name: "Linux",
    formats: [".AppImage"],
    arch: "x64",
    note: "chmod +x Engram-*.AppImage && ./Engram-*.AppImage",
  },
];

export function Download() {
  return (
    <section className="download-section" id="download">
      <div className="section-container">
        <div className="section-header">
          <h2 className="section-title">Download the desktop app</h2>
          <p className="section-subtitle">
            Native installers for macOS, Windows, and Linux. Built with
            Electron — the same React web app, packaged for your OS.
          </p>
        </div>

        <div className="download-grid">
          {PLATFORMS.map((p) => (
            <div key={p.name} className="download-card">
              <div className="download-icon">{p.icon}</div>
              <h3 className="download-platform">{p.name}</h3>
              <div className="download-arch">{p.arch}</div>
              <ul className="download-formats">
                {p.formats.map((f) => (
                  <li key={f}>{f}</li>
                ))}
              </ul>
              <p className="download-note">{p.note}</p>
            </div>
          ))}
        </div>

        <div className="download-cta">
          <a
            className="btn btn-primary btn-large"
            href={RELEASES_URL}
            target="_blank"
            rel="noopener noreferrer"
          >
            View all releases on GitHub →
          </a>
        </div>

        <div className="download-releases-note">
          <span className="download-releases-icon">🏷️</span>
          <p className="download-releases-text">
            Releases are tagged <code>engram-v*</code> (e.g.{" "}
            <code>engram-v0.2.0</code>). Each release includes installers for
            all three platforms built automatically by GitHub Actions.
          </p>
        </div>

        <div className="download-web-alt">
          <p className="download-web-text">
            Prefer not to install anything?
          </p>
          <a
            className="btn btn-secondary"
            href="https://adhithyan15.github.io/coding-adventures/engram/"
            target="_blank"
            rel="noopener noreferrer"
          >
            Open in browser →
          </a>
        </div>
      </div>
    </section>
  );
}
