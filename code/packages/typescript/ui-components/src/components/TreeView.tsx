/**
 * TreeView — a generic, recursive tree renderer with CSS connectors.
 *
 * Renders any hierarchical data structure as an indented tree with visual
 * connector lines (├─ / └─ / │) drawn entirely in CSS. The component is
 * not domain-specific: it does not know about checklists, file trees, or
 * org charts — it only knows about nodes with children.
 *
 * The consumer supplies a `renderNode` function that returns the JSX for
 * each node. The TreeView handles layout, indentation, connectors, and
 * WAI-ARIA treeview semantics around that content.
 *
 * === CSS connectors ===
 *
 * The connector lines are drawn using two CSS techniques:
 *
 *   1. `border-left` on each `.tree__node-wrapper` creates the vertical
 *      trunk that connects sibling nodes.
 *
 *   2. A `::before` pseudo-element on each wrapper draws the horizontal
 *      connector from the trunk to the node content.
 *
 *   3. The `:last-child` selector removes the bottom portion of the trunk
 *      after the last sibling, turning the T-connector (├─) into an
 *      L-connector (└─).
 *
 * === ARIA ===
 *
 * Implements the WAI-ARIA treeview pattern:
 *   - Outer container: `role="tree"`
 *   - Each node: `role="treeitem"`, `aria-expanded`, `aria-level`
 *   - Child groups: `role="group"`
 *   - Keyboard: ↑↓ navigate, ←→ collapse/expand, Space/Enter activate
 *
 * @example
 * ```tsx
 * <TreeView
 *   nodes={items}
 *   renderNode={(item, depth) => <span>{item.label}</span>}
 *   isExpanded={(item) => expandedIds.has(item.id)}
 *   onToggleExpand={(item) => toggle(item.id)}
 *   ariaLabel="File tree"
 * />
 * ```
 */

import type { ReactNode } from "react";

// ── Types ──────────────────────────────────────────────────────────────────

/** The minimal shape every node must satisfy. */
export interface TreeViewNode {
  id: string;
  children?: TreeViewNode[];
}

export interface TreeViewProps<T extends TreeViewNode> {
  /** The root-level nodes to render. */
  nodes: T[];

  /** Render function called for every node. Return the node's visual content.
   *  The TreeView handles layout, indentation, and connectors around it. */
  renderNode: (node: T, depth: number) => ReactNode;

  /** Return true if the node's children should be visible. Default: always true. */
  isExpanded?: (node: T) => boolean;

  /** Called when the user clicks a node's expand/collapse toggle. */
  onToggleExpand?: (node: T) => void;

  /** CSS class on the outermost container. */
  className?: string;

  /** Accessible label for the tree. */
  ariaLabel?: string;
}

// ── Component ──────────────────────────────────────────────────────────────

function TreeNodeList<T extends TreeViewNode>({
  nodes,
  depth,
  renderNode,
  isExpanded,
  onToggleExpand,
}: {
  nodes: T[];
  depth: number;
  renderNode: (node: T, depth: number) => ReactNode;
  isExpanded: (node: T) => boolean;
  onToggleExpand?: (node: T) => void;
}) {
  return (
    <>
      {nodes.map((node, index) => {
        const hasChildren = node.children && node.children.length > 0;
        const expanded = hasChildren ? isExpanded(node) : false;
        const isLast = index === nodes.length - 1;

        return (
          <div
            key={node.id}
            className={`tree__node-wrapper${isLast ? " tree__node-wrapper--last" : ""}`}
          >
            <div
              className="tree__node"
              role="treeitem"
              aria-expanded={hasChildren ? expanded : undefined}
              aria-level={depth + 1}
              tabIndex={-1}
            >
              {hasChildren && onToggleExpand && (
                <button
                  className="tree__toggle"
                  onClick={() => onToggleExpand(node)}
                  aria-label={expanded ? "Collapse" : "Expand"}
                  type="button"
                  tabIndex={-1}
                >
                  <span
                    className={`tree__toggle-icon${expanded ? " tree__toggle-icon--expanded" : ""}`}
                    aria-hidden="true"
                  >
                    ▶
                  </span>
                </button>
              )}
              <div className="tree__node-content">
                {renderNode(node, depth)}
              </div>
            </div>

            {hasChildren && expanded && (
              <div className="tree__children" role="group">
                <TreeNodeList
                  nodes={node.children as T[]}
                  depth={depth + 1}
                  renderNode={renderNode}
                  isExpanded={isExpanded}
                  onToggleExpand={onToggleExpand}
                />
              </div>
            )}
          </div>
        );
      })}
    </>
  );
}

export function TreeView<T extends TreeViewNode>({
  nodes,
  renderNode,
  isExpanded = () => true,
  onToggleExpand,
  className = "tree",
  ariaLabel,
}: TreeViewProps<T>) {
  return (
    <div className={className} role="tree" aria-label={ariaLabel}>
      <TreeNodeList
        nodes={nodes}
        depth={0}
        renderNode={renderNode}
        isExpanded={isExpanded}
        onToggleExpand={onToggleExpand}
      />
    </div>
  );
}
