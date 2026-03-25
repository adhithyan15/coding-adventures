const GITHUB_URL =
  "https://github.com/adhithyan15/coding-adventures/tree/main/code/programs/typescript/engram-app";
const SPEC_URL =
  "https://github.com/adhithyan15/coding-adventures/blob/main/code/specs/engram-app.md";
const RELEASES_URL =
  "https://github.com/adhithyan15/coding-adventures/releases?q=engram-v&expanded=true";

export function Footer() {
  return (
    <footer className="footer">
      <div className="footer-inner">
        <div className="footer-brand">
          <span className="footer-logo">🧠</span>
          <span className="footer-name">Engram</span>
          <span className="footer-tagline">Spaced repetition, open source</span>
        </div>
        <div className="footer-links">
          <a href={GITHUB_URL} target="_blank" rel="noopener noreferrer">
            Source
          </a>
          <a href={SPEC_URL} target="_blank" rel="noopener noreferrer">
            Spec
          </a>
          <a href={RELEASES_URL} target="_blank" rel="noopener noreferrer">
            Releases
          </a>
        </div>
      </div>
    </footer>
  );
}
