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

  it("loads a .mac file into the editor and runs it", async () => {
    render(<App />);

    const file = new File(["x : 5$\nx + 1;"], "batch.mac", { type: "text/plain" });
    fireEvent.change(screen.getByLabelText("Load MACSYMA file"), {
      target: { files: [file] },
    });

    expect(await screen.findByText("batch.mac")).toBeInTheDocument();
    expect(screen.getByLabelText("MACSYMA source")).toHaveValue("x : 5$\nx + 1;");

    fireEvent.click(screen.getByRole("button", { name: "Run" }));

    const transcript = within(screen.getByLabelText("Transcript"));
    expect(await transcript.findByText("6")).toBeInTheDocument();
    expect(transcript.getByText("%i2")).toBeInTheDocument();
  });

  it("renders display2d outputs from the runtime transcript text", async () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText("MACSYMA source"), {
      target: { value: "ev(1/(x + 1), display2d);" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Run" }));

    const transcript = within(screen.getByLabelText("Transcript"));
    expect(await transcript.findByText((content) => content.includes("─") && content.includes("x + 1")))
      .toBeInTheDocument();
  });

  it("renders showtime diagnostics from the runtime transcript metadata", async () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText("MACSYMA source"), {
      target: { value: "showtime:true$\n2 + 3$" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Run" }));

    const transcript = within(screen.getByLabelText("Transcript"));
    expect(await transcript.findByText(/^Evaluation took \d+\.\d{6} seconds\.$/)).toBeInTheDocument();
    expect(transcript.queryByText("%o2")).toBeInTheDocument();
  });
});
