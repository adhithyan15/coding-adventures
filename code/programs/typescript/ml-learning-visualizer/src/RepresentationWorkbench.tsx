import { useState } from "react";
import { AutoencoderWorkbench } from "./AutoencoderWorkbench.js";
import { GanWorkbench } from "./GanWorkbench.js";
import { VariationalWorkbench } from "./VariationalWorkbench.js";

export function RepresentationWorkbench() {
  const [lab, setLab] = useState<"autoencoder" | "variational" | "gan">("autoencoder");

  return (
    <div className="representation-workbench">
      <nav className="representation-lab-switch" aria-label="Representation learning lab">
        <button
          aria-pressed={lab === "autoencoder"}
          type="button"
          onClick={() => setLab("autoencoder")}
        >
          Deterministic bottleneck
        </button>
        <button
          aria-pressed={lab === "variational"}
          type="button"
          onClick={() => setLab("variational")}
        >
          Variational sample
        </button>
        <button
          aria-pressed={lab === "gan"}
          type="button"
          onClick={() => setLab("gan")}
        >
          Adversarial game
        </button>
      </nav>
      {lab === "autoencoder" ? (
        <AutoencoderWorkbench />
      ) : lab === "variational" ? (
        <VariationalWorkbench />
      ) : (
        <GanWorkbench />
      )}
    </div>
  );
}
