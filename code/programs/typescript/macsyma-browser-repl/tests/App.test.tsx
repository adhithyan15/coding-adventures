import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "../src/App.js";

describe("Macsyma browser REPL", () => {
  it("evaluates the default program in the browser session", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Run" }));

    const transcript = within(screen.getByLabelText("Transcript"));
    expect(await transcript.findByText("7")).toBeInTheDocument();
    expect(transcript.getByText("25")).toBeInTheDocument();
    expect(transcript.getByText("%i2")).toBeInTheDocument();
  });

  it("keeps history across runs and can reset", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "History" }));
    fireEvent.click(screen.getByRole("button", { name: "Run" }));

    const transcript = within(screen.getByLabelText("Transcript"));
    expect(await transcript.findAllByText("10")).toHaveLength(2);
    expect(transcript.getAllByText("5")).toHaveLength(2);
    expect(transcript.getByText("%i4")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Reset" }));

    expect(screen.getByText("(%i1)")).toBeInTheDocument();
  });
});
