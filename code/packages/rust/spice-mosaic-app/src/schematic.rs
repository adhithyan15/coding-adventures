//! Canonical schematic capture for the Berkeley SPICE Mosaic workbench.
//!
//! This module deliberately models only grid terminals and endpoint wires. It
//! is an editor-owned representation that always lowers to a Berkeley deck;
//! parsing and simulation remain the shared `spice-netlist-parser` contract.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

/// An integer grid location used by component terminals and wire endpoints.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SchematicPoint {
    pub x: i32,
    pub y: i32,
}

/// The first symbol palette supported by canonical schematic capture.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SchematicComponentKind {
    Resistor,
    Capacitor,
    DcVoltage,
    Ground,
}

impl SchematicComponentKind {
    fn reference_prefix(self) -> char {
        match self {
            Self::Resistor => 'R',
            Self::Capacitor => 'C',
            Self::DcVoltage => 'V',
            Self::Ground => 'G',
        }
    }

    fn terminal_count(self) -> usize {
        match self {
            Self::Ground => 1,
            Self::Resistor | Self::Capacitor | Self::DcVoltage => 2,
        }
    }
}

/// A placed symbol whose terminals are connected by exact grid endpoints.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchematicComponent {
    pub reference: String,
    pub kind: SchematicComponentKind,
    pub value: String,
    pub terminals: Vec<SchematicPoint>,
}

/// A direct connection between two terminal grid endpoints.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchematicWire {
    pub start: SchematicPoint,
    pub end: SchematicPoint,
}

/// A compact editor document that can be lowered into a canonical `.op` deck.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchematicDocument {
    pub title: String,
    pub components: Vec<SchematicComponent>,
    pub wires: Vec<SchematicWire>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchematicError(pub String);

impl fmt::Display for SchematicError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for SchematicError {}

fn invalid(message: impl Into<String>) -> SchematicError {
    SchematicError(message.into())
}

#[derive(Default)]
struct DisjointSet {
    parents: Vec<usize>,
}

impl DisjointSet {
    fn with_len(length: usize) -> Self {
        Self {
            parents: (0..length).collect(),
        }
    }

    fn find(&mut self, index: usize) -> usize {
        if self.parents[index] != index {
            let root = self.find(self.parents[index]);
            self.parents[index] = root;
        }
        self.parents[index]
    }

    fn union(&mut self, left: usize, right: usize) {
        let left = self.find(left);
        let right = self.find(right);
        if left != right {
            self.parents[right] = left;
        }
    }
}

