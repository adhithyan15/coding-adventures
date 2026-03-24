export function Footer() {
  return (
    <footer className="footer">
      <div className="footer-inner">
        <div className="footer-brand">
          <span className="footer-logo">◈</span>
          <span className="footer-name">Lattice</span>
          <span className="footer-tagline">Part of the coding-adventures monorepo</span>
        </div>
        <div className="footer-links">
          <a
            href="https://github.com/adhithyan15/coding-adventures"
            target="_blank"
            rel="noopener noreferrer"
          >
            GitHub
          </a>
          <a
            href="https://adhithyan15.github.io/coding-adventures/logic-gates/"
            target="_blank"
            rel="noopener noreferrer"
          >
            Logic Gates Visualizer
          </a>
        </div>
      </div>
    </footer>
  );
}
