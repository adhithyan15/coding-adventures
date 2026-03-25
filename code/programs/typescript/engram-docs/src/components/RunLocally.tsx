import { useState } from "react";

type Tab = "browser" | "desktop";

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = () => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };
  return (
    <button className="copy-btn" onClick={handleCopy} aria-label="Copy to clipboard">
      {copied ? "Copied!" : "Copy"}
    </button>
  );
}

interface StepProps {
  number: number;
  title: string;
  code: string;
  note?: string;
}

function Step({ number, title, code, note }: StepProps) {
  return (
    <div className="run-step">
      <div className="run-step-number">{number}</div>
      <div className="run-step-body">
        <div className="run-step-title">{title}</div>
        {note && <p className="run-step-note">{note}</p>}
        <div className="run-code-block">
          <div className="run-code-header">
            <span className="run-code-label">bash</span>
            <CopyButton text={code} />
          </div>
          <pre className="run-code-pre"><code>{code}</code></pre>
        </div>
      </div>
    </div>
  );
}

const BROWSER_STEPS = [
  {
    title: "Prerequisites",
    code: "node --version   # need Node.js 20+\nnpm --version    # comes with Node",
    note: "Install Node.js from nodejs.org if you don't have it.",
  },
  {
    title: "Clone the repo",
    code: "git clone https://github.com/adhithyan15/coding-adventures.git\ncd coding-adventures",
  },
  {
    title: "Install shared packages",
    code: "cd code/packages/typescript/indexeddb && npm install\ncd ../store && npm install\ncd ../ui-components && npm install",
    note: "These are local file: dependencies — install in leaf-to-root order.",
  },
  {
    title: "Install and run Engram",
    code: "cd ../../../programs/typescript/engram-app\nnpm install\nnpm run dev",
    note: "Opens at http://localhost:5173/ — the US State Capitals deck is seeded automatically on first launch.",
  },
];

const DESKTOP_STEPS = [
  {
    title: "Complete the browser steps first (1–4 above)",
    code: "# Follow steps 1–4 from the Browser tab",
    note: "The Electron wrapper needs engram-app built and running.",
  },
  {
    title: "Open a second terminal — install the Electron wrapper",
    code: "cd code/programs/typescript/engram-electron\nnpm install\nnpm run build",
    note: "Compiles electron/main.ts → dist-electron/main.js.",
  },
  {
    title: "Launch the desktop app against the dev server",
    code: "npm run dev",
    note: "Electron loads from http://localhost:5173/ in dev mode (hot reload works).",
  },
];

export function RunLocally() {
  const [tab, setTab] = useState<Tab>("browser");

  const steps = tab === "browser" ? BROWSER_STEPS : DESKTOP_STEPS;

  return (
    <section className="run-section" id="run-locally">
      <div className="section-container">
        <div className="section-header">
          <h2 className="section-title">Run locally</h2>
          <p className="section-subtitle">
            Clone the repo and get Engram running in under five minutes.
          </p>
        </div>

        <div className="run-tabs">
          <button
            className={`run-tab ${tab === "browser" ? "active" : ""}`}
            onClick={() => setTab("browser")}
          >
            🌐 In the browser
          </button>
          <button
            className={`run-tab ${tab === "desktop" ? "active" : ""}`}
            onClick={() => setTab("desktop")}
          >
            🖥️ Desktop (Electron)
          </button>
        </div>

        <div className="run-steps">
          {steps.map((s, i) => (
            <Step key={i} number={i + 1} title={s.title} code={s.code} note={s.note} />
          ))}
        </div>

        <div className="run-note">
          <span className="run-note-icon">📝</span>
          <p className="run-note-text">
            The app seeds a US State Capitals deck (50 cards) on first launch
            when IndexedDB is empty. All study data is stored locally in your
            browser — nothing leaves your device.
          </p>
        </div>
      </div>
    </section>
  );
}
