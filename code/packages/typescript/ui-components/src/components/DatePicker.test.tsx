import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { DatePicker } from "./DatePicker.js";

describe("DatePicker", () => {
  it("renders a date input with aria-label", () => {
    render(<DatePicker value="" onChange={() => {}} label="Due date" />);
    const input = screen.getByLabelText("Due date");
    expect(input).toBeInTheDocument();
    expect(input).toHaveAttribute("type", "date");
  });

  it("displays the provided value", () => {
    render(
      <DatePicker value="2026-03-25" onChange={() => {}} label="Due date" />,
    );
    const input = screen.getByLabelText("Due date") as HTMLInputElement;
    expect(input.value).toBe("2026-03-25");
  });

  it("calls onChange when date is selected", () => {
    const onChange = vi.fn();
    render(<DatePicker value="" onChange={onChange} label="Due date" />);
    const input = screen.getByLabelText("Due date");
    fireEvent.change(input, { target: { value: "2026-04-01" } });
    expect(onChange).toHaveBeenCalledWith("2026-04-01");
  });

  it("shows clear button when value is set", () => {
    render(
      <DatePicker value="2026-03-25" onChange={() => {}} label="Due date" />,
    );
    expect(screen.getByLabelText("Clear Due date")).toBeInTheDocument();
  });

  it("does not show clear button when value is empty", () => {
    render(<DatePicker value="" onChange={() => {}} label="Due date" />);
    expect(screen.queryByLabelText(/clear/i)).not.toBeInTheDocument();
  });

  it("clear button calls onChange with empty string", () => {
    const onChange = vi.fn();
    render(
      <DatePicker value="2026-03-25" onChange={onChange} label="Due date" />,
    );
    fireEvent.click(screen.getByLabelText("Clear Due date"));
    expect(onChange).toHaveBeenCalledWith("");
  });

  it("applies custom className", () => {
    const { container } = render(
      <DatePicker
        value=""
        onChange={() => {}}
        label="Due date"
        className="custom"
      />,
    );
    expect(container.querySelector(".custom")).toBeInTheDocument();
  });

  it("sets id on the input element", () => {
    render(
      <DatePicker
        value=""
        onChange={() => {}}
        label="Due date"
        id="my-date"
      />,
    );
    expect(document.getElementById("my-date")).toBeInTheDocument();
  });
});
