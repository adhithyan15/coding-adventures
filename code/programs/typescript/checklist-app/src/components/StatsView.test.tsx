import { describe, it, expect, beforeAll, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../i18n/locales/en.json";
import { store } from "../state.js";
import {
  stateLoadAction,
  createTemplateAction,
  createInstanceAction,
  checkItemAction,
  completeInstanceAction,
  abandonInstanceAction,
} from "../actions.js";
import { StatsView } from "./StatsView.js";

beforeAll(() => {
  initI18n({ en });
});

beforeEach(() => {
  store.dispatch(stateLoadAction([], []));
});

describe("StatsView", () => {
  it("shows template name", () => {
    store.dispatch(createTemplateAction("My Checklist", "", []));
    const templateId = store.getState().templates[0]!.id;
    store.dispatch(createInstanceAction(templateId));
    const inst = store.getState().instances[0]!;
    store.dispatch(completeInstanceAction(inst.id));
    render(<StatsView instanceId={inst.id} onNavigate={() => {}} />);
    expect(screen.getByText("My Checklist")).toBeInTheDocument();
  });

  it("shows 100% when all items are checked", () => {
    store.dispatch(createTemplateAction("T", "", [
      { id: "i1", type: "check", label: "Step 1" },
    ]));
    const templateId = store.getState().templates[0]!.id;
    store.dispatch(createInstanceAction(templateId));
    const inst = store.getState().instances[0]!;
    store.dispatch(checkItemAction(inst.id, "i1"));
    store.dispatch(completeInstanceAction(inst.id));
    render(<StatsView instanceId={inst.id} onNavigate={() => {}} />);
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("shows 0% when no items are checked", () => {
    store.dispatch(createTemplateAction("T", "", [
      { id: "i1", type: "check", label: "Step 1" },
    ]));
    const templateId = store.getState().templates[0]!.id;
    store.dispatch(createInstanceAction(templateId));
    const inst = store.getState().instances[0]!;
    store.dispatch(completeInstanceAction(inst.id));
    render(<StatsView instanceId={inst.id} onNavigate={() => {}} />);
    expect(screen.getByText("0%")).toBeInTheDocument();
  });

  it("Run Again creates a new instance and navigates", () => {
    store.dispatch(createTemplateAction("T", "", []));
    const templateId = store.getState().templates[0]!.id;
    store.dispatch(createInstanceAction(templateId));
    const inst = store.getState().instances[0]!;
    store.dispatch(completeInstanceAction(inst.id));
    const navigate = vi.fn();
    render(<StatsView instanceId={inst.id} onNavigate={navigate} />);
    fireEvent.click(screen.getByText("Run Again"));
    expect(store.getState().instances).toHaveLength(2);
    expect(navigate).toHaveBeenCalledWith(
      expect.stringMatching(/^\/instance\//),
    );
  });

  it("Back to Library navigates to /", () => {
    store.dispatch(createTemplateAction("T", "", []));
    const templateId = store.getState().templates[0]!.id;
    store.dispatch(createInstanceAction(templateId));
    const inst = store.getState().instances[0]!;
    store.dispatch(abandonInstanceAction(inst.id));
    const navigate = vi.fn();
    render(<StatsView instanceId={inst.id} onNavigate={navigate} />);
    fireEvent.click(screen.getByText("Back to Library"));
    expect(navigate).toHaveBeenCalledWith("/");
  });
});
