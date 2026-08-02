import { useState } from "react";
import { GradientFlowWorkbench } from "./GradientFlowWorkbench.js";
import { InitializationWorkbench } from "./InitializationWorkbench.js";
import { TrainingStabilizersWorkbench } from "./TrainingStabilizersWorkbench.js";

export function DeepTrainingWorkbench() {
  const [lab, setLab] = useState<"initialization" | "gradient-flow" | "stabilizers">("initialization");
  return (
    <div className="deep-training-workbench">
      <nav className="deep-training-switch" aria-label="Deep training learning lab">
        <button aria-pressed={lab === "initialization"} type="button" onClick={() => setLab("initialization")}>Initialization</button>
        <button aria-pressed={lab === "gradient-flow"} type="button" onClick={() => setLab("gradient-flow")}>Gradient flow</button>
        <button aria-pressed={lab === "stabilizers"} type="button" onClick={() => setLab("stabilizers")}>Stabilizers</button>
      </nav>
      {lab === "initialization"
        ? <InitializationWorkbench />
        : lab === "gradient-flow"
          ? <GradientFlowWorkbench />
          : <TrainingStabilizersWorkbench />}
    </div>
  );
}
