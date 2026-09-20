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
    Diode,
    Bjt,
    Jfet,
    Mosfet,
    DcVoltage,
    DcCurrent,
    AcVoltage,
    Ground,
}

/// The semiconductor polarity selected for transistor symbols.
///
/// The owning component kind determines the Berkeley model family: BJT uses
/// NPN/PNP, JFET uses NJF/PJF, and MOSFET uses NMOS/PMOS.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum SchematicModelPolarity {
    #[default]
    N,
    P,
}

/// A finite, family-specific model-card field exposed by schematic capture.
///
/// This deliberately covers the primary and corpus-backed Berkeley parameters
/// for each built-in nonlinear symbol. Vendor libraries and arbitrary
/// parameter names remain a separate capture concern.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum SchematicModelParameter {
    SaturationCurrent,
    EmissionCoefficient,
    ZeroBiasJunctionCapacitance,
    TransitTime,
    ForwardBeta,
    BjtBaseEmitterCapacitance,
    BjtForwardTransitTime,
    JfetTransconductance,
    ThresholdVoltage,
    ChannelLengthModulation,
    JfetGateSourceCapacitance,
    JfetGateDrainCapacitance,
    MosfetTransconductance,
    MosfetSubstrateDoping,
    MosfetDrainBulkCapacitance,
    MosfetGateSourceOverlapCapacitance,
    MosfetGateDrainOverlapCapacitance,
    MosfetGateBulkOverlapCapacitance,
    MosfetSourceBulkCapacitance,
    MosfetBulkJunctionPotential,
    MosfetBulkJunctionGradingCoefficient,
}

impl SchematicModelParameter {
    pub fn key(self) -> &'static str {
        match self {
            Self::SaturationCurrent => "IS",
            Self::EmissionCoefficient => "N",
            Self::ZeroBiasJunctionCapacitance => "CJO",
            Self::TransitTime => "TT",
            Self::ForwardBeta => "BF",
            Self::BjtBaseEmitterCapacitance => "CJE",
            Self::BjtForwardTransitTime => "TF",
            Self::JfetTransconductance => "BETA",
            Self::ThresholdVoltage => "VTO",
            Self::ChannelLengthModulation => "LAMBDA",
            Self::JfetGateSourceCapacitance => "CGS",
            Self::JfetGateDrainCapacitance => "CGD",
            Self::MosfetTransconductance => "KP",
            Self::MosfetSubstrateDoping => "NSUB",
            Self::MosfetDrainBulkCapacitance => "CBD",
            Self::MosfetGateSourceOverlapCapacitance => "CGSO",
            Self::MosfetGateDrainOverlapCapacitance => "CGDO",
            Self::MosfetGateBulkOverlapCapacitance => "CGBO",
            Self::MosfetSourceBulkCapacitance => "CBS",
            Self::MosfetBulkJunctionPotential => "PB",
            Self::MosfetBulkJunctionGradingCoefficient => "MJ",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::SaturationCurrent => "Saturation current (IS)",
            Self::EmissionCoefficient => "Emission coefficient (N)",
            Self::ZeroBiasJunctionCapacitance => "Zero-bias junction capacitance (CJO)",
            Self::TransitTime => "Transit time (TT)",
            Self::ForwardBeta => "Forward beta (BF)",
            Self::BjtBaseEmitterCapacitance => "Base-emitter capacitance (CJE)",
            Self::BjtForwardTransitTime => "Forward transit time (TF)",
            Self::JfetTransconductance => "Transconductance (BETA)",
            Self::ThresholdVoltage => "Threshold voltage (VTO)",
            Self::ChannelLengthModulation => "Channel-length modulation (LAMBDA)",
            Self::JfetGateSourceCapacitance => "Gate-source capacitance (CGS)",
            Self::JfetGateDrainCapacitance => "Gate-drain capacitance (CGD)",
            Self::MosfetTransconductance => "Transconductance (KP)",
            Self::MosfetSubstrateDoping => "Substrate doping (NSUB)",
            Self::MosfetDrainBulkCapacitance => "Drain-bulk capacitance (CBD)",
            Self::MosfetGateSourceOverlapCapacitance => "Gate-source overlap capacitance (CGSO)",
            Self::MosfetGateDrainOverlapCapacitance => "Gate-drain overlap capacitance (CGDO)",
            Self::MosfetGateBulkOverlapCapacitance => "Gate-bulk overlap capacitance (CGBO)",
            Self::MosfetSourceBulkCapacitance => "Source-bulk capacitance (CBS)",
            Self::MosfetBulkJunctionPotential => "Bulk junction potential (PB)",
            Self::MosfetBulkJunctionGradingCoefficient => "Bulk junction grading coefficient (MJ)",
        }
    }

    fn for_kind(kind: SchematicComponentKind) -> &'static [Self] {
        match kind {
            SchematicComponentKind::Diode => &[
                Self::SaturationCurrent,
                Self::EmissionCoefficient,
                Self::ZeroBiasJunctionCapacitance,
                Self::TransitTime,
            ],
            SchematicComponentKind::Bjt => &[
                Self::SaturationCurrent,
                Self::ForwardBeta,
                Self::BjtBaseEmitterCapacitance,
                Self::BjtForwardTransitTime,
            ],
            SchematicComponentKind::Jfet => &[
                Self::JfetTransconductance,
                Self::ThresholdVoltage,
                Self::ChannelLengthModulation,
                Self::JfetGateSourceCapacitance,
                Self::JfetGateDrainCapacitance,
            ],
            SchematicComponentKind::Mosfet => &[
                Self::ThresholdVoltage,
                Self::MosfetTransconductance,
                Self::ChannelLengthModulation,
                Self::MosfetSubstrateDoping,
                Self::MosfetDrainBulkCapacitance,
                Self::MosfetGateSourceOverlapCapacitance,
                Self::MosfetGateDrainOverlapCapacitance,
                Self::MosfetGateBulkOverlapCapacitance,
                Self::MosfetSourceBulkCapacitance,
                Self::MosfetBulkJunctionPotential,
                Self::MosfetBulkJunctionGradingCoefficient,
            ],
            _ => &[],
        }
    }
}

impl SchematicModelPolarity {
    /// Parse the stable host-control values.
    pub fn from_host_value(value: &str) -> Result<Self, SchematicError> {
        match value {
            "N" => Ok(Self::N),
            "P" => Ok(Self::P),
            _ => Err(invalid("schematic model polarity must be N or P")),
        }
    }

    /// Return the Berkeley model keyword for one transistor family.
    fn model_keyword(self, kind: SchematicComponentKind) -> Option<&'static str> {
        match (kind, self) {
            (SchematicComponentKind::Bjt, Self::N) => Some("NPN"),
            (SchematicComponentKind::Bjt, Self::P) => Some("PNP"),
            (SchematicComponentKind::Jfet, Self::N) => Some("NJF"),
            (SchematicComponentKind::Jfet, Self::P) => Some("PJF"),
            (SchematicComponentKind::Mosfet, Self::N) => Some("NMOS"),
            (SchematicComponentKind::Mosfet, Self::P) => Some("PMOS"),
            _ => None,
        }
    }

    /// Return the inspector label for one transistor family.
    pub fn label(self, kind: SchematicComponentKind) -> Option<&'static str> {
        self.model_keyword(kind)
    }
}

impl SchematicComponentKind {
    /// Parse the stable palette labels exposed by the Mosaic workbench.
    pub fn from_palette_label(label: &str) -> Result<Self, SchematicError> {
        match label {
            "Resistor" => Ok(Self::Resistor),
            "Capacitor" => Ok(Self::Capacitor),
            "Inductor" => Ok(Self::Inductor),
            "Diode" => Ok(Self::Diode),
            "BJT" => Ok(Self::Bjt),
            "JFET" => Ok(Self::Jfet),
            "MOSFET" => Ok(Self::Mosfet),
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
            Self::Diode => "Diode",
            Self::Bjt => "BJT",
            Self::Jfet => "JFET",
            Self::Mosfet => "MOSFET",
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
            Self::Diode => 'D',
            Self::Bjt => 'Q',
            Self::Jfet => 'J',
            Self::Mosfet => 'M',
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
            | Self::Diode
            | Self::DcVoltage
            | Self::DcCurrent
            | Self::AcVoltage => 2,
            Self::Bjt | Self::Jfet => 3,
            Self::Mosfet => 4,
        }
    }

    fn is_transistor(self) -> bool {
        matches!(self, Self::Bjt | Self::Jfet | Self::Mosfet)
    }

    pub fn is_nonlinear(self) -> bool {
        matches!(self, Self::Diode | Self::Bjt | Self::Jfet | Self::Mosfet)
    }

    /// Return the N- and P-type model labels for inspector controls.
    pub fn model_polarity_labels(self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::Bjt => Some(("NPN", "PNP")),
            Self::Jfet => Some(("NJF", "PJF")),
            Self::Mosfet => Some(("NMOS", "PMOS")),
            _ => None,
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
    TransferFunction,
}

impl SchematicAnalysis {
    /// Parse the stable analysis labels exposed by the Mosaic workbench.
    pub fn from_palette_label(label: &str) -> Result<Self, SchematicError> {
        match label {
            "Operating point" => Ok(Self::OperatingPoint),
            "DC sweep" => Ok(Self::DcSweep),
            "AC sweep" => Ok(Self::AcSweep),
            "Transient" => Ok(Self::Transient),
            "Transfer function" => Ok(Self::TransferFunction),
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
            Self::TransferFunction => "Transfer function",
        }
    }

    fn parameter_labels(self) -> [&'static str; 3] {
        match self {
            Self::OperatingPoint => ["", "", ""],
            Self::DcSweep => ["Start", "Stop", "Step"],
            Self::AcSweep => ["Points per decade", "Start frequency", "Stop frequency"],
            Self::Transient => ["Time step", "Stop time", ""],
            Self::TransferFunction => ["Output node", "", ""],
        }
    }

    fn probe_selector(self) -> &'static str {
        match self {
            Self::OperatingPoint => "op",
            Self::DcSweep => "dc",
            Self::AcSweep => "ac",
            Self::Transient => "tran",
            Self::TransferFunction => "tf",
        }
    }
}

/// Persisted Berkeley card values for the selected schematic analysis.
///
/// The defaults reproduce the initial palette controls while allowing hosts to
/// edit real sweep cards without manufacturing a second netlist format.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchematicAnalysisSettings {
    pub dc_source: String,
    pub dc_start: String,
    pub dc_stop: String,
    pub dc_step: String,
    pub ac_points_per_decade: String,
    pub ac_start_frequency: String,
    pub ac_stop_frequency: String,
    pub transient_time_step: String,
    pub transient_stop_time: String,
    #[serde(default)]
    pub tf_output_node: String,
    #[serde(default = "default_tf_source")]
    pub tf_source: String,
}

fn default_tf_source() -> String {
    "V1".to_owned()
}

impl Default for SchematicAnalysisSettings {
    fn default() -> Self {
        Self {
            dc_source: "V1".to_owned(),
            dc_start: "0".to_owned(),
            dc_stop: "5".to_owned(),
            dc_step: "1".to_owned(),
            ac_points_per_decade: "10".to_owned(),
            ac_start_frequency: "10".to_owned(),
            ac_stop_frequency: "10k".to_owned(),
            transient_time_step: "1m".to_owned(),
            transient_stop_time: "10m".to_owned(),
            tf_output_node: String::new(),
            tf_source: default_tf_source(),
        }
    }
}

