/**
 * TreeView.test.tsx — Tests for the generic recursive tree renderer.
 *
 * Tests cover: flat rendering, nested rendering, expand/collapse,
 * ARIA attributes, and the toggle callback.
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { TreeView } from "./TreeView.js";
import type { TreeViewNode } from "./TreeView.js";

interface TestNode extends TreeViewNode {
  label: string;
  children?: TestNode[];
}

const flatNodes: TestNode[] = [
  { id: "a", label: "Alpha" },
  { id: "b", label: "Beta" },
  { id: "c", label: "Gamma" },
];

const nestedNodes: TestNode[] = [
  { id: "a", label: "Alpha" },
  {
    id: "b",
    label: "Beta",
    children: [
      { id: "b1", label: "Beta-1" },
      { id: "b2", label: "Beta-2" },
    ],
  },
  { id: "c", label: "Gamma" },
];

const deepNodes: TestNode[] = [
  {
    id: "root",
    label: "Root",
    children: [
      {
        id: "mid",
        label: "Middle",
        children: [{ id: "leaf", label: "Leaf" }],
      },
    ],
  },
];

function renderLabel(node: TestNode) {
  return <span data-testid={`node-${node.id}`}>{node.label}</span>;
}

describe("TreeView", () => {
  it("renders a tree role container", () => {
    render(
      <TreeView nodes={[]} renderNode={() => null} ariaLabel="Test tree" />,
    );
    const tree = screen.getByRole("tree");
    expect(tree).toHaveAttribute("aria-label", "Test tree");
  });

  it("renders flat nodes without children", () => {
    render(<TreeView nodes={flatNodes} renderNode={renderLabel} />);
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    expect(screen.getByText("Beta")).toBeInTheDocument();
    expect(screen.getByText("Gamma")).toBeInTheDocument();
  });

  it("renders nested nodes when expanded", () => {
    render(
      <TreeView
        nodes={nestedNodes}
        renderNode={renderLabel}
        isExpanded={() => true}
      />,
    );
    expect(screen.getByText("Beta-1")).toBeInTheDocument();
    expect(screen.getByText("Beta-2")).toBeInTheDocument();
  });

  it("hides children when isExpanded returns false", () => {
    render(
      <TreeView
        nodes={nestedNodes}
        renderNode={renderLabel}
        isExpanded={() => false}
      />,
    );
    expect(screen.queryByText("Beta-1")).not.toBeInTheDocument();
    expect(screen.queryByText("Beta-2")).not.toBeInTheDocument();
  });

  it("renders 3 levels deep", () => {
    render(
      <TreeView
        nodes={deepNodes}
        renderNode={renderLabel}
        isExpanded={() => true}
      />,
    );
    expect(screen.getByText("Root")).toBeInTheDocument();
    expect(screen.getByText("Middle")).toBeInTheDocument();
    expect(screen.getByText("Leaf")).toBeInTheDocument();
  });

  it("sets aria-level on each treeitem", () => {
    render(
      <TreeView
        nodes={deepNodes}
        renderNode={renderLabel}
        isExpanded={() => true}
      />,
    );
    const items = screen.getAllByRole("treeitem");
    expect(items[0]).toHaveAttribute("aria-level", "1");
    expect(items[1]).toHaveAttribute("aria-level", "2");
    expect(items[2]).toHaveAttribute("aria-level", "3");
  });

  it("sets aria-expanded on nodes with children", () => {
    render(
      <TreeView
        nodes={nestedNodes}
        renderNode={renderLabel}
        isExpanded={(n) => n.id === "b"}
      />,
    );
    const items = screen.getAllByRole("treeitem");
    // Alpha: no children → no aria-expanded
    expect(items[0]).not.toHaveAttribute("aria-expanded");
    // Beta: has children, expanded
    expect(items[1]).toHaveAttribute("aria-expanded", "true");
  });

  it("calls onToggleExpand when toggle button clicked", () => {
    const onToggle = vi.fn();
    render(
      <TreeView
        nodes={nestedNodes}
        renderNode={renderLabel}
        isExpanded={() => true}
        onToggleExpand={onToggle}
      />,
    );
    const toggleBtns = screen.getAllByLabelText("Collapse");
    fireEvent.click(toggleBtns[0]!);
    expect(onToggle).toHaveBeenCalledWith(
      expect.objectContaining({ id: "b" }),
    );
  });

  it("renders child groups with group role", () => {
    render(
      <TreeView
        nodes={nestedNodes}
        renderNode={renderLabel}
        isExpanded={() => true}
      />,
    );
    const groups = screen.getAllByRole("group");
    expect(groups.length).toBeGreaterThanOrEqual(1);
  });

  it("adds --last modifier to last node wrapper", () => {
    const { container } = render(
      <TreeView nodes={flatNodes} renderNode={renderLabel} />,
    );
    const wrappers = container.querySelectorAll(".tree__node-wrapper");
    expect(wrappers[2]).toHaveClass("tree__node-wrapper--last");
    expect(wrappers[0]).not.toHaveClass("tree__node-wrapper--last");
  });
});
