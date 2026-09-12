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

/// The symbol palette supported by canonical schematic capture.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SchematicComponentKind {
    Resistor,
    Capacitor,
    Inductor,
    DcVoltage,
    DcCurrent,
    AcVoltage,
    Ground,
}

impl SchematicComponentKind {
    /// Parse the stable palette labels exposed by the Mosaic workbench.
    pub fn from_palette_label(label: &str) -> Result<Self, SchematicError> {
        match label {
            "Resistor" => Ok(Self::Resistor),
            "Capacitor" => Ok(Self::Capacitor),
            "Inductor" => Ok(Self::Inductor),
            "DC source" => Ok(Self::DcVoltage),
            "DC current" => Ok(Self::DcCurrent),
            "AC source" => Ok(Self::AcVoltage),
            "Ground" => Ok(Self::Ground),
            _ => Err(invalid("unknown schematic palette component")),
        }
    }

    /// Human-readable names are part of the host palette contract.
    pub fn palette_label(self) -> &'static str {
        match self {
            Self::Resistor => "Resistor",
            Self::Capacitor => "Capacitor",
            Self::Inductor => "Inductor",
            Self::DcVoltage => "DC source",
            Self::DcCurrent => "DC current",
            Self::AcVoltage => "AC source",
            Self::Ground => "Ground",
        }
    }

    fn reference_prefix(self) -> char {
        match self {
            Self::Resistor => 'R',
            Self::Capacitor => 'C',
            Self::Inductor => 'L',
            Self::DcVoltage | Self::AcVoltage => 'V',
            Self::DcCurrent => 'I',
            Self::Ground => 'G',
        }
    }

    fn terminal_count(self) -> usize {
        match self {
            Self::Ground => 1,
            Self::Resistor
            | Self::Capacitor
            | Self::Inductor
            | Self::DcVoltage
            | Self::DcCurrent
            | Self::AcVoltage => 2,
        }
    }
}

/// A canonical analysis card selected by the schematic session.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Default)]
pub enum SchematicAnalysis {
    #[default]
    OperatingPoint,
    DcSweep,
    AcSweep,
    Transient,
}

impl SchematicAnalysis {
    /// Parse the stable analysis labels exposed by the Mosaic workbench.
    pub fn from_palette_label(label: &str) -> Result<Self, SchematicError> {
        match label {
            "Operating point" => Ok(Self::OperatingPoint),
            "DC sweep" => Ok(Self::DcSweep),
            "AC sweep" => Ok(Self::AcSweep),
            "Transient" => Ok(Self::Transient),
            _ => Err(invalid("unknown schematic analysis")),
        }
    }

