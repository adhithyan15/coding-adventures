import { describe, it, expect, beforeAll, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../i18n/locales/en.json";
import { appState, createState, createTemplate } from "../state.js";
import { TemplateLibrary } from "./TemplateLibrary.js";

beforeAll(() => {
  initI18n({ en });
});

beforeEach(() => {
  // Reset singleton state between tests
  appState.templates.length = 0;
  appState.instances.length = 0;
});

describe("TemplateLibrary", () => {
  it("shows empty state when no templates", () => {
    render(<TemplateLibrary onNavigate={() => {}} />);
    expect(screen.getByText(/no checklists yet/i)).toBeInTheDocument();
  });

  it("renders a card for each template", () => {
    createTemplate(appState, "Alpha", "desc", []);
    createTemplate(appState, "Beta", "desc", []);
    render(<TemplateLibrary onNavigate={() => {}} />);
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    expect(screen.getByText("Beta")).toBeInTheDocument();
  });

  it("Run button creates an instance and navigates", () => {
    createTemplate(appState, "My List", "", [
      { id: "i1", type: "check", label: "Step 1" },
    ]);
    const navigate = vi.fn();
    render(<TemplateLibrary onNavigate={navigate} />);
    fireEvent.click(screen.getByText("Run"));
    expect(navigate).toHaveBeenCalledWith(
      expect.stringMatching(/^\/instance\//),
    );
    expect(appState.instances).toHaveLength(1);
  });

  it("Edit button navigates to editor", () => {
    const t = createTemplate(appState, "My List", "", []);
    const navigate = vi.fn();
    render(<TemplateLibrary onNavigate={navigate} />);
    fireEvent.click(screen.getByText("Edit"));
    expect(navigate).toHaveBeenCalledWith(`/template/${t.id}/edit`);
  });

  it("New Checklist button navigates to /template/new", () => {
    const navigate = vi.fn();
    render(<TemplateLibrary onNavigate={navigate} />);
    fireEvent.click(screen.getByText("New Checklist"));
    expect(navigate).toHaveBeenCalledWith("/template/new");
  });
});
