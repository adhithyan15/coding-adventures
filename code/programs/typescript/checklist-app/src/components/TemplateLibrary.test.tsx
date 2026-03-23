import { describe, it, expect, beforeAll, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../i18n/locales/en.json";
import { store } from "../state.js";
import {
  stateLoadAction,
  createTemplateAction,
} from "../actions.js";
import { TemplateLibrary } from "./TemplateLibrary.js";

beforeAll(() => {
  initI18n({ en });
});

beforeEach(() => {
  // Reset store state between tests
  store.dispatch(stateLoadAction([], [], []));
});

describe("TemplateLibrary", () => {
  it("shows empty state when no templates", () => {
    render(<TemplateLibrary onNavigate={() => {}} />);
    expect(screen.getByText(/no checklists yet/i)).toBeInTheDocument();
  });

  it("renders a card for each template", () => {
    store.dispatch(createTemplateAction("Alpha", "desc", []));
    store.dispatch(createTemplateAction("Beta", "desc", []));
    render(<TemplateLibrary onNavigate={() => {}} />);
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    expect(screen.getByText("Beta")).toBeInTheDocument();
  });

  it("Run button creates an instance and navigates", () => {
    store.dispatch(createTemplateAction("My List", "", [
      { id: "i1", type: "check", label: "Step 1" },
    ]));
    const navigate = vi.fn();
    render(<TemplateLibrary onNavigate={navigate} />);
    fireEvent.click(screen.getByText("Run"));
    expect(navigate).toHaveBeenCalledWith(
      expect.stringMatching(/^\/instance\//),
    );
    expect(store.getState().instances).toHaveLength(1);
  });

  it("Edit button navigates to editor", () => {
    store.dispatch(createTemplateAction("My List", "", []));
    const templateId = store.getState().templates[0]!.id;
    const navigate = vi.fn();
    render(<TemplateLibrary onNavigate={navigate} />);
    fireEvent.click(screen.getByText("Edit"));
    expect(navigate).toHaveBeenCalledWith(`/template/${templateId}/edit`);
  });

  it("New Checklist button navigates to /template/new", () => {
    const navigate = vi.fn();
    render(<TemplateLibrary onNavigate={navigate} />);
    fireEvent.click(screen.getByText("New Checklist"));
    expect(navigate).toHaveBeenCalledWith("/template/new");
  });
});