    /// Human-readable names are part of the host analysis control contract.
    pub fn palette_label(self) -> &'static str {
        match self {
            Self::OperatingPoint => "Operating point",
            Self::DcSweep => "DC sweep",
            Self::AcSweep => "AC sweep",
            Self::Transient => "Transient",
        }
    }

    fn directive(self) -> &'static str {
        match self {
            Self::OperatingPoint => ".op",
            Self::DcSweep => ".dc V1 0 5 1",
            Self::AcSweep => ".ac dec 10 10 10k",
            Self::Transient => ".tran 1m 10m",
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

/// A compact editor document that can be lowered into a canonical Berkeley deck.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchematicDocument {
    pub title: String,
    pub components: Vec<SchematicComponent>,
    pub wires: Vec<SchematicWire>,
    #[serde(default)]
    pub analysis: SchematicAnalysis,
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
    /// Place one palette component on the next deterministic grid cell.
    ///
    /// Placement does not require a complete runnable circuit. The document is
    /// still validated when it is synchronized into a canonical netlist.
    pub fn place_palette_component(
        &mut self,
        kind: SchematicComponentKind,
    ) -> Result<String, SchematicError> {
        let prefix = kind.reference_prefix();
        let next = self
            .components
            .iter()
            .filter_map(|component| {
                component
                    .reference
                    .strip_prefix(prefix)
                    .and_then(|suffix| suffix.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(0)
            + 1;
        let reference = format!("{prefix}{next}");
        let position = self.components.len() as i32;
        let center = SchematicPoint {
            x: 40 + (position % 4) * 80,
            y: 40 + (position / 4) * 70,
        };
        let terminals = match kind {
            SchematicComponentKind::Resistor
            | SchematicComponentKind::Capacitor
            | SchematicComponentKind::Inductor => vec![
                SchematicPoint {
                    x: center.x - 20,
                    y: center.y,
                },
                SchematicPoint {
                    x: center.x + 20,
                    y: center.y,
                },
            ],
            SchematicComponentKind::DcVoltage
            | SchematicComponentKind::DcCurrent
            | SchematicComponentKind::AcVoltage => vec![
                SchematicPoint {
                    x: center.x,
                    y: center.y - 20,
                },
                SchematicPoint {
                    x: center.x,
                    y: center.y + 20,
                },
            ],
            SchematicComponentKind::Ground => vec![center],
        };
        let value = match kind {
            SchematicComponentKind::Resistor => "1k",
            SchematicComponentKind::Capacitor => "1u",
            SchematicComponentKind::Inductor => "1m",
            SchematicComponentKind::DcVoltage => "5",
            SchematicComponentKind::DcCurrent => "1m",
            SchematicComponentKind::AcVoltage => "1",
            SchematicComponentKind::Ground => "",
        }
        .to_owned();
        self.components.push(SchematicComponent {
            reference: reference.clone(),
            kind,
            value,
            terminals,
        });
        Ok(reference)
    }

    /// Connect the nearest terminals of two selected components.
    ///
    /// The wire remains a semantic endpoint connection. Renderers derive an
    /// orthogonal route from those endpoints, so the canonical deck remains
    /// independent of presentation geometry.
    pub fn route_components(
        &mut self,
        start_reference: &str,
        end_reference: &str,
    ) -> Result<SchematicWire, SchematicError> {
        if start_reference == end_reference {
            return Err(invalid("schematic route requires two different components"));
        }
        let start = self
            .components
            .iter()
            .find(|component| component.reference == start_reference)
            .ok_or_else(|| invalid("schematic route start reference is unknown"))?;
        let end = self
            .components
            .iter()
            .find(|component| component.reference == end_reference)
            .ok_or_else(|| invalid("schematic route target reference is unknown"))?;
        let wire = start
            .terminals
            .iter()
            .flat_map(|left| {
                end.terminals.iter().map(move |right| SchematicWire {
                    start: *left,
                    end: *right,
                })
            })
            .filter(|wire| wire.start != wire.end)
            .min_by_key(|wire| {
                (
                    (wire.start.x - wire.end.x).abs() + (wire.start.y - wire.end.y).abs(),
                    wire.start,
                    wire.end,
                )
            })
            .ok_or_else(|| invalid("selected components already share a terminal"))?;
        if self.wires.iter().any(|existing| {
            (existing.start == wire.start && existing.end == wire.end)
                || (existing.start == wire.end && existing.end == wire.start)
        }) {
            return Err(invalid("selected component terminals are already routed"));
        }
        self.connect_wire(wire)?;
        Ok(wire)
    }

    /// Add one endpoint-only wire between terminals that belong to this document.
    ///
    /// This is the common admission boundary for raw host wiring and routed
    /// component connections. Keeping it here prevents serialized editor state
    /// from introducing phantom net points during Berkeley deck lowering.
    pub fn connect_wire(&mut self, wire: SchematicWire) -> Result<(), SchematicError> {
        self.validate_wire_endpoints(&wire)?;
        if self.has_wire(&wire) {
            return Err(invalid("schematic wire is already connected"));
        }
        self.wires.push(wire);
        Ok(())
    }

    fn validate_wire_endpoints(&self, wire: &SchematicWire) -> Result<(), SchematicError> {
        if wire.start == wire.end {
            return Err(invalid("schematic wires must have distinct endpoints"));
        }
        let terminal_points = self
            .components
            .iter()
            .flat_map(|component| component.terminals.iter().copied())
            .collect::<BTreeSet<_>>();
        if !terminal_points.contains(&wire.start) || !terminal_points.contains(&wire.end) {
            return Err(invalid(
                "schematic wire endpoints must be component terminals",
            ));
        }
        Ok(())
    }

    fn has_wire(&self, wire: &SchematicWire) -> bool {
        self.wires.iter().any(|existing| {
            (existing.start == wire.start && existing.end == wire.end)
                || (existing.start == wire.end && existing.end == wire.start)
        })
    }

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
        for (index, wire) in self.wires.iter().enumerate() {
            self.validate_wire_endpoints(wire)?;
            if self.wires[..index].iter().any(|existing| {
                (existing.start == wire.start && existing.end == wire.end)
                    || (existing.start == wire.end && existing.end == wire.start)
            }) {
                return Err(invalid("schematic wire is already connected"));
            }
        }
        Ok(())
    }

    /// Emit a deterministic Berkeley deck independent of placement order.
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
                SchematicComponentKind::Resistor
                | SchematicComponentKind::Capacitor
                | SchematicComponentKind::Inductor => format!(
                    "{} {} {} {}",
                    component.reference, terminals[0], terminals[1], component.value
                ),
                SchematicComponentKind::DcVoltage => format!(
                    "{} {} {} DC {}",
                    component.reference, terminals[0], terminals[1], component.value
                ),
                SchematicComponentKind::DcCurrent => format!(
                    "{} {} {} DC {}",
                    component.reference, terminals[0], terminals[1], component.value
                ),
                SchematicComponentKind::AcVoltage => format!(
                    "{} {} {} AC {}",
                    component.reference, terminals[0], terminals[1], component.value
                ),
                SchematicComponentKind::Ground => unreachable!("ground symbols are skipped"),
            };
            lines.push(line);
        }
        lines.extend([self.analysis.directive().to_owned(), ".end".to_owned()]);
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
            analysis: SchematicAnalysis::OperatingPoint,
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
    fn palette_placement_and_component_routing_are_deterministic() {
        let mut document = SchematicDocument {
            title: "Palette routing".to_owned(),
            components: Vec::new(),
            wires: Vec::new(),
            analysis: SchematicAnalysis::default(),
        };
        assert_eq!(
            document
                .place_palette_component(SchematicComponentKind::Resistor)
                .unwrap(),
            "R1"
        );
        assert_eq!(
            document
                .place_palette_component(SchematicComponentKind::Capacitor)
                .unwrap(),
            "C1"
        );
        assert_eq!(
            document
                .place_palette_component(SchematicComponentKind::Inductor)
                .unwrap(),
            "L1"
        );
        assert_eq!(
            document
                .place_palette_component(SchematicComponentKind::DcCurrent)
                .unwrap(),
            "I1"
        );
        assert_eq!(
            document
                .place_palette_component(SchematicComponentKind::AcVoltage)
                .unwrap(),
            "V1"
        );
        let wire = document.route_components("R1", "C1").unwrap();
        assert_eq!(wire.start, point(60, 40));
        assert_eq!(wire.end, point(100, 40));
        assert_eq!(
            document
                .route_components("R1", "C1")
                .unwrap_err()
                .to_string(),
            "selected component terminals are already routed"
        );
    }

    #[test]
    fn expanded_palette_and_analysis_controls_lower_to_runnable_ac_deck() {
        let document = SchematicDocument {
            title: "RLC sweep".to_owned(),
            components: vec![
                SchematicComponent {
                    reference: "V1".to_owned(),
                    kind: SchematicComponentKind::AcVoltage,
                    value: "1".to_owned(),
                    terminals: vec![point(0, 20), point(0, 0)],
                },
                SchematicComponent {
                    reference: "L1".to_owned(),
                    kind: SchematicComponentKind::Inductor,
                    value: "1m".to_owned(),
                    terminals: vec![point(0, 20), point(20, 20)],
                },
                SchematicComponent {
                    reference: "I1".to_owned(),
                    kind: SchematicComponentKind::DcCurrent,
                    value: "1m".to_owned(),
                    terminals: vec![point(0, 20), point(0, 0)],
                },
                SchematicComponent {
                    reference: "R1".to_owned(),
                    kind: SchematicComponentKind::Resistor,
                    value: "10".to_owned(),
                    terminals: vec![point(20, 20), point(20, 0)],
                },
                SchematicComponent {
                    reference: "G1".to_owned(),
                    kind: SchematicComponentKind::Ground,
                    value: String::new(),
                    terminals: vec![point(0, 0)],
                },
            ],
            wires: vec![SchematicWire {
                start: point(20, 0),
                end: point(0, 0),
            }],
            analysis: SchematicAnalysis::AcSweep,
        };
        let deck = document.to_berkeley_netlist().unwrap();
        assert_eq!(
            deck,
            "* RLC sweep\nI1 n1 0 DC 1m\nL1 n1 n2 1m\nR1 n2 0 10\nV1 n1 0 AC 1\n.ac dec 10 10 10k\n.end\n"
        );
        parse_netlist(&deck).unwrap();
        assert_eq!(run_netlist(&deck).unwrap().len(), 1);

        for (analysis, directive) in [
            (SchematicAnalysis::OperatingPoint, ".op"),
            (SchematicAnalysis::DcSweep, ".dc V1 0 5 1"),
            (SchematicAnalysis::Transient, ".tran 1m 10m"),
        ] {
            let mut controlled = document.clone();
            controlled.analysis = analysis;
            assert!(controlled
                .to_berkeley_netlist()
                .unwrap()
                .contains(directive));
        }
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

        let mut document = rc_document();
        document.wires[0].end = point(99, 99);
        assert_eq!(
            document.validate().unwrap_err().to_string(),
            "schematic wire endpoints must be component terminals"
        );

        let mut document = rc_document();
        let duplicate = SchematicWire {
            start: document.wires[0].end,
            end: document.wires[0].start,
        };
        document.wires.push(duplicate);
        assert_eq!(
            document.validate().unwrap_err().to_string(),
            "schematic wire is already connected"
        );

        let mut document = rc_document();
        let duplicate = SchematicWire {
            start: document.wires[0].end,
            end: document.wires[0].start,
        };
        assert_eq!(
            document.connect_wire(duplicate).unwrap_err().to_string(),
            "schematic wire is already connected"
        );
    }
}