impl SchematicDocument {
    /// Reject incomplete capture state before it can produce an ambiguous deck.
    pub fn validate(&self) -> Result<(), SchematicError> {
        if self.title.trim().is_empty() || self.title.contains(['\n', '\r']) {
            return Err(invalid("schematic title must be one non-empty line"));
        }

        let mut references = BTreeSet::new();
        let mut ground_count = 0;
        for component in &self.components {
            let prefix = component.kind.reference_prefix();
            if !component.reference.starts_with(prefix)
                || component.reference.len() == 1
                || !component
                    .reference
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '_')
            {
                return Err(invalid(format!(
                    "{} reference must begin with {prefix} and use ASCII letters, digits, or _",
                    component.kind.reference_prefix()
                )));
            }
            if !references.insert(component.reference.as_str()) {
                return Err(invalid(format!(
                    "duplicate component reference {}",
                    component.reference
                )));
            }
            if component.terminals.len() != component.kind.terminal_count() {
                return Err(invalid(format!(
                    "{} requires {} terminals",
                    component.reference,
                    component.kind.terminal_count()
                )));
            }
            if component.kind != SchematicComponentKind::Ground
                && (component.value.is_empty() || component.value.chars().any(char::is_whitespace))
            {
                return Err(invalid(format!(
                    "{} value must be one non-empty SPICE token",
                    component.reference
                )));
            }
            if component.kind == SchematicComponentKind::Ground {
                ground_count += 1;
            }
        }
        if ground_count == 0 {
            return Err(invalid("schematic requires at least one ground symbol"));
        }
        if self.wires.iter().any(|wire| wire.start == wire.end) {
            return Err(invalid("schematic wires must have distinct endpoints"));
        }
        Ok(())
    }

    /// Emit a deterministic Berkeley `.op` deck independent of placement order.
    pub fn to_berkeley_netlist(&self) -> Result<String, SchematicError> {
        self.validate()?;

        let mut point_ids = BTreeMap::new();
        for point in self
            .components
            .iter()
            .flat_map(|component| component.terminals.iter().copied())
            .chain(self.wires.iter().flat_map(|wire| [wire.start, wire.end]))
        {
            let next = point_ids.len();
            point_ids.entry(point).or_insert(next);
        }

        let mut sets = DisjointSet::with_len(point_ids.len());
        for wire in &self.wires {
            sets.union(point_ids[&wire.start], point_ids[&wire.end]);
        }

        let ground_points = self
            .components
            .iter()
            .filter(|component| component.kind == SchematicComponentKind::Ground)
            .map(|component| point_ids[&component.terminals[0]])
            .collect::<Vec<_>>();
        let ground_point = ground_points[0];
        for point in ground_points.into_iter().skip(1) {
            sets.union(ground_point, point);
        }
        let ground_root = sets.find(ground_point);

        let mut root_points: BTreeMap<usize, SchematicPoint> = BTreeMap::new();
        for (point, point_id) in &point_ids {
            let root = sets.find(*point_id);
            root_points
                .entry(root)
                .and_modify(|lowest| *lowest = (*lowest).min(*point))
                .or_insert(*point);
        }
        let mut ordered_roots = root_points.into_iter().collect::<Vec<_>>();
        ordered_roots.sort_by_key(|(_, point)| *point);

        let mut names = BTreeMap::new();
        let mut next_net = 1;
        for (root, _) in ordered_roots {
            if root == ground_root {
                names.insert(root, "0".to_owned());
            } else {
                names.insert(root, format!("n{next_net}"));
                next_net += 1;
            }
        }

        let mut components = self.components.iter().collect::<Vec<_>>();
        components.sort_by(|left, right| left.reference.cmp(&right.reference));
        let mut lines = vec![format!("* {}", self.title)];
        for component in components {
            if component.kind == SchematicComponentKind::Ground {
                continue;
            }
            let terminals = component
                .terminals
                .iter()
                .map(|point| {
                    let root = sets.find(point_ids[point]);
                    names[&root].as_str()
                })
                .collect::<Vec<_>>();
            let line = match component.kind {
                SchematicComponentKind::Resistor | SchematicComponentKind::Capacitor => format!(
                    "{} {} {} {}",
                    component.reference, terminals[0], terminals[1], component.value
                ),
                SchematicComponentKind::DcVoltage => format!(
                    "{} {} {} DC {}",
                    component.reference, terminals[0], terminals[1], component.value
                ),
                SchematicComponentKind::Ground => unreachable!("ground symbols are skipped"),
            };
            lines.push(line);
        }
        lines.extend([".op".to_owned(), ".end".to_owned()]);
        Ok(lines.join("\n") + "\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spice_netlist_parser::{parse_netlist, run_netlist};

    fn point(x: i32, y: i32) -> SchematicPoint {
        SchematicPoint { x, y }
    }

    fn rc_document() -> SchematicDocument {
        SchematicDocument {
            title: "RC divider".to_owned(),
            components: vec![
                SchematicComponent {
                    reference: "C1".to_owned(),
                    kind: SchematicComponentKind::Capacitor,
                    value: "1u".to_owned(),
                    terminals: vec![point(50, 20), point(50, 0)],
                },
                SchematicComponent {
                    reference: "G1".to_owned(),
                    kind: SchematicComponentKind::Ground,
                    value: String::new(),
                    terminals: vec![point(10, 0)],
                },
                SchematicComponent {
                    reference: "V1".to_owned(),
                    kind: SchematicComponentKind::DcVoltage,
                    value: "5".to_owned(),
                    terminals: vec![point(0, 20), point(0, 0)],
                },
                SchematicComponent {
                    reference: "R1".to_owned(),
                    kind: SchematicComponentKind::Resistor,
                    value: "1k".to_owned(),
                    terminals: vec![point(10, 20), point(40, 20)],
                },
            ],
            wires: vec![
                SchematicWire {
                    start: point(0, 20),
                    end: point(10, 20),
                },
                SchematicWire {
                    start: point(40, 20),
                    end: point(50, 20),
                },
                SchematicWire {
                    start: point(0, 0),
                    end: point(10, 0),
                },
                SchematicWire {
                    start: point(10, 0),
                    end: point(50, 0),
                },
            ],
        }
    }

    #[test]
    fn lowers_a_wired_rc_document_to_a_runnable_canonical_deck() {
        let deck = rc_document().to_berkeley_netlist().unwrap();
        assert_eq!(
            deck,
            "* RC divider\nC1 n2 0 1u\nR1 n1 n2 1k\nV1 n1 0 DC 5\n.op\n.end\n"
        );
        parse_netlist(&deck).unwrap();
        assert_eq!(run_netlist(&deck).unwrap().len(), 1);
    }

    #[test]
    fn component_and_wire_order_do_not_change_the_deck() {
        let document = rc_document();
        let expected = document.to_berkeley_netlist().unwrap();
        let mut reordered = document;
        reordered.components.reverse();
        reordered.wires.reverse();
        assert_eq!(reordered.to_berkeley_netlist().unwrap(), expected);
    }

    #[test]
    fn all_ground_symbols_resolve_to_spice_ground() {
        let mut document = rc_document();
        document.components.push(SchematicComponent {
            reference: "G2".to_owned(),
            kind: SchematicComponentKind::Ground,
            value: String::new(),
            terminals: vec![point(70, 0)],
        });
        document.components.push(SchematicComponent {
            reference: "R2".to_owned(),
            kind: SchematicComponentKind::Resistor,
            value: "2k".to_owned(),
            terminals: vec![point(40, 20), point(70, 0)],
        });
        assert!(document
            .to_berkeley_netlist()
            .unwrap()
            .contains("R2 n2 0 2k"));
    }

    #[test]
    fn rejects_incomplete_or_ambiguous_capture_state() {
        let mut document = rc_document();
        document
            .components
            .retain(|component| component.kind != SchematicComponentKind::Ground);
        assert_eq!(
            document.validate().unwrap_err().to_string(),
            "schematic requires at least one ground symbol"
        );

        let mut document = rc_document();
        document.components[0].terminals.pop();
        assert_eq!(
            document.validate().unwrap_err().to_string(),
            "C1 requires 2 terminals"
        );

        let mut document = rc_document();
        document.components[0].reference = "Q1".to_owned();
        assert_eq!(
            document.validate().unwrap_err().to_string(),
            "C reference must begin with C and use ASCII letters, digits, or _"
        );

        let mut document = rc_document();
        document.components[0].value = "1 u".to_owned();
        assert_eq!(
            document.validate().unwrap_err().to_string(),
            "C1 value must be one non-empty SPICE token"
        );

        let mut document = rc_document();
        document.wires[0].end = document.wires[0].start;
        assert_eq!(
            document.validate().unwrap_err().to_string(),
            "schematic wires must have distinct endpoints"
        );
    }
}
