import { render, screen } from "@testing-library/react";
import { describe, expect, test } from "vitest";
import { App } from "./App";

describe("App", () => {
  test("renders the electronics heading", () => {
    render(<App />);
    expect(
      screen.getByRole("heading", {
        name: /electronics visualizers/i,
      })
    ).toBeTruthy();
  });
});
