import { describe, it, expect, beforeAll, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../i18n/locales/en.json";
import {
  appState,
  createTemplate,
  createInstance,
  checkItem,
  completeInstance,
  abandonInstance,
} from "../state.js";
import { StatsView } from "./StatsView.js";

beforeAll(() => {
  initI18n({ en });
});

beforeEach(() => {
  appState.templates.length = 0;
  appState.instances.length = 0;
});

describe("StatsView", () => {
  it("shows template name", () => {
    const t = createTemplate(appState, "My Checklist", "", []);
    const inst = createInstance(appState, t.id);
    completeInstance(appState, inst.id);
    render(<StatsView instanceId={inst.id} onNavigate={() => {}} />);
    expect(screen.getByText("My Checklist")).toBeInTheDocument();
  });

  it("shows 100% when all items are checked", () => {
    const t = createTemplate(appState, "T", "", [
      { id: "i1", type: "check", label: "Step 1" },
    ]);
    const inst = createInstance(appState, t.id);
    checkItem(appState, inst.id, "i1");
    completeInstance(appState, inst.id);
    render(<StatsView instanceId={inst.id} onNavigate={() => {}} />);
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("shows 0% when no items are checked", () => {
    const t = createTemplate(appState, "T", "", [
      { id: "i1", type: "check", label: "Step 1" },
    ]);
    const inst = createInstance(appState, t.id);
    completeInstance(appState, inst.id);
    render(<StatsView instanceId={inst.id} onNavigate={() => {}} />);
    expect(screen.getByText("0%")).toBeInTheDocument();
  });

  it("Run Again creates a new instance and navigates", () => {
    const t = createTemplate(appState, "T", "", []);
    const inst = createInstance(appState, t.id);
    completeInstance(appState, inst.id);
    const navigate = vi.fn();
    render(<StatsView instanceId={inst.id} onNavigate={navigate} />);
    fireEvent.click(screen.getByText("Run Again"));
    expect(appState.instances).toHaveLength(2);
    expect(navigate).toHaveBeenCalledWith(
      expect.stringMatching(/^\/instance\//),
    );
  });

  it("Back to Library navigates to /", () => {
    const t = createTemplate(appState, "T", "", []);
    const inst = createInstance(appState, t.id);
    abandonInstance(appState, inst.id);
    const navigate = vi.fn();
    render(<StatsView instanceId={inst.id} onNavigate={navigate} />);
    fireEvent.click(screen.getByText("Back to Library"));
    expect(navigate).toHaveBeenCalledWith("/");
  });
});