/// One ordered Berkeley analysis card owned by a schematic document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchematicAnalysisCard {
    #[serde(default)]
    pub id: u64,
    pub analysis: SchematicAnalysis,
    #[serde(default)]
    pub settings: SchematicAnalysisSettings,
}

impl SchematicAnalysisCard {
    fn new(id: u64, analysis: SchematicAnalysis) -> Self {
        Self {
            id,
            analysis,
            settings: SchematicAnalysisSettings::default(),
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

/// A user-authored Berkeley node name anchored to a component terminal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchematicNetLabel {
    pub point: SchematicPoint,
    pub name: String,
}

/// One schematic-owned result signal retained by a global Berkeley `.save` card.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SchematicOutputProbe {
    Voltage {
        node: String,
    },
    DifferentialVoltage {
        positive_node: String,
        negative_node: String,
    },
    Current {
        source: String,
    },
}

impl SchematicOutputProbe {
    /// The canonical Berkeley probe token shown by host controls and deck output.
    pub fn token(&self) -> String {
        match self {
            Self::Voltage { node } => format!("V({node})"),
            Self::DifferentialVoltage {
                positive_node,
                negative_node,
            } => format!("V({positive_node},{negative_node})"),
            Self::Current { source } => format!("I({source})"),
        }
    }
}

/// Ordered output probes that apply only to one durable analysis-card identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchematicScopedOutputProbes {
    pub analysis_card_id: u64,
    #[serde(default)]
    pub probes: Vec<SchematicOutputProbe>,
}

/// A compact editor document that can be lowered into a canonical Berkeley deck.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchematicDocument {
    pub title: String,
    pub components: Vec<SchematicComponent>,
    pub wires: Vec<SchematicWire>,
    #[serde(default)]
    pub net_labels: Vec<SchematicNetLabel>,
    #[serde(default)]
    pub output_probes: Vec<SchematicOutputProbe>,
    #[serde(default)]
    pub scoped_output_probes: Vec<SchematicScopedOutputProbes>,
    /// Explicit transistor-polarity selections keyed by component reference.
    /// Missing legacy entries retain the N-type default.
    #[serde(default)]
    pub model_polarities: BTreeMap<String, SchematicModelPolarity>,
    /// Typed primary model-card fields keyed by component reference.
    /// Missing legacy entries continue to lower from the scalar component value.
    #[serde(default)]
    pub model_parameters: BTreeMap<String, BTreeMap<SchematicModelParameter, String>>,
    #[serde(default)]
    pub analysis: SchematicAnalysis,
    #[serde(default)]
    pub analysis_settings: SchematicAnalysisSettings,
    /// Ordered canonical cards. An empty list retains legacy single-card state.
    #[serde(default)]
    pub analysis_cards: Vec<SchematicAnalysisCard>,
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

fn legacy_model_parameter(value: &str, parameter: SchematicModelParameter) -> Option<String> {
    let (name, value) = value.split_once('=')?;
    (name.eq_ignore_ascii_case(parameter.key()) && !value.is_empty()).then(|| value.to_owned())
}

