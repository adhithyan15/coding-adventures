/**
 * BranchGroup.test.tsx — Tests for the collapsible branch wrapper.
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { BranchGroup } from "./BranchGroup.js";

describe("BranchGroup", () => {
  it("renders label", () => {
    render(
      <BranchGroup label="If yes:" collapsed={false} inactive={false}>
        <p>Child content</p>
      </BranchGroup>,
    );
    expect(screen.getByText("If yes:")).toBeInTheDocument();
  });

  it("active state: shows children at full opacity", () => {
    const { container } = render(
      <BranchGroup label="If yes:" collapsed={false} inactive={false}>
        <p>Child content</p>
      </BranchGroup>,
    );
    expect(screen.getByText("Child content")).toBeInTheDocument();
    expect(container.querySelector(".branch-group--active")).toBeInTheDocument();
  });

  it("inactive + collapsed: shows summary, hides children", () => {
    render(
      <BranchGroup
        label="If no:"
        collapsed={true}
        inactive={true}
        summary="3 steps • click to expand"
      >
        <p>Hidden content</p>
      </BranchGroup>,
    );
    expect(screen.getByText("3 steps • click to expand")).toBeInTheDocument();
    expect(screen.queryByText("Hidden content")).not.toBeInTheDocument();
  });

  it("inactive + collapsed: clicking summary calls onToggleCollapse", () => {
    const onToggle = vi.fn();
    render(
      <BranchGroup
        label="If no:"
        collapsed={true}
        inactive={true}
        summary="3 steps • click to expand"
        onToggleCollapse={onToggle}
      >
        <p>Content</p>
      </BranchGroup>,
    );
    fireEvent.click(screen.getByText("3 steps • click to expand"));
    expect(onToggle).toHaveBeenCalledOnce();
  });

  it("inactive + expanded: shows children with inactive class", () => {
    const { container } = render(
      <BranchGroup label="If no:" collapsed={false} inactive={true}>
        <p>Visible but dimmed</p>
      </BranchGroup>,
    );
    expect(screen.getByText("Visible but dimmed")).toBeInTheDocument();
    expect(
      container.querySelector(".branch-group--inactive"),
    ).toBeInTheDocument();
    expect(
      container.querySelector(".branch-group--expanded"),
    ).toBeInTheDocument();
  });

  it("applies custom className", () => {
    const { container } = render(
      <BranchGroup
        label="Test"
        collapsed={false}
        inactive={false}
        className="custom-branch"
      >
        <p>Content</p>
      </BranchGroup>,
    );
    expect(container.querySelector(".custom-branch")).toBeInTheDocument();
  });

  it("collapsed state has correct ARIA attributes", () => {
    render(
      <BranchGroup
        label="If no:"
        collapsed={true}
        inactive={true}
        summary="3 steps • click to expand"
      >
        <p>Content</p>
      </BranchGroup>,
    );
    const summaryBtn = screen.getByRole("button");
    expect(summaryBtn).toHaveAttribute(
      "aria-label",
      "3 steps • click to expand",
    );
  });
});
