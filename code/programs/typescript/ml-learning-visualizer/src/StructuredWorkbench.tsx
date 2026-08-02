import { useState } from "react";
import { HopfieldWorkbench } from "./HopfieldWorkbench.js";
import { MessagePassingWorkbench } from "./MessagePassingWorkbench.js";
import { GraphNeighborhoodWorkbench } from "./GraphNeighborhoodWorkbench.js";

export function StructuredWorkbench() {
  const [lab, setLab] = useState<"hopfield" | "message" | "graph-models">("hopfield");
  return (
    <div className="structured-workbench">
      <nav className="structured-lab-switch" aria-label="Structured and memory learning lab">
        <button aria-pressed={lab === "hopfield"} type="button" onClick={() => setLab("hopfield")}>Hopfield memory</button>
        <button aria-pressed={lab === "message"} type="button" onClick={() => setLab("message")}>Message passing</button>
        <button aria-pressed={lab === "graph-models"} type="button" onClick={() => setLab("graph-models")}>GCN vs GAT</button>
      </nav>
      {lab === "hopfield" ? <HopfieldWorkbench /> : lab === "message" ? <MessagePassingWorkbench /> : <GraphNeighborhoodWorkbench />}
    </div>
  );
}