fn analysis_parameter_values(
    analysis: SchematicAnalysis,
    settings: &SchematicAnalysisSettings,
) -> [&str; 3] {
    match analysis {
        SchematicAnalysis::OperatingPoint => ["", "", ""],
        SchematicAnalysis::DcSweep => [&settings.dc_start, &settings.dc_stop, &settings.dc_step],
        SchematicAnalysis::AcSweep => [
            &settings.ac_points_per_decade,
            &settings.ac_start_frequency,
            &settings.ac_stop_frequency,
        ],
        SchematicAnalysis::Transient => [
            &settings.transient_time_step,
            &settings.transient_stop_time,
            "",
        ],
        SchematicAnalysis::TransferFunction => [&settings.tf_output_node, "", ""],
    }
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
    /// Return ordered cards while preserving legacy persisted documents.
    pub fn analysis_cards(&self) -> Vec<SchematicAnalysisCard> {
        if self.analysis_cards.is_empty() {
            vec![SchematicAnalysisCard {
                id: 1,
                analysis: self.analysis,
                settings: self.analysis_settings.clone(),
            }]
        } else {
            self.analysis_cards.clone()
        }
    }

    /// Return the number of cards available to schematic hosts.
    pub fn analysis_card_count(&self) -> usize {
        self.analysis_cards.len().max(1)
    }

    /// Return source-order labels for card-selection controls.
    pub fn analysis_card_labels(&self) -> Vec<String> {
        self.analysis_cards()
            .into_iter()
            .enumerate()
            .map(|(index, card)| format!("{}. {}", index + 1, card.analysis.palette_label()))
            .collect()
    }

    /// Return one card by its source-order position.
    pub fn analysis_card(&self, index: usize) -> Option<SchematicAnalysisCard> {
        self.analysis_cards().into_iter().nth(index)
    }

    fn materialized_analysis_cards(&mut self) -> &mut Vec<SchematicAnalysisCard> {
        if self.analysis_cards.is_empty() {
            self.analysis_cards.push(SchematicAnalysisCard {
                id: 1,
                analysis: self.analysis,
                settings: self.analysis_settings.clone(),
            });
        }
        let mut assigned = BTreeSet::new();
        let mut next = 1_u64;
        for card in &mut self.analysis_cards {
            if card.id == 0 || !assigned.insert(card.id) {
                while assigned.contains(&next) {
                    next += 1;
                }
                card.id = next;
                assigned.insert(next);
            }
            next = next.max(card.id.saturating_add(1));
        }
        &mut self.analysis_cards
    }

    /// Materialize legacy single-card fields for persisted ordered-plan state.
    pub fn migrate_legacy_analysis_cards(&mut self) {
        self.materialized_analysis_cards();
    }

    fn analysis_card_mut(
        &mut self,
        index: usize,
    ) -> Result<&mut SchematicAnalysisCard, SchematicError> {
        self.materialized_analysis_cards()
            .get_mut(index)
            .ok_or_else(|| invalid("schematic analysis card is unavailable"))
    }

    /// Append a card and return its source-order index.
    pub fn add_analysis_card(&mut self, analysis: SchematicAnalysis) -> usize {
        let cards = self.materialized_analysis_cards();
        let id = cards.iter().map(|card| card.id).max().unwrap_or(0) + 1;
        cards.push(SchematicAnalysisCard::new(id, analysis));
        cards.len() - 1
    }

    /// Remove one card while keeping every schematic runnable by default.
    pub fn remove_analysis_card(&mut self, index: usize) -> Result<(), SchematicError> {
        let cards = self.materialized_analysis_cards();
        if cards.len() == 1 {
            return Err(invalid("schematic requires at least one analysis card"));
        }
        if index >= cards.len() {
            return Err(invalid("schematic analysis card is unavailable"));
        }
        let removed = cards.remove(index);
        self.scoped_output_probes
            .retain(|scope| scope.analysis_card_id != removed.id);
        Ok(())
    }

    /// Move one card to a new source-order position without rewriting its settings.
    pub fn move_analysis_card(&mut self, from: usize, to: usize) -> Result<(), SchematicError> {
        let cards = self.materialized_analysis_cards();
        if from >= cards.len() || to >= cards.len() {
            return Err(invalid("schematic analysis card is unavailable"));
        }
        if from != to {
            let card = cards.remove(from);
            cards.insert(to, card);
        }
        Ok(())
    }

    /// Change a card kind without disturbing other source-ordered cards.
    pub fn set_analysis_card_kind(
        &mut self,
        index: usize,
        analysis: SchematicAnalysis,
    ) -> Result<(), SchematicError> {
        self.analysis_card_mut(index)?.analysis = analysis;
        Ok(())
    }

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
            | SchematicComponentKind::Inductor
            | SchematicComponentKind::Diode => vec![
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
            SchematicComponentKind::Bjt | SchematicComponentKind::Jfet => vec![
                SchematicPoint {
                    x: center.x - 20,
                    y: center.y,
                },
                SchematicPoint {
                    x: center.x + 20,
                    y: center.y - 20,
                },
                SchematicPoint {
                    x: center.x + 20,
                    y: center.y + 20,
                },
            ],
            SchematicComponentKind::Mosfet => vec![
                SchematicPoint {
                    x: center.x - 20,
                    y: center.y,
                },
                SchematicPoint {
                    x: center.x + 20,
                    y: center.y - 20,
                },
                SchematicPoint {
                    x: center.x + 20,
                    y: center.y + 20,
                },
                SchematicPoint {
                    x: center.x,
                    y: center.y + 40,
                },
            ],
            SchematicComponentKind::Ground => vec![center],
        };
        let value = match kind {
            SchematicComponentKind::Resistor => "1k",
            SchematicComponentKind::Capacitor => "1u",
            SchematicComponentKind::Inductor => "1m",
            SchematicComponentKind::Diode => "IS=1e-14",
            SchematicComponentKind::Bjt => "BF=100",
            SchematicComponentKind::Jfet => "BETA=1m",
            SchematicComponentKind::Mosfet => "VTO=0.7",
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

    /// Connect two explicitly selected terminals on different components.
    pub fn route_terminals(
        &mut self,
        start_reference: &str,
        start_terminal: usize,
        end_reference: &str,
        end_terminal: usize,
    ) -> Result<SchematicWire, SchematicError> {
        if start_reference == end_reference {
            return Err(invalid("schematic route requires two different components"));
        }
        let start = self
            .components
            .iter()
            .find(|component| component.reference == start_reference)
            .ok_or_else(|| invalid("schematic route start reference is unknown"))?
            .terminals
            .get(start_terminal)
            .copied()
            .ok_or_else(|| invalid("schematic route start terminal is unavailable"))?;
        let end = self
            .components
            .iter()
            .find(|component| component.reference == end_reference)
            .ok_or_else(|| invalid("schematic route target reference is unknown"))?
            .terminals
            .get(end_terminal)
            .copied()
            .ok_or_else(|| invalid("schematic route target terminal is unavailable"))?;
        let wire = SchematicWire { start, end };
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
        if let Err(error) = self.validate_net_labels() {
            self.wires.pop();
            return Err(error);
        }
        Ok(())
    }

    /// Remove one component and every endpoint wire incident to it.
    ///
    /// An incomplete document remains editable; synchronization continues to
    /// validate requirements such as a ground symbol and runnable analysis.
    pub fn remove_component(&mut self, reference: &str) -> Result<usize, SchematicError> {
        let index = self
            .components
            .iter()
            .position(|component| component.reference == reference)
            .ok_or_else(|| invalid("schematic component reference is unknown"))?;
        let component = self.components.remove(index);
        self.model_polarities.remove(reference);
        self.model_parameters.remove(reference);
        let terminals = component.terminals.into_iter().collect::<BTreeSet<_>>();
        let wire_count = self.wires.len();
        self.wires
            .retain(|wire| !terminals.contains(&wire.start) && !terminals.contains(&wire.end));
        self.net_labels
            .retain(|label| !terminals.contains(&label.point));
        Ok(wire_count - self.wires.len())
    }

    /// Remove one endpoint wire by its displayed source-order position.
    pub fn remove_wire(&mut self, index: usize) -> Result<SchematicWire, SchematicError> {
        if index >= self.wires.len() {
            return Err(invalid("schematic wire is unavailable"));
        }
        Ok(self.wires.remove(index))
    }

    /// Return the explicit name for the net containing one terminal, if any.
    pub fn net_label_at(&self, point: SchematicPoint) -> Option<&str> {
        let (point_ids, mut sets, _) = self.net_topology();
        let root = sets.find(*point_ids.get(&point)?);
        self.net_labels
            .iter()
            .find(|label| {
                point_ids
                    .get(&label.point)
                    .is_some_and(|point_id| sets.find(*point_id) == root)
            })
            .map(|label| label.name.as_str())
    }

    /// Set or clear a Berkeley node name on a component terminal.
    ///
    /// An empty name removes the label, which lets text controls clear an
    /// accidental label without a second destructive action. Matching labels
    /// on disconnected terminal nets lower as one Berkeley node.
    pub fn set_net_label(
        &mut self,
        point: SchematicPoint,
        name: &str,
    ) -> Result<(), SchematicError> {
        if !self.terminal_points().contains(&point) {
            return Err(invalid(
                "schematic net labels must target component terminals",
            ));
        }
        let name = name.trim();
        if !name.is_empty() {
            Self::validate_net_label_name(name)?;
        }
        let previous = self.net_labels.clone();
        let (point_ids, mut sets, _) = self.net_topology();
        let root = sets.find(point_ids[&point]);
        self.net_labels.retain(|label| {
            point_ids
                .get(&label.point)
                .is_none_or(|point_id| sets.find(*point_id) != root)
        });
        if !name.is_empty() {
            self.net_labels.push(SchematicNetLabel {
                point,
                name: name.to_owned(),
            });
        }
        if let Err(error) = self.validate_net_labels() {
            self.net_labels = previous;
            return Err(error);
        }
        Ok(())
    }

    /// Update the one-line title used by document I/O and deck comments.
    pub fn set_title(&mut self, title: &str) -> Result<(), SchematicError> {
        if title.trim().is_empty() || title.contains(['\n', '\r']) {
            return Err(invalid("schematic title must be one non-empty line"));
        }
        self.title = title.to_owned();
        Ok(())
    }

    /// Rename one component without leaving a source-controlled card behind on its old source.
    pub fn rename_component(
        &mut self,
        current_reference: &str,
        new_reference: &str,
    ) -> Result<usize, SchematicError> {
        let index = self
            .components
            .iter()
            .position(|component| component.reference == current_reference)
            .ok_or_else(|| invalid("schematic component reference is unknown"))?;
        let kind = self.components[index].kind;
        let prefix = kind.reference_prefix();
        if !new_reference.starts_with(prefix)
            || new_reference.len() == 1
            || !new_reference
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            return Err(invalid(format!(
                "{new_reference} reference must begin with {prefix} and use ASCII letters, digits, or _"
            )));
        }
        if self
            .components
            .iter()
            .enumerate()
            .any(|(other_index, component)| {
                other_index != index && component.reference == new_reference
            })
        {
            return Err(invalid(format!(
                "duplicate component reference {new_reference}"
            )));
        }
        let old_reference = std::mem::replace(
            &mut self.components[index].reference,
            new_reference.to_owned(),
        );
        let mut updated_cards = 0;
        if self.analysis_cards.is_empty() {
            let mut updated = false;
            if self.analysis_settings.dc_source == old_reference {
                self.analysis_settings.dc_source = new_reference.to_owned();
                updated = true;
            }
            if self.analysis == SchematicAnalysis::TransferFunction
                && self.analysis_settings.tf_source == old_reference
            {
                self.analysis_settings.tf_source = new_reference.to_owned();
                updated = true;
            }
            updated_cards += usize::from(updated);
        } else {
            for card in &mut self.analysis_cards {
                let mut updated = false;
                if card.settings.dc_source == old_reference {
                    card.settings.dc_source = new_reference.to_owned();
                    updated = true;
                }
                if card.analysis == SchematicAnalysis::TransferFunction
                    && card.settings.tf_source == old_reference
                {
                    card.settings.tf_source = new_reference.to_owned();
                    updated = true;
                }
                updated_cards += usize::from(updated);
            }
        }
        for probe in &mut self.output_probes {
            if let SchematicOutputProbe::Current { source } = probe {
                if *source == old_reference {
                    *source = new_reference.to_owned();
                }
            }
        }
        for scope in &mut self.scoped_output_probes {
            for probe in &mut scope.probes {
                if let SchematicOutputProbe::Current { source } = probe {
                    if *source == old_reference {
                        *source = new_reference.to_owned();
                    }
                }
            }
        }
        if let Some(polarity) = self.model_polarities.remove(&old_reference) {
            self.model_polarities
                .insert(new_reference.to_owned(), polarity);
        }
        if let Some(parameters) = self.model_parameters.remove(&old_reference) {
            self.model_parameters
                .insert(new_reference.to_owned(), parameters);
        }
        Ok(updated_cards)
    }

    /// Update the SPICE value of one selected, non-ground component.
    pub fn set_component_value(
        &mut self,
        reference: &str,
        value: &str,
    ) -> Result<(), SchematicError> {
        let component = self
            .components
            .iter_mut()
            .find(|component| component.reference == reference)
            .ok_or_else(|| invalid("schematic component reference is unknown"))?;
        if component.kind == SchematicComponentKind::Ground {
            return Err(invalid(format!(
                "{} ground symbol does not accept a SPICE value",
                component.reference
            )));
        }
        if value.is_empty() || value.chars().any(char::is_whitespace) {
            return Err(invalid(format!(
                "{} value must be one non-empty SPICE token",
                component.reference
            )));
        }
        component.value = value.to_owned();
        Ok(())
    }

    /// Return the selected semiconductor polarity for one component.
    pub fn component_model_polarity(
        &self,
        reference: &str,
    ) -> Result<Option<SchematicModelPolarity>, SchematicError> {
        let component = self
            .components
            .iter()
            .find(|component| component.reference == reference)
            .ok_or_else(|| invalid("schematic component reference is unknown"))?;
        Ok(component.kind.is_transistor().then(|| {
            self.model_polarities
                .get(reference)
                .copied()
                .unwrap_or_default()
        }))
    }

    /// Set the typed polarity of one BJT, JFET, or MOSFET symbol.
    pub fn set_component_model_polarity(
        &mut self,
        reference: &str,
        polarity: SchematicModelPolarity,
    ) -> Result<(), SchematicError> {
        let component = self
            .components
            .iter()
            .find(|component| component.reference == reference)
            .ok_or_else(|| invalid("schematic component reference is unknown"))?;
        if !component.kind.is_transistor() {
            return Err(invalid(format!(
                "{} does not support a semiconductor model polarity",
                component.reference
            )));
        }
        self.model_polarities.insert(reference.to_owned(), polarity);
        Ok(())
    }

    /// Return the two typed primary model fields available for one component.
    pub fn component_model_parameters(
        &self,
        reference: &str,
    ) -> Result<Vec<(SchematicModelParameter, String)>, SchematicError> {
        let component = self
            .components
            .iter()
            .find(|component| component.reference == reference)
            .ok_or_else(|| invalid("schematic component reference is unknown"))?;
        let overrides = self.model_parameters.get(reference);
        Ok(SchematicModelParameter::for_kind(component.kind)
            .iter()
            .copied()
            .map(|parameter| {
                let value = overrides
                    .and_then(|parameters| parameters.get(&parameter))
                    .cloned()
                    .or_else(|| legacy_model_parameter(&component.value, parameter))
                    .unwrap_or_default();
                (parameter, value)
            })
            .collect())
    }

    /// Set or clear one typed primary nonlinear model field.
    pub fn set_component_model_parameter(
        &mut self,
        reference: &str,
        parameter: SchematicModelParameter,
        value: &str,
    ) -> Result<(), SchematicError> {
        let component = self
            .components
            .iter()
            .find(|component| component.reference == reference)
            .ok_or_else(|| invalid("schematic component reference is unknown"))?;
        if !SchematicModelParameter::for_kind(component.kind).contains(&parameter) {
            return Err(invalid(format!(
                "{} does not support {}",
                component.reference,
                parameter.key()
            )));
        }
        if !value.is_empty() && value.chars().any(char::is_whitespace) {
            return Err(invalid(format!(
                "{} {} must be one SPICE token",
                component.reference,
                parameter.key()
            )));
        }
        if value.is_empty() {
            if let Some(parameters) = self.model_parameters.get_mut(reference) {
                parameters.remove(&parameter);
                if parameters.is_empty() {
                    self.model_parameters.remove(reference);
                }
            }
        } else {
            self.model_parameters
                .entry(reference.to_owned())
                .or_default()
                .insert(parameter, value.to_owned());
        }
        Ok(())
    }

    /// Return selectable independent sources for source-controlled analysis cards.
    pub fn independent_source_references(&self) -> Vec<&str> {
        self.components
            .iter()
            .filter(|component| {
                matches!(
                    component.kind,
                    SchematicComponentKind::DcVoltage
                        | SchematicComponentKind::DcCurrent
                        | SchematicComponentKind::AcVoltage
                )
            })
            .map(|component| component.reference.as_str())
            .collect()
    }

    /// Return schematic elements with executable branch-current result columns.
    pub fn branch_current_element_references(&self) -> Vec<&str> {
        self.components
            .iter()
            .filter(|component| {
                matches!(
                    component.kind,
                    SchematicComponentKind::Resistor
                        | SchematicComponentKind::Capacitor
                        | SchematicComponentKind::Inductor
                        | SchematicComponentKind::Diode
                        | SchematicComponentKind::Bjt
                        | SchematicComponentKind::Jfet
                        | SchematicComponentKind::Mosfet
                        | SchematicComponentKind::DcVoltage
                        | SchematicComponentKind::AcVoltage
                )
            })
            .map(|component| component.reference.as_str())
            .collect()
    }

    /// Return named non-ground nets that can be probed by a transfer-function card.
    pub fn transfer_function_output_nodes(&self) -> Vec<&str> {
        let ground_points = self
            .components
            .iter()
            .filter(|component| component.kind == SchematicComponentKind::Ground)
            .flat_map(|component| component.terminals.iter())
            .collect::<BTreeSet<_>>();
        self.net_labels
            .iter()
            .filter(|label| !ground_points.contains(&label.point))
            .map(|label| label.name.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Return the saved-output probes in their persisted deck order.
    pub fn saved_output_probe_tokens(&self) -> Vec<String> {
        self.output_probes
            .iter()
            .map(SchematicOutputProbe::token)
            .collect()
    }

    /// Append one labelled non-ground voltage probe, unless it is already saved.
    pub fn add_saved_output_voltage_probe(&mut self, node: &str) -> Result<bool, SchematicError> {
        if !self.transfer_function_output_nodes().contains(&node) {
            return Err(invalid(format!(
                "{node} is not a labelled non-ground schematic net"
            )));
        }
        let probe = SchematicOutputProbe::Voltage {
            node: node.to_owned(),
        };
        if self.output_probes.contains(&probe) {
            return Ok(false);
        }
        self.output_probes.push(probe);
        Ok(true)
    }

    /// Append one labelled differential-voltage probe, unless it is already saved.
    pub fn add_saved_output_differential_voltage_probe(
        &mut self,
        positive_node: &str,
        negative_node: &str,
    ) -> Result<bool, SchematicError> {
        self.validate_differential_output_nodes(positive_node, negative_node)?;
        let probe = SchematicOutputProbe::DifferentialVoltage {
            positive_node: positive_node.to_owned(),
            negative_node: negative_node.to_owned(),
        };
        if self.output_probes.contains(&probe) {
            return Ok(false);
        }
        self.output_probes.push(probe);
        Ok(true)
    }

    /// Append one element branch-current probe, unless it is already saved.
    pub fn add_saved_output_current_probe(&mut self, source: &str) -> Result<bool, SchematicError> {
        if !self.branch_current_element_references().contains(&source) {
            return Err(invalid(format!(
                "{source} is not a schematic element with a branch-current result"
            )));
        }
        let probe = SchematicOutputProbe::Current {
            source: source.to_owned(),
        };
        if self.output_probes.contains(&probe) {
            return Ok(false);
        }
        self.output_probes.push(probe);
        Ok(true)
    }

    /// Remove one saved-output probe by its displayed source-order position.
    pub fn remove_saved_output_probe(
        &mut self,
        index: usize,
    ) -> Result<SchematicOutputProbe, SchematicError> {
        if index >= self.output_probes.len() {
            return Err(invalid("schematic saved output probe is unavailable"));
        }
        Ok(self.output_probes.remove(index))
    }

    /// Return scoped probes for one source-order analysis card.
    pub fn scoped_output_probe_tokens(
        &self,
        card_index: usize,
    ) -> Result<Vec<String>, SchematicError> {
        let card = self
            .analysis_card(card_index)
            .ok_or_else(|| invalid("schematic analysis card is unavailable"))?;
        Ok(self
            .scoped_output_probes
            .iter()
            .find(|scope| scope.analysis_card_id == card.id)
            .map(|scope| {
                scope
                    .probes
                    .iter()
                    .map(SchematicOutputProbe::token)
                    .collect()
            })
            .unwrap_or_default())
    }

    fn scoped_probes_mut(
        &mut self,
        card_index: usize,
    ) -> Result<&mut Vec<SchematicOutputProbe>, SchematicError> {
        let card_id = self.analysis_card_mut(card_index)?.id;
        let scope_index = match self
            .scoped_output_probes
            .iter()
            .position(|scope| scope.analysis_card_id == card_id)
        {
            Some(index) => index,
            None => {
                self.scoped_output_probes.push(SchematicScopedOutputProbes {
                    analysis_card_id: card_id,
                    probes: Vec::new(),
                });
                self.scoped_output_probes.len() - 1
            }
        };
        Ok(&mut self.scoped_output_probes[scope_index].probes)
    }

    /// Append a labelled non-ground voltage probe for one analysis card.
    pub fn add_scoped_output_voltage_probe(
        &mut self,
        card_index: usize,
        node: &str,
    ) -> Result<bool, SchematicError> {
        if !self.transfer_function_output_nodes().contains(&node) {
            return Err(invalid(format!(
                "{node} is not a labelled non-ground schematic net"
            )));
        }
        let probe = SchematicOutputProbe::Voltage {
            node: node.to_owned(),
        };
        let probes = self.scoped_probes_mut(card_index)?;
        if probes.contains(&probe) {
            return Ok(false);
        }
        probes.push(probe);
        Ok(true)
    }

    /// Append one labelled differential-voltage probe for one analysis card.
    pub fn add_scoped_output_differential_voltage_probe(
        &mut self,
        card_index: usize,
        positive_node: &str,
        negative_node: &str,
    ) -> Result<bool, SchematicError> {
        self.validate_differential_output_nodes(positive_node, negative_node)?;
        let probe = SchematicOutputProbe::DifferentialVoltage {
            positive_node: positive_node.to_owned(),
            negative_node: negative_node.to_owned(),
        };
        let probes = self.scoped_probes_mut(card_index)?;
        if probes.contains(&probe) {
            return Ok(false);
        }
        probes.push(probe);
        Ok(true)
    }

    /// Append an element branch-current probe for one analysis card.
    pub fn add_scoped_output_current_probe(
        &mut self,
        card_index: usize,
        source: &str,
    ) -> Result<bool, SchematicError> {
        if !self.branch_current_element_references().contains(&source) {
            return Err(invalid(format!(
                "{source} is not a schematic element with a branch-current result"
            )));
        }
        let probe = SchematicOutputProbe::Current {
            source: source.to_owned(),
        };
        let probes = self.scoped_probes_mut(card_index)?;
        if probes.contains(&probe) {
            return Ok(false);
        }
        probes.push(probe);
        Ok(true)
    }

    /// Remove one scoped probe by its displayed source-order position.
    pub fn remove_scoped_output_probe(
        &mut self,
        card_index: usize,
        probe_index: usize,
    ) -> Result<SchematicOutputProbe, SchematicError> {
        let card_id = self.analysis_card_mut(card_index)?.id;
        let scope_index = self
            .scoped_output_probes
            .iter()
            .position(|scope| scope.analysis_card_id == card_id)
            .ok_or_else(|| invalid("schematic scoped output probe is unavailable"))?;
        if probe_index >= self.scoped_output_probes[scope_index].probes.len() {
            return Err(invalid("schematic scoped output probe is unavailable"));
        }
        let removed = self.scoped_output_probes[scope_index]
            .probes
            .remove(probe_index);
        if self.scoped_output_probes[scope_index].probes.is_empty() {
            self.scoped_output_probes.remove(scope_index);
        }
        Ok(removed)
    }

    /// Return selectable independent sources for a canonical DC sweep.
    pub fn dc_sweep_source_references(&self) -> Vec<&str> {
        self.independent_source_references()
    }

    /// Select the independent source controlled by the first analysis card.
    pub fn set_dc_sweep_source(&mut self, reference: &str) -> Result<(), SchematicError> {
        self.set_analysis_card_dc_sweep_source(0, reference)
    }

    /// Select the independent source controlled by one DC-sweep card.
    pub fn set_analysis_card_dc_sweep_source(
        &mut self,
        index: usize,
        reference: &str,
    ) -> Result<(), SchematicError> {
        if self.analysis_card(index).map(|card| card.analysis) != Some(SchematicAnalysis::DcSweep) {
            return Err(invalid(
                "schematic analysis source applies only to DC sweep",
            ));
        }
        self.set_analysis_card_source(index, reference)
    }

    /// Select the independent source controlled by a DC-sweep or transfer-function card.
    pub fn set_analysis_card_source(
        &mut self,
        index: usize,
        reference: &str,
    ) -> Result<(), SchematicError> {
        let analysis = self
            .analysis_card(index)
            .ok_or_else(|| invalid("schematic analysis card is unavailable"))?
            .analysis;
        if !matches!(
            analysis,
            SchematicAnalysis::DcSweep | SchematicAnalysis::TransferFunction
        ) {
            return Err(invalid(
                "schematic analysis source applies only to DC sweep or transfer function",
            ));
        }
        if !self.independent_source_references().contains(&reference) {
            return Err(invalid(format!(
                "{reference} is not an independent voltage or current source"
            )));
        }
        let settings = &mut self.analysis_card_mut(index)?.settings;
        match analysis {
            SchematicAnalysis::DcSweep => settings.dc_source = reference.to_owned(),
            SchematicAnalysis::TransferFunction => settings.tf_source = reference.to_owned(),
            _ => unreachable!("source availability was checked before assigning a source"),
        }
        Ok(())
    }

    /// Read the three visible editor fields for the active analysis card.
    pub fn analysis_parameter_values(&self) -> [&str; 3] {
        self.analysis_card_parameter_values(0)
            .unwrap_or(["", "", ""])
    }

    /// Read the three visible editor fields for one ordered analysis card.
    pub fn analysis_card_parameter_values(
        &self,
        index: usize,
    ) -> Result<[&str; 3], SchematicError> {
        let (analysis, settings) = if let Some(card) = self.analysis_cards.get(index) {
            (card.analysis, &card.settings)
        } else if self.analysis_cards.is_empty() && index == 0 {
            (self.analysis, &self.analysis_settings)
        } else {
            return Err(invalid("schematic analysis card is unavailable"));
        };
        Ok(analysis_parameter_values(analysis, settings))
    }

    /// Read the labels paired with the active analysis card's editor fields.
    pub fn analysis_parameter_labels(&self) -> [&'static str; 3] {
        self.analysis_card_parameter_labels(0)
            .unwrap_or(["", "", ""])
    }

    /// Read the labels paired with one ordered card's editor fields.
    pub fn analysis_card_parameter_labels(
        &self,
        index: usize,
    ) -> Result<[&'static str; 3], SchematicError> {
        self.analysis_card(index)
            .map(|card| card.analysis.parameter_labels())
            .ok_or_else(|| invalid("schematic analysis card is unavailable"))
    }

    /// Update one visible analysis-card parameter through the canonical document.
    pub fn set_analysis_parameter(
        &mut self,
        index: usize,
        value: &str,
    ) -> Result<(), SchematicError> {
        self.set_analysis_card_parameter(0, index, value)
    }

    /// Update one visible parameter on an ordered analysis card.
    pub fn set_analysis_card_parameter(
        &mut self,
        card_index: usize,
        index: usize,
        value: &str,
    ) -> Result<(), SchematicError> {
        let analysis = self
            .analysis_card(card_index)
            .ok_or_else(|| invalid("schematic analysis card is unavailable"))?
            .analysis;
        if analysis == SchematicAnalysis::OperatingPoint {
            return Err(invalid("operating point does not accept sweep parameters"));
        }
        if index >= 3
            || (analysis == SchematicAnalysis::Transient && index == 2)
            || (analysis == SchematicAnalysis::TransferFunction && index != 0)
        {
            return Err(invalid("schematic analysis parameter is unavailable"));
        }
        if value.is_empty() || value.chars().any(char::is_whitespace) {
            return Err(invalid(
                "schematic analysis parameter must be one non-empty SPICE token",
            ));
        }
        let settings = &mut self.analysis_card_mut(card_index)?.settings;
        match (analysis, index) {
            (SchematicAnalysis::DcSweep, 0) => settings.dc_start = value.to_owned(),
            (SchematicAnalysis::DcSweep, 1) => settings.dc_stop = value.to_owned(),
            (SchematicAnalysis::DcSweep, 2) => settings.dc_step = value.to_owned(),
            (SchematicAnalysis::AcSweep, 0) => settings.ac_points_per_decade = value.to_owned(),
            (SchematicAnalysis::AcSweep, 1) => settings.ac_start_frequency = value.to_owned(),
            (SchematicAnalysis::AcSweep, 2) => settings.ac_stop_frequency = value.to_owned(),
            (SchematicAnalysis::Transient, 0) => settings.transient_time_step = value.to_owned(),
            (SchematicAnalysis::Transient, 1) => settings.transient_stop_time = value.to_owned(),
            (SchematicAnalysis::TransferFunction, 0) => settings.tf_output_node = value.to_owned(),
            _ => unreachable!("availability was checked before assigning a parameter"),
        }
        Ok(())
    }

    fn analysis_directive(&self, card: &SchematicAnalysisCard) -> Result<String, SchematicError> {
        let settings = &card.settings;
        match card.analysis {
            SchematicAnalysis::OperatingPoint => Ok(".op".to_owned()),
            SchematicAnalysis::DcSweep => {
                if !self
                    .dc_sweep_source_references()
                    .contains(&settings.dc_source.as_str())
                {
                    return Err(invalid(format!(
                        "{} is not an independent voltage or current source",
                        settings.dc_source
                    )));
                }
                Ok(format!(
                    ".dc {} {} {} {}",
                    settings.dc_source, settings.dc_start, settings.dc_stop, settings.dc_step
                ))
            }
            SchematicAnalysis::AcSweep => Ok(format!(
                ".ac dec {} {} {}",
                settings.ac_points_per_decade,
                settings.ac_start_frequency,
                settings.ac_stop_frequency
            )),
            SchematicAnalysis::Transient => Ok(format!(
                ".tran {} {}",
                settings.transient_time_step, settings.transient_stop_time
            )),
            SchematicAnalysis::TransferFunction => {
                if !self
                    .independent_source_references()
                    .contains(&settings.tf_source.as_str())
                {
                    return Err(invalid(format!(
                        "{} is not an independent voltage or current source",
                        settings.tf_source
                    )));
                }
                if !self
                    .transfer_function_output_nodes()
                    .contains(&settings.tf_output_node.as_str())
                {
                    return Err(invalid(format!(
                        "{} is not a labelled non-ground schematic net",
                        settings.tf_output_node
                    )));
                }
                Ok(format!(
                    ".tf V({}) {}",
                    settings.tf_output_node, settings.tf_source
                ))
            }
        }
    }

    fn saved_output_directive(&self) -> Result<Option<String>, SchematicError> {
        if self.output_probes.is_empty() {
            return Ok(None);
        }
        for probe in &self.output_probes {
            match probe {
                SchematicOutputProbe::Voltage { node }
                    if !self
                        .transfer_function_output_nodes()
                        .contains(&node.as_str()) =>
                {
                    return Err(invalid(format!(
                        "{node} is not a labelled non-ground schematic net"
                    )));
                }
                SchematicOutputProbe::DifferentialVoltage {
                    positive_node,
                    negative_node,
                } => self.validate_differential_output_nodes(positive_node, negative_node)?,
                SchematicOutputProbe::Current { source }
                    if !self
                        .branch_current_element_references()
                        .contains(&source.as_str()) =>
                {
                    return Err(invalid(format!(
                        "{source} is not a schematic element with a branch-current result"
                    )));
                }
                _ => {}
            }
        }
        Ok(Some(format!(
            ".save {}",
            self.saved_output_probe_tokens().join(" ")
        )))
    }

    fn scoped_output_directive(
        &self,
        card: &SchematicAnalysisCard,
    ) -> Result<Option<String>, SchematicError> {
        let Some(scope) = self
            .scoped_output_probes
            .iter()
            .find(|scope| scope.analysis_card_id == card.id)
        else {
            return Ok(None);
        };
        if scope.probes.is_empty() {
            return Ok(None);
        }
        for probe in &scope.probes {
            match probe {
                SchematicOutputProbe::Voltage { node }
                    if !self
                        .transfer_function_output_nodes()
                        .contains(&node.as_str()) =>
                {
                    return Err(invalid(format!(
                        "{node} is not a labelled non-ground schematic net"
                    )));
                }
                SchematicOutputProbe::DifferentialVoltage {
                    positive_node,
                    negative_node,
                } => self.validate_differential_output_nodes(positive_node, negative_node)?,
                SchematicOutputProbe::Current { source }
                    if !self
                        .branch_current_element_references()
                        .contains(&source.as_str()) =>
                {
                    return Err(invalid(format!(
                        "{source} is not a schematic element with a branch-current result"
                    )));
                }
                _ => {}
            }
        }
        Ok(Some(format!(
            ".probe {} {}",
            card.analysis.probe_selector(),
            scope
                .probes
                .iter()
                .map(SchematicOutputProbe::token)
                .collect::<Vec<_>>()
                .join(" ")
        )))
    }

    fn validate_differential_output_nodes(
        &self,
        positive_node: &str,
        negative_node: &str,
    ) -> Result<(), SchematicError> {
        if positive_node == negative_node {
            return Err(invalid(
                "differential voltage probes require distinct labelled non-ground schematic nets",
            ));
        }
        for node in [positive_node, negative_node] {
            if !self.transfer_function_output_nodes().contains(&node) {
                return Err(invalid(format!(
                    "{node} is not a labelled non-ground schematic net"
                )));
            }
        }
        Ok(())
    }

    fn validate_wire_endpoints(&self, wire: &SchematicWire) -> Result<(), SchematicError> {
        if wire.start == wire.end {
            return Err(invalid("schematic wires must have distinct endpoints"));
        }
        let terminal_points = self.terminal_points();
        if !terminal_points.contains(&wire.start) || !terminal_points.contains(&wire.end) {
            return Err(invalid(
                "schematic wire endpoints must be component terminals",
            ));
        }
        Ok(())
    }

    fn terminal_points(&self) -> BTreeSet<SchematicPoint> {
        self.components
            .iter()
            .flat_map(|component| component.terminals.iter().copied())
            .collect()
    }

    fn validate_net_label_name(name: &str) -> Result<(), SchematicError> {
        let mut characters = name.chars();
        if name == "0"
            || !matches!(characters.next(), Some(character) if character.is_ascii_alphabetic() || character == '_')
            || !characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            return Err(invalid(
                "schematic net labels must begin with an ASCII letter or _ and use ASCII letters, digits, or _",
            ));
        }
        Ok(())
    }

    fn net_topology(&self) -> (BTreeMap<SchematicPoint, usize>, DisjointSet, Option<usize>) {
        let mut point_ids = BTreeMap::new();
        for point in self
            .components
            .iter()
            .flat_map(|component| component.terminals.iter().copied())
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
        let ground_root = ground_points.first().copied().map(|ground_point| {
            for point in ground_points.into_iter().skip(1) {
                sets.union(ground_point, point);
            }
            sets.find(ground_point)
        });
        (point_ids, sets, ground_root)
    }

    fn net_label_names(
        &self,
        point_ids: &BTreeMap<SchematicPoint, usize>,
        sets: &mut DisjointSet,
        ground_root: Option<usize>,
    ) -> Result<BTreeMap<usize, String>, SchematicError> {
        // Multiple physical roots may share one explicit Berkeley node name.
        let mut names = BTreeMap::new();
        for label in &self.net_labels {
            if !self.terminal_points().contains(&label.point) {
                return Err(invalid(
                    "schematic net labels must target component terminals",
                ));
            }
            Self::validate_net_label_name(&label.name)?;
            let root = sets.find(point_ids[&label.point]);
            if Some(root) == ground_root {
                return Err(invalid("schematic net labels must not target ground"));
            }
            if let Some(existing) = names.get(&root) {
                if existing != &label.name {
                    return Err(invalid("schematic net has conflicting labels"));
                }
            } else {
                names.insert(root, label.name.clone());
            }
        }
        Ok(names)
    }

    fn validate_net_labels(&self) -> Result<(), SchematicError> {
        let (point_ids, mut sets, ground_root) = self.net_topology();
        self.net_label_names(&point_ids, &mut sets, ground_root)?;
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
        for reference in self.model_polarities.keys() {
            let component = self
                .components
                .iter()
                .find(|component| component.reference == *reference)
                .ok_or_else(|| {
                    invalid(format!(
                        "model polarity references unknown component {reference}"
                    ))
                })?;
            if !component.kind.is_transistor() {
                return Err(invalid(format!(
                    "{} does not support a semiconductor model polarity",
                    component.reference
                )));
            }
        }
        for (reference, parameters) in &self.model_parameters {
            let component = self
                .components
                .iter()
                .find(|component| component.reference == *reference)
                .ok_or_else(|| {
                    invalid(format!(
                        "model parameters reference unknown component {reference}"
                    ))
                })?;
            if !component.kind.is_nonlinear() {
                return Err(invalid(format!(
                    "{} does not support semiconductor model parameters",
                    component.reference
                )));
            }
            for (parameter, value) in parameters {
                if !SchematicModelParameter::for_kind(component.kind).contains(parameter) {
                    return Err(invalid(format!(
                        "{} does not support {}",
                        component.reference,
                        parameter.key()
                    )));
                }
                if value.is_empty() || value.chars().any(char::is_whitespace) {
                    return Err(invalid(format!(
                        "{} {} must be one SPICE token",
                        component.reference,
                        parameter.key()
                    )));
                }
            }
        }
        if ground_count == 0 {
            return Err(invalid("schematic requires at least one ground symbol"));
        }
        let cards = self.analysis_cards();
        let card_ids = cards.iter().map(|card| card.id).collect::<BTreeSet<_>>();
        for scope in &self.scoped_output_probes {
            if scope.analysis_card_id == 0 || !card_ids.contains(&scope.analysis_card_id) {
                return Err(invalid(
                    "scoped output probes require an available analysis card",
                ));
            }
        }
        for card in &cards {
            for (label, value) in card
                .analysis
                .parameter_labels()
                .into_iter()
                .zip(analysis_parameter_values(card.analysis, &card.settings))
            {
                if !label.is_empty() && (value.is_empty() || value.chars().any(char::is_whitespace))
                {
                    return Err(invalid(format!(
                        "{label} must be one non-empty SPICE token"
                    )));
                }
            }
            self.analysis_directive(card)?;
            self.scoped_output_directive(card)?;
        }
        self.saved_output_directive()?;
        for (index, wire) in self.wires.iter().enumerate() {
            self.validate_wire_endpoints(wire)?;
            if self.wires[..index].iter().any(|existing| {
                (existing.start == wire.start && existing.end == wire.end)
                    || (existing.start == wire.end && existing.end == wire.start)
            }) {
                return Err(invalid("schematic wire is already connected"));
            }
        }
        self.validate_net_labels()?;
        Ok(())
    }

    /// Emit a deterministic Berkeley deck independent of placement order.
    pub fn to_berkeley_netlist(&self) -> Result<String, SchematicError> {
        let mut document = self.clone();
        document.migrate_legacy_analysis_cards();
        document.to_berkeley_netlist_materialized()
    }

    fn model_parameter_text(&self, component: &SchematicComponent) -> String {
        let mut tokens = vec![component.value.clone()];
        let Some(overrides) = self.model_parameters.get(&component.reference) else {
            return tokens.join(" ");
        };
        for parameter in SchematicModelParameter::for_kind(component.kind) {
            let Some(value) = overrides.get(parameter) else {
                continue;
            };
            let token = format!("{}={value}", parameter.key());
            if let Some(existing) = tokens
                .iter_mut()
                .find(|existing| legacy_model_parameter(existing, *parameter).is_some())
            {
                *existing = token;
            } else {
                tokens.push(token);
            }
        }
        tokens.join(" ")
    }

    fn to_berkeley_netlist_materialized(&self) -> Result<String, SchematicError> {
        self.validate()?;

        let (point_ids, mut sets, ground_root) = self.net_topology();
        let ground_root = ground_root.expect("a validated schematic has a ground symbol");
        let label_names = self.net_label_names(&point_ids, &mut sets, Some(ground_root))?;

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
        let reserved_names = label_names.values().collect::<BTreeSet<_>>();
        let mut next_net = 1;
        for (root, _) in ordered_roots {
            if root == ground_root {
                names.insert(root, "0".to_owned());
            } else if let Some(name) = label_names.get(&root) {
                names.insert(root, name.clone());
            } else {
                while reserved_names.contains(&format!("n{next_net}")) {
                    next_net += 1;
                }
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
                SchematicComponentKind::Diode => {
                    let model = format!("Schematic{}Model", component.reference);
                    lines.push(format!(
                        ".model {model} D({})",
                        self.model_parameter_text(component)
                    ));
                    format!(
                        "{} {} {} {model}",
                        component.reference, terminals[0], terminals[1]
                    )
                }
                SchematicComponentKind::Bjt => {
                    let model = format!("Schematic{}Model", component.reference);
                    let model_kind = self
                        .component_model_polarity(&component.reference)?
                        .expect("BJT supports a model polarity")
                        .model_keyword(component.kind)
                        .expect("BJT model polarity has a keyword");
                    lines.push(format!(
                        ".model {model} {model_kind}({})",
                        self.model_parameter_text(component)
                    ));
                    format!(
                        "{} {} {} {} {model}",
                        component.reference, terminals[0], terminals[1], terminals[2]
                    )
                }
                SchematicComponentKind::Jfet => {
                    let model = format!("Schematic{}Model", component.reference);
                    let model_kind = self
                        .component_model_polarity(&component.reference)?
                        .expect("JFET supports a model polarity")
                        .model_keyword(component.kind)
                        .expect("JFET model polarity has a keyword");
                    lines.push(format!(
                        ".model {model} {model_kind}({})",
                        self.model_parameter_text(component)
                    ));
                    format!(
                        "{} {} {} {} {model}",
                        component.reference, terminals[0], terminals[1], terminals[2]
                    )
                }
                SchematicComponentKind::Mosfet => {
                    let model = format!("Schematic{}Model", component.reference);
                    let model_kind = self
                        .component_model_polarity(&component.reference)?
                        .expect("MOSFET supports a model polarity")
                        .model_keyword(component.kind)
                        .expect("MOSFET model polarity has a keyword");
                    lines.push(format!(
                        ".model {model} {model_kind}(LEVEL=1 {})",
                        self.model_parameter_text(component)
                    ));
                    format!(
                        "{} {} {} {} {} {model}",
                        component.reference, terminals[0], terminals[1], terminals[2], terminals[3]
                    )
                }
                SchematicComponentKind::Ground => unreachable!("ground symbols are skipped"),
            };
            lines.push(line);
        }
        for card in self.analysis_cards() {
            lines.push(self.analysis_directive(&card)?);
            if let Some(probe) = self.scoped_output_directive(&card)? {
                lines.push(probe);
            }
        }
        if let Some(card) = self.saved_output_directive()? {
            lines.push(card);
        }
        lines.push(".end".to_owned());
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
            net_labels: Vec::new(),
            output_probes: Vec::new(),
            scoped_output_probes: Vec::new(),
            model_polarities: BTreeMap::new(),
            model_parameters: BTreeMap::new(),
            analysis: SchematicAnalysis::OperatingPoint,
            analysis_settings: SchematicAnalysisSettings::default(),
            analysis_cards: Vec::new(),
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
    fn lowers_terminal_net_labels_and_reserves_them_from_anonymous_names() {
        let mut document = rc_document();
        document.set_net_label(point(0, 20), "INPUT").unwrap();
        document.set_net_label(point(40, 20), "n1").unwrap();
        assert_eq!(
            document.to_berkeley_netlist().unwrap(),
            "* RC divider\nC1 n1 0 1u\nR1 INPUT n1 1k\nV1 INPUT 0 DC 5\n.op\n.end\n"
        );
        document.set_net_label(point(40, 20), "OUTPUT").unwrap();
        let deck = document.to_berkeley_netlist().unwrap();
        assert!(deck.contains("C1 OUTPUT 0 1u\nR1 INPUT OUTPUT 1k"));
        parse_netlist(&deck).unwrap();
    }

    #[test]
    fn matching_net_labels_link_disconnected_nets_during_lowering() {
        let mut document = rc_document();
        document.remove_wire(0).unwrap();
        document.set_net_label(point(0, 20), "SENSE").unwrap();
        document.set_net_label(point(10, 20), "SENSE").unwrap();

        let deck = document.to_berkeley_netlist().unwrap();
        assert_eq!(
            deck,
            "* RC divider\nC1 n1 0 1u\nR1 SENSE n1 1k\nV1 SENSE 0 DC 5\n.op\n.end\n"
        );
        parse_netlist(&deck).unwrap();
        assert_eq!(run_netlist(&deck).unwrap().len(), 1);

        document
            .connect_wire(SchematicWire {
                start: point(0, 20),
                end: point(10, 20),
            })
            .unwrap();
        assert_eq!(document.net_label_at(point(0, 20)), Some("SENSE"));
        assert_eq!(document.net_label_at(point(10, 20)), Some("SENSE"));
    }

    #[test]
    fn routes_explicit_component_terminals_without_using_nearest_geometry() {
        let mut document = rc_document();
        let wire = document.route_terminals("V1", 0, "C1", 0).unwrap();
        assert_eq!(wire.start, point(0, 20));
        assert_eq!(wire.end, point(50, 20));
        assert!(document.wires.contains(&wire));
        assert_eq!(
            document
                .route_terminals("V1", 2, "C1", 0)
                .unwrap_err()
                .to_string(),
            "schematic route start terminal is unavailable"
        );
        assert_eq!(
            document
                .route_terminals("V1", 0, "C1", 2)
                .unwrap_err()
                .to_string(),
            "schematic route target terminal is unavailable"
        );
    }

    #[test]
    fn net_labels_reject_invalid_or_conflicting_connections_and_clean_up_with_components() {
        let mut document = rc_document();
        assert_eq!(
            document.set_net_label(point(0, 20), "0").unwrap_err().to_string(),
            "schematic net labels must begin with an ASCII letter or _ and use ASCII letters, digits, or _"
        );
        assert_eq!(
            document
                .set_net_label(point(99, 99), "INPUT")
                .unwrap_err()
                .to_string(),
            "schematic net labels must target component terminals"
        );
        assert_eq!(
            document
                .set_net_label(point(0, 0), "GROUND")
                .unwrap_err()
                .to_string(),
            "schematic net labels must not target ground"
        );

        document.set_net_label(point(0, 20), "INPUT").unwrap();
        assert_eq!(document.net_label_at(point(10, 20)), Some("INPUT"));
        document.set_net_label(point(10, 20), "ALIAS").unwrap();
        assert_eq!(document.net_label_at(point(0, 20)), Some("ALIAS"));
        document.set_net_label(point(0, 20), "INPUT").unwrap();
        document.net_labels.push(SchematicNetLabel {
            point: point(10, 20),
            name: "ALIAS".to_owned(),
        });
        assert_eq!(
            document.validate().unwrap_err().to_string(),
            "schematic net has conflicting labels"
        );
        document.net_labels.pop();
        document.set_net_label(point(40, 20), "OUTPUT").unwrap();
        assert_eq!(
            document
                .connect_wire(SchematicWire {
                    start: point(10, 20),
                    end: point(40, 20),
                })
                .unwrap_err()
                .to_string(),
            "schematic net has conflicting labels"
        );
        assert_eq!(document.wires.len(), 4);

        document.remove_component("R1").unwrap();
        assert_eq!(document.net_label_at(point(10, 20)), None);
        assert_eq!(document.net_label_at(point(40, 20)), None);
    }

    #[test]
    fn palette_placement_and_component_routing_are_deterministic() {
        let mut document = SchematicDocument {
            title: "Palette routing".to_owned(),
            components: Vec::new(),
            wires: Vec::new(),
            net_labels: Vec::new(),
            output_probes: Vec::new(),
            scoped_output_probes: Vec::new(),
            model_polarities: BTreeMap::new(),
            model_parameters: BTreeMap::new(),
            analysis: SchematicAnalysis::default(),
            analysis_settings: SchematicAnalysisSettings::default(),
            analysis_cards: Vec::new(),
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
            net_labels: Vec::new(),
            output_probes: Vec::new(),
            scoped_output_probes: Vec::new(),
            model_polarities: BTreeMap::new(),
            model_parameters: BTreeMap::new(),
            analysis: SchematicAnalysis::AcSweep,
            analysis_settings: SchematicAnalysisSettings::default(),
            analysis_cards: Vec::new(),
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
    fn configurable_analysis_cards_lower_selected_sources_and_parameters() {
        let mut document = rc_document();
        document.analysis = SchematicAnalysis::DcSweep;
        document.set_dc_sweep_source("V1").unwrap();
        document.set_analysis_parameter(0, "-1").unwrap();
        document.set_analysis_parameter(1, "4").unwrap();
        document.set_analysis_parameter(2, "0.5").unwrap();
        let deck = document.to_berkeley_netlist().unwrap();
        assert!(deck.contains(".dc V1 -1 4 0.5"));
        parse_netlist(&deck).unwrap();
        assert_eq!(run_netlist(&deck).unwrap().len(), 1);

        document
            .set_analysis_card_kind(0, SchematicAnalysis::AcSweep)
            .unwrap();
        document.set_analysis_parameter(0, "20").unwrap();
        document.set_analysis_parameter(1, "1").unwrap();
        document.set_analysis_parameter(2, "1k").unwrap();
        assert!(document
            .to_berkeley_netlist()
            .unwrap()
            .contains(".ac dec 20 1 1k"));

        document
            .set_analysis_card_kind(0, SchematicAnalysis::Transient)
            .unwrap();
        document.set_analysis_parameter(0, "2m").unwrap();
        document.set_analysis_parameter(1, "20m").unwrap();
        assert!(document
            .to_berkeley_netlist()
            .unwrap()
            .contains(".tran 2m 20m"));

        assert_eq!(
            document
                .set_analysis_parameter(2, "1")
                .unwrap_err()
                .to_string(),
            "schematic analysis parameter is unavailable"
        );
        assert_eq!(
            document
                .set_analysis_parameter(0, "1 m")
                .unwrap_err()
                .to_string(),
            "schematic analysis parameter must be one non-empty SPICE token"
        );
        document
            .set_analysis_card_kind(0, SchematicAnalysis::DcSweep)
            .unwrap();
        assert_eq!(
            document.set_dc_sweep_source("R1").unwrap_err().to_string(),
            "R1 is not an independent voltage or current source"
        );
    }

    #[test]
    fn transfer_function_cards_require_a_labelled_output_and_independent_source() {
        let mut document = rc_document();
        document
            .set_analysis_card_kind(0, SchematicAnalysis::TransferFunction)
            .unwrap();
        assert_eq!(
            document.to_berkeley_netlist().unwrap_err().to_string(),
            "Output node must be one non-empty SPICE token"
        );

        document.set_net_label(point(40, 20), "OUT").unwrap();
        assert_eq!(document.transfer_function_output_nodes(), ["OUT"]);
        document.set_analysis_card_parameter(0, 0, "OUT").unwrap();
        document.set_analysis_card_source(0, "V1").unwrap();
        let deck = document.to_berkeley_netlist().unwrap();
        assert!(deck.contains(".tf V(OUT) V1"));
        parse_netlist(&deck).unwrap();
        assert_eq!(run_netlist(&deck).unwrap().len(), 1);

        assert_eq!(
            document
                .set_analysis_card_source(0, "R1")
                .unwrap_err()
                .to_string(),
            "R1 is not an independent voltage or current source"
        );
        assert_eq!(
            document
                .set_analysis_card_parameter(0, 1, "ignored")
                .unwrap_err()
                .to_string(),
            "schematic analysis parameter is unavailable"
        );
        document
            .set_analysis_card_parameter(0, 0, "MISSING")
            .unwrap();
        assert_eq!(
            document.to_berkeley_netlist().unwrap_err().to_string(),
            "MISSING is not a labelled non-ground schematic net"
        );
    }

    #[test]
    fn persisted_analysis_settings_without_transfer_fields_keep_safe_defaults() {
        let settings: SchematicAnalysisSettings = serde_json::from_str(
            r#"{
                "dc_source":"V1",
                "dc_start":"0",
                "dc_stop":"5",
                "dc_step":"1",
                "ac_points_per_decade":"10",
                "ac_start_frequency":"10",
                "ac_stop_frequency":"10k",
                "transient_time_step":"1m",
                "transient_stop_time":"10m"
            }"#,
        )
        .unwrap();
        assert_eq!(settings.tf_output_node, "");
        assert_eq!(settings.tf_source, "V1");
    }

    #[test]
    fn ordered_analysis_cards_lower_and_validate_in_source_order() {
        let mut document = rc_document();
        document
            .set_analysis_card_kind(0, SchematicAnalysis::DcSweep)
            .unwrap();
        document.set_analysis_card_dc_sweep_source(0, "V1").unwrap();
        document.set_analysis_card_parameter(0, 0, "-1").unwrap();
        document.set_analysis_card_parameter(0, 1, "2").unwrap();
        document.set_analysis_card_parameter(0, 2, "0.5").unwrap();
        let ac = document.add_analysis_card(SchematicAnalysis::AcSweep);
        document.set_analysis_card_parameter(ac, 0, "20").unwrap();
        document.set_analysis_card_parameter(ac, 1, "1").unwrap();
        document.set_analysis_card_parameter(ac, 2, "1k").unwrap();
        let transient = document.add_analysis_card(SchematicAnalysis::Transient);
        document
            .set_analysis_card_parameter(transient, 0, "2m")
            .unwrap();
        document
            .set_analysis_card_parameter(transient, 1, "20m")
            .unwrap();

        assert_eq!(
            document.analysis_card_labels(),
            ["1. DC sweep", "2. AC sweep", "3. Transient"]
        );
        let deck = document.to_berkeley_netlist().unwrap();
        assert!(deck.contains(".dc V1 -1 2 0.5\n.ac dec 20 1 1k\n.tran 2m 20m\n.end"));
        parse_netlist(&deck).unwrap();
        assert_eq!(run_netlist(&deck).unwrap().len(), 3);
        document.move_analysis_card(1, 0).unwrap();
        assert_eq!(
            document.analysis_card_labels(),
            ["1. AC sweep", "2. DC sweep", "3. Transient"]
        );
        assert!(document
            .to_berkeley_netlist()
            .unwrap()
            .contains(".ac dec 20 1 1k\n.dc V1 -1 2 0.5\n.tran 2m 20m\n.end"));
        assert_eq!(
            document.move_analysis_card(0, 3).unwrap_err().to_string(),
            "schematic analysis card is unavailable"
        );
        document.remove_analysis_card(1).unwrap();
        assert_eq!(
            document.analysis_card_labels(),
            ["1. AC sweep", "2. Transient"]
        );
        document.remove_analysis_card(1).unwrap();
        assert_eq!(
            document.remove_analysis_card(0).unwrap_err().to_string(),
            "schematic requires at least one analysis card"
        );
    }

    #[test]
    fn edit_operations_remove_incident_wires_without_relaxing_endpoint_rules() {
        let mut document = rc_document();
        document
            .connect_wire(SchematicWire {
                start: point(10, 20),
                end: point(10, 0),
            })
            .unwrap();
        assert!(document.remove_component("R1").unwrap() >= 1);
        assert!(document
            .components
            .iter()
            .all(|component| component.reference != "R1"));
        assert!(document.wires.iter().all(|wire| {
            wire.start != point(10, 20)
                && wire.end != point(10, 20)
                && wire.start != point(40, 20)
                && wire.end != point(40, 20)
        }));
        assert_eq!(
            document.remove_component("R1").unwrap_err().to_string(),
            "schematic component reference is unknown"
        );

        document
            .connect_wire(SchematicWire {
                start: point(0, 20),
                end: point(0, 0),
            })
            .unwrap();
        let last_wire = document.wires.len() - 1;
        assert_eq!(
            document.remove_wire(last_wire).unwrap(),
            SchematicWire {
                start: point(0, 20),
                end: point(0, 0),
            }
        );
        assert_eq!(
            document
                .remove_wire(document.wires.len())
                .unwrap_err()
                .to_string(),
            "schematic wire is unavailable"
        );
    }

    #[test]
    fn metadata_edits_keep_titles_valid_and_dc_source_references_current() {
        let mut document = rc_document();
        document.set_title("Renamed divider").unwrap();
        assert_eq!(document.title, "Renamed divider");
        assert_eq!(
            document.set_title("\n").unwrap_err().to_string(),
            "schematic title must be one non-empty line"
        );
        assert_eq!(document.rename_component("V1", "VDD").unwrap(), 1);
        assert_eq!(document.analysis_settings.dc_source, "VDD");
        document
            .set_analysis_card_kind(0, SchematicAnalysis::DcSweep)
            .unwrap();
        assert_eq!(document.rename_component("VDD", "VBIAS").unwrap(), 1);
        assert_eq!(
            document.analysis_card(0).unwrap().settings.dc_source,
            "VBIAS"
        );
        assert!(document
            .to_berkeley_netlist()
            .unwrap()
            .contains(".dc VBIAS 0 5 1"));
        assert_eq!(
            document
                .rename_component("VBIAS", "I1")
                .unwrap_err()
                .to_string(),
            "I1 reference must begin with V and use ASCII letters, digits, or _"
        );
        assert_eq!(
            document
                .rename_component("VBIAS", "R1")
                .unwrap_err()
                .to_string(),
            "R1 reference must begin with V and use ASCII letters, digits, or _"
        );
    }

    #[test]
    fn component_renames_keep_transfer_function_source_bindings_current() {
        let mut document = rc_document();
        document.set_net_label(point(40, 20), "OUT").unwrap();
        document
            .set_analysis_card_kind(0, SchematicAnalysis::TransferFunction)
            .unwrap();
        document.set_analysis_card_parameter(0, 0, "OUT").unwrap();
        document.set_analysis_card_source(0, "V1").unwrap();

        assert_eq!(document.rename_component("V1", "VBIAS").unwrap(), 1);
        assert_eq!(
            document.analysis_card(0).unwrap().settings.tf_source,
            "VBIAS"
        );
        assert!(document
            .to_berkeley_netlist()
            .unwrap()
            .contains(".tf V(OUT) VBIAS"));
    }

    #[test]
    fn saved_output_probes_lower_voltage_differential_and_linear_currents() {
        let mut document = rc_document();
        document.set_net_label(point(0, 20), "IN").unwrap();
        document.set_net_label(point(40, 20), "OUT").unwrap();
        assert!(document.add_saved_output_voltage_probe("OUT").unwrap());
        assert!(document
            .add_saved_output_differential_voltage_probe("OUT", "IN")
            .unwrap());
        assert!(document.add_saved_output_current_probe("V1").unwrap());
        assert!(document.add_saved_output_current_probe("R1").unwrap());
        assert!(document.add_saved_output_current_probe("C1").unwrap());
        assert!(!document.add_saved_output_voltage_probe("OUT").unwrap());
        assert!(!document
            .add_saved_output_differential_voltage_probe("OUT", "IN")
            .unwrap());
        assert_eq!(
            document.saved_output_probe_tokens(),
            ["V(OUT)", "V(OUT,IN)", "I(V1)", "I(R1)", "I(C1)"]
        );
        assert!(document
            .to_berkeley_netlist()
            .unwrap()
            .contains(".save V(OUT) V(OUT,IN) I(V1) I(R1) I(C1)"));
        document.rename_component("V1", "VBIAS").unwrap();
        assert_eq!(
            document.saved_output_probe_tokens(),
            ["V(OUT)", "V(OUT,IN)", "I(VBIAS)", "I(R1)", "I(C1)"]
        );
        let deck = document.to_berkeley_netlist().unwrap();
        parse_netlist(&deck).unwrap();
        assert_eq!(run_netlist(&deck).unwrap().len(), 1);
        assert!(deck.contains(".save V(OUT) V(OUT,IN) I(VBIAS) I(R1) I(C1)"));
        assert_eq!(
            document
                .add_saved_output_voltage_probe("MISSING")
                .unwrap_err()
                .to_string(),
            "MISSING is not a labelled non-ground schematic net"
        );
        assert_eq!(
            document
                .add_saved_output_differential_voltage_probe("OUT", "OUT")
                .unwrap_err()
                .to_string(),
            "differential voltage probes require distinct labelled non-ground schematic nets"
        );
        assert_eq!(
            document
                .add_saved_output_differential_voltage_probe("OUT", "MISSING")
                .unwrap_err()
                .to_string(),
            "MISSING is not a labelled non-ground schematic net"
        );
        assert_eq!(
            document
                .add_saved_output_current_probe("I1")
                .unwrap_err()
                .to_string(),
            "I1 is not a schematic element with a branch-current result"
        );
        assert_eq!(
            document
                .remove_saved_output_probe(5)
                .unwrap_err()
                .to_string(),
            "schematic saved output probe is unavailable"
        );
    }

    #[test]
    fn linear_passive_branch_currents_are_selectable_from_the_document_palette() {
        let mut document = rc_document();
        document.components.push(SchematicComponent {
            reference: "L1".to_owned(),
            kind: SchematicComponentKind::Inductor,
            value: "1m".to_owned(),
            terminals: vec![point(60, 20), point(60, 0)],
        });

        assert_eq!(
            document.branch_current_element_references(),
            ["C1", "V1", "R1", "L1"]
        );
        assert!(document.add_saved_output_current_probe("L1").unwrap());
        assert_eq!(document.saved_output_probe_tokens(), ["I(L1)"]);
    }

    #[test]
    fn nonlinear_palette_symbols_use_stable_defaults_and_terminal_arities() {
        let mut document = rc_document();
        for (label, reference, value, terminal_count) in [
            ("Diode", "D1", "IS=1e-14", 2),
            ("BJT", "Q1", "BF=100", 3),
            ("JFET", "J1", "BETA=1m", 3),
            ("MOSFET", "M1", "VTO=0.7", 4),
        ] {
            let kind = SchematicComponentKind::from_palette_label(label).unwrap();
            assert_eq!(document.place_palette_component(kind).unwrap(), reference);
            let component = document.components.last().unwrap();
            assert_eq!(component.value, value);
            assert_eq!(component.terminals.len(), terminal_count);
        }

        assert_eq!(
            document.branch_current_element_references(),
            ["C1", "V1", "R1", "D1", "Q1", "J1", "M1"]
        );
    }

    #[test]
    fn nonlinear_branch_currents_lower_through_global_and_scoped_output_selection() {
        let mut document = rc_document();
        document.components.extend([
            SchematicComponent {
                reference: "D1".to_owned(),
                kind: SchematicComponentKind::Diode,
                value: "IS=1e-14".to_owned(),
                terminals: vec![point(0, 20), point(0, 0)],
            },
            SchematicComponent {
                reference: "Q1".to_owned(),
                kind: SchematicComponentKind::Bjt,
                value: "BF=100".to_owned(),
                terminals: vec![point(0, 20), point(0, 20), point(0, 0)],
            },
            SchematicComponent {
                reference: "J1".to_owned(),
                kind: SchematicComponentKind::Jfet,
                value: "BETA=1m".to_owned(),
                terminals: vec![point(0, 20), point(0, 0), point(0, 0)],
            },
            SchematicComponent {
                reference: "M1".to_owned(),
                kind: SchematicComponentKind::Mosfet,
                value: "VTO=0.7".to_owned(),
                terminals: vec![point(0, 20), point(0, 20), point(0, 0), point(0, 0)],
            },
        ]);

        assert_eq!(
            document.branch_current_element_references(),
            ["C1", "V1", "R1", "D1", "Q1", "J1", "M1"]
        );
        for reference in ["D1", "Q1", "J1", "M1"] {
            assert!(document.add_saved_output_current_probe(reference).unwrap());
        }
        let ac = document.add_analysis_card(SchematicAnalysis::AcSweep);
        for reference in ["D1", "Q1", "J1", "M1"] {
            assert!(document
                .add_scoped_output_current_probe(ac, reference)
                .unwrap());
        }

        let deck = document.to_berkeley_netlist().unwrap();
        for model in [
            ".model SchematicD1Model D(IS=1e-14)",
            ".model SchematicQ1Model NPN(BF=100)",
            ".model SchematicJ1Model NJF(BETA=1m)",
            ".model SchematicM1Model NMOS(LEVEL=1 VTO=0.7)",
        ] {
            assert!(deck.contains(model), "missing {model} in {deck}");
        }
        assert!(deck.contains(".save I(D1) I(Q1) I(J1) I(M1)"));
        assert!(deck.contains(".probe ac I(D1) I(Q1) I(J1) I(M1)"));
        parse_netlist(&deck).unwrap();
    }

    #[test]
    fn nonlinear_model_fields_are_typed_persisted_and_lowered_deterministically() {
        let mut document = rc_document();
        document.components.extend([
            SchematicComponent {
                reference: "D1".to_owned(),
                kind: SchematicComponentKind::Diode,
                value: "IS=1e-14".to_owned(),
                terminals: vec![point(0, 20), point(0, 0)],
            },
            SchematicComponent {
                reference: "Q1".to_owned(),
                kind: SchematicComponentKind::Bjt,
                value: "BF=100".to_owned(),
                terminals: vec![point(0, 20), point(0, 20), point(0, 0)],
            },
            SchematicComponent {
                reference: "J1".to_owned(),
                kind: SchematicComponentKind::Jfet,
                value: "BETA=1m".to_owned(),
                terminals: vec![point(0, 20), point(0, 0), point(0, 0)],
            },
            SchematicComponent {
                reference: "M1".to_owned(),
                kind: SchematicComponentKind::Mosfet,
                value: "VTO=0.7".to_owned(),
                terminals: vec![point(0, 20), point(0, 20), point(0, 0), point(0, 0)],
            },
        ]);
        for (reference, parameter, value) in [
            ("D1", SchematicModelParameter::EmissionCoefficient, "1.2"),
            (
                "D1",
                SchematicModelParameter::ZeroBiasJunctionCapacitance,
                "1.5e-12",
            ),
            ("D1", SchematicModelParameter::TransitTime, "4e-9"),
            ("Q1", SchematicModelParameter::SaturationCurrent, "2e-14"),
            (
                "Q1",
                SchematicModelParameter::BjtBaseEmitterCapacitance,
                "2e-12",
            ),
            (
                "Q1",
                SchematicModelParameter::BjtForwardTransitTime,
                "1e-10",
            ),
            ("J1", SchematicModelParameter::ThresholdVoltage, "-2"),
            (
                "J1",
                SchematicModelParameter::ChannelLengthModulation,
                "0.02",
            ),
            (
                "J1",
                SchematicModelParameter::JfetGateSourceCapacitance,
                "2n",
            ),
            (
                "J1",
                SchematicModelParameter::JfetGateDrainCapacitance,
                "100p",
            ),
            ("M1", SchematicModelParameter::MosfetTransconductance, "2m"),
            (
                "M1",
                SchematicModelParameter::ChannelLengthModulation,
                "0.04",
            ),
            ("M1", SchematicModelParameter::MosfetSubstrateDoping, "1.6"),
            (
                "M1",
                SchematicModelParameter::MosfetDrainBulkCapacitance,
                "3e-13",
            ),
            (
                "M1",
                SchematicModelParameter::MosfetGateSourceOverlapCapacitance,
                "20p",
            ),
            (
                "M1",
                SchematicModelParameter::MosfetGateDrainOverlapCapacitance,
                "5p",
            ),
            (
                "M1",
                SchematicModelParameter::MosfetGateBulkOverlapCapacitance,
                "1p",
            ),
            (
                "M1",
                SchematicModelParameter::MosfetSourceBulkCapacitance,
                "4e-13",
            ),
            (
                "M1",
                SchematicModelParameter::MosfetBulkJunctionPotential,
                "0.9",
            ),
            (
                "M1",
                SchematicModelParameter::MosfetBulkJunctionGradingCoefficient,
                "0.45",
            ),
        ] {
            document
                .set_component_model_parameter(reference, parameter, value)
                .unwrap();
        }
        assert_eq!(
            document.component_model_parameters("Q1").unwrap(),
            vec![
                (
                    SchematicModelParameter::SaturationCurrent,
                    "2e-14".to_owned()
                ),
                (SchematicModelParameter::ForwardBeta, "100".to_owned()),
                (
                    SchematicModelParameter::BjtBaseEmitterCapacitance,
                    "2e-12".to_owned()
                ),
                (
                    SchematicModelParameter::BjtForwardTransitTime,
                    "1e-10".to_owned()
                ),
            ]
        );
        let deck = document.to_berkeley_netlist().unwrap();
        for model in [
            ".model SchematicD1Model D(IS=1e-14 N=1.2 CJO=1.5e-12 TT=4e-9)",
            ".model SchematicQ1Model NPN(BF=100 IS=2e-14 CJE=2e-12 TF=1e-10)",
            ".model SchematicJ1Model NJF(BETA=1m VTO=-2 LAMBDA=0.02 CGS=2n CGD=100p)",
            ".model SchematicM1Model NMOS(LEVEL=1 VTO=0.7 KP=2m LAMBDA=0.04 NSUB=1.6 CBD=3e-13 CGSO=20p CGDO=5p CGBO=1p CBS=4e-13 PB=0.9 MJ=0.45)",
        ] {
            assert!(deck.contains(model), "missing {model} in {deck}");
        }
        parse_netlist(&deck).unwrap();

        document.rename_component("Q1", "QMODEL").unwrap();
        assert!(document.model_parameters.contains_key("QMODEL"));
        document.remove_component("J1").unwrap();
        assert!(!document.model_parameters.contains_key("J1"));

        let persisted = serde_json::to_value(&document).unwrap();
        let restored: SchematicDocument = serde_json::from_value(persisted).unwrap();
        assert_eq!(restored, document);
        assert_eq!(
            document
                .set_component_model_parameter("R1", SchematicModelParameter::ForwardBeta, "50")
                .unwrap_err()
                .to_string(),
            "R1 does not support BF"
        );
    }

    #[test]
    fn transistor_polarities_are_typed_persisted_and_lowered_by_model_family() {
        let mut document = rc_document();
        document.components.extend([
            SchematicComponent {
                reference: "Q1".to_owned(),
                kind: SchematicComponentKind::Bjt,
                value: "BF=100".to_owned(),
                terminals: vec![point(0, 20), point(0, 20), point(0, 0)],
            },
            SchematicComponent {
                reference: "J1".to_owned(),
                kind: SchematicComponentKind::Jfet,
                value: "BETA=1m".to_owned(),
                terminals: vec![point(0, 20), point(0, 0), point(0, 0)],
            },
            SchematicComponent {
                reference: "M1".to_owned(),
                kind: SchematicComponentKind::Mosfet,
                value: "VTO=0.7".to_owned(),
                terminals: vec![point(0, 20), point(0, 20), point(0, 0), point(0, 0)],
            },
        ]);
        for reference in ["Q1", "J1", "M1"] {
            document
                .set_component_model_polarity(reference, SchematicModelPolarity::P)
                .unwrap();
        }
        assert_eq!(
            document.component_model_polarity("Q1").unwrap(),
            Some(SchematicModelPolarity::P)
        );
        assert_eq!(
            SchematicComponentKind::Mosfet.model_polarity_labels(),
            Some(("NMOS", "PMOS"))
        );
        let deck = document.to_berkeley_netlist().unwrap();
        for model in [
            ".model SchematicQ1Model PNP(BF=100)",
            ".model SchematicJ1Model PJF(BETA=1m)",
            ".model SchematicM1Model PMOS(LEVEL=1 VTO=0.7)",
        ] {
            assert!(deck.contains(model), "missing {model} in {deck}");
        }
        document.rename_component("Q1", "QP1").unwrap();
        assert_eq!(
            document.component_model_polarity("QP1").unwrap(),
            Some(SchematicModelPolarity::P)
        );
        document.remove_component("J1").unwrap();
        assert!(!document.model_polarities.contains_key("J1"));
        document
            .model_polarities
            .insert("R1".to_owned(), SchematicModelPolarity::P);
        assert_eq!(
            document.validate().unwrap_err().to_string(),
            "R1 does not support a semiconductor model polarity"
        );
    }

    #[test]
    fn scoped_output_probes_follow_card_identity_and_drop_with_the_card() {
        let mut document = rc_document();
        document.set_net_label(point(0, 20), "IN").unwrap();
        document.set_net_label(point(40, 20), "OUT").unwrap();
        let ac = document.add_analysis_card(SchematicAnalysis::AcSweep);
        assert!(document.add_scoped_output_voltage_probe(ac, "OUT").unwrap());
        assert!(document
            .add_scoped_output_differential_voltage_probe(ac, "OUT", "IN")
            .unwrap());
        assert!(document.add_scoped_output_current_probe(ac, "V1").unwrap());
        assert!(document.add_scoped_output_current_probe(ac, "R1").unwrap());
        assert!(document.add_scoped_output_current_probe(ac, "C1").unwrap());
        assert_eq!(
            document.scoped_output_probe_tokens(ac).unwrap(),
            ["V(OUT)", "V(OUT,IN)", "I(V1)", "I(R1)", "I(C1)"]
        );

        let deck = document.to_berkeley_netlist().unwrap();
        assert!(deck.contains(
            ".op\n.ac dec 10 10 10k\n.probe ac V(OUT) V(OUT,IN) I(V1) I(R1) I(C1)\n.end"
        ));
        parse_netlist(&deck).unwrap();

        document.move_analysis_card(ac, 0).unwrap();
        assert_eq!(
            document.scoped_output_probe_tokens(0).unwrap(),
            ["V(OUT)", "V(OUT,IN)", "I(V1)", "I(R1)", "I(C1)"]
        );
        assert!(document.to_berkeley_netlist().unwrap().contains(
            ".ac dec 10 10 10k\n.probe ac V(OUT) V(OUT,IN) I(V1) I(R1) I(C1)\n.op\n.end"
        ));
        document.rename_component("V1", "VBIAS").unwrap();
        assert_eq!(
            document.scoped_output_probe_tokens(0).unwrap(),
            ["V(OUT)", "V(OUT,IN)", "I(VBIAS)", "I(R1)", "I(C1)"]
        );
        document.remove_analysis_card(0).unwrap();
        assert!(document.scoped_output_probes.is_empty());
        assert!(!document.to_berkeley_netlist().unwrap().contains(".probe"));
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

    #[test]
    fn edits_component_values_without_permitting_ground_or_whitespace() {
        let mut document = rc_document();
        document.set_component_value("R1", "2k").unwrap();
        assert!(document
            .to_berkeley_netlist()
            .unwrap()
            .contains("R1 n1 n2 2k"));
        assert_eq!(
            document
                .set_component_value("R1", "2 k")
                .unwrap_err()
                .to_string(),
            "R1 value must be one non-empty SPICE token"
        );
        assert_eq!(
            document
                .set_component_value("G1", "0")
                .unwrap_err()
                .to_string(),
            "G1 ground symbol does not accept a SPICE value"
        );
        assert_eq!(
            document
                .set_component_value("X1", "1")
                .unwrap_err()
                .to_string(),
            "schematic component reference is unknown"
        );
    }
}
