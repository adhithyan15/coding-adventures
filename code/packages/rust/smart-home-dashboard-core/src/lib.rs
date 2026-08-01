//! Native dashboard manifest contracts shared by migration and runtime clients.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

pub const DASHBOARD_MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeDashboardCardKind {
    EntityControl,
    EntityList,
    History,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeDashboardCard {
    pub card_id: String,
    pub kind: NativeDashboardCardKind,
    pub source_type: String,
    #[serde(default)]
    pub title: Option<String>,
    pub entity_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeDashboardView {
    pub view_id: String,
    pub title: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    pub cards: Vec<NativeDashboardCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeDashboard {
    pub dashboard_id: String,
    pub url_path: String,
    pub title: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub require_admin: bool,
    pub show_in_sidebar: bool,
    pub views: Vec<NativeDashboardView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeAssistantDashboardResource {
    pub resource_id: String,
    pub url: String,
    pub resource_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeDashboardManifest {
    pub schema_version: u32,
    pub source_instance_id: String,
    pub generated_at_ms: u64,
    pub dashboards: Vec<NativeDashboard>,
    pub source_resources: Vec<HomeAssistantDashboardResource>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct NativeDashboardManifestSummary {
    pub dashboards: usize,
    pub views: usize,
    pub cards: usize,
    pub entity_references: usize,
    pub source_resources: usize,
}

impl NativeDashboardManifest {
    pub fn validate(&self) -> Result<(), DashboardManifestError> {
        if self.schema_version != DASHBOARD_MANIFEST_SCHEMA_VERSION {
            return Err(DashboardManifestError::Validation(format!(
                "unsupported dashboard manifest schema version {}",
                self.schema_version
            )));
        }
        require_non_empty("source_instance_id", &self.source_instance_id)?;

        let mut dashboard_ids = BTreeSet::new();
        let mut view_ids = BTreeSet::new();
        let mut card_ids = BTreeSet::new();
        for dashboard in &self.dashboards {
            require_non_empty("dashboard_id", &dashboard.dashboard_id)?;
            require_non_empty("dashboard title", &dashboard.title)?;
            require_non_empty("dashboard url_path", &dashboard.url_path)?;
            if !dashboard_ids.insert(dashboard.dashboard_id.as_str()) {
                return Err(DashboardManifestError::Validation(format!(
                    "duplicate dashboard_id `{}`",
                    dashboard.dashboard_id
                )));
            }
            for view in &dashboard.views {
                require_non_empty("view_id", &view.view_id)?;
                require_non_empty("view title", &view.title)?;
                if !view_ids.insert((dashboard.dashboard_id.as_str(), view.view_id.as_str())) {
                    return Err(DashboardManifestError::Validation(format!(
                        "duplicate view_id `{}` in dashboard `{}`",
                        view.view_id, dashboard.dashboard_id
                    )));
                }
                for card in &view.cards {
                    require_non_empty("card_id", &card.card_id)?;
                    require_non_empty("card source_type", &card.source_type)?;
                    if !card_ids.insert((
                        dashboard.dashboard_id.as_str(),
                        view.view_id.as_str(),
                        card.card_id.as_str(),
                    )) {
                        return Err(DashboardManifestError::Validation(format!(
                            "duplicate card_id `{}` in view `{}`",
                            card.card_id, view.view_id
                        )));
                    }
                    if card
                        .entity_ids
                        .iter()
                        .any(|entity_id| entity_id.trim().is_empty())
                    {
                        return Err(DashboardManifestError::Validation(format!(
                            "card `{}` contains an empty entity id",
                            card.card_id
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn summary(&self) -> NativeDashboardManifestSummary {
        let mut summary = NativeDashboardManifestSummary {
            dashboards: self.dashboards.len(),
            source_resources: self.source_resources.len(),
            ..NativeDashboardManifestSummary::default()
        };
        for dashboard in &self.dashboards {
            summary.views += dashboard.views.len();
            for view in &dashboard.views {
                summary.cards += view.cards.len();
                summary.entity_references += view
                    .cards
                    .iter()
                    .map(|card| card.entity_ids.len())
                    .sum::<usize>();
            }
        }
        summary
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DashboardManifestDocument {
    Manifest(NativeDashboardManifest),
    MigrationArtifact {
        dry_run: bool,
        plan: DashboardManifestPlan,
    },
}

#[derive(Debug, Deserialize)]
struct DashboardManifestPlan {
    manifest: NativeDashboardManifest,
}

pub fn parse_dashboard_manifest(
    bytes: &[u8],
) -> Result<NativeDashboardManifest, DashboardManifestError> {
    let document: DashboardManifestDocument = serde_json::from_slice(bytes)
        .map_err(|error| DashboardManifestError::Decode(error.to_string()))?;
    let manifest = match document {
        DashboardManifestDocument::Manifest(manifest) => manifest,
        DashboardManifestDocument::MigrationArtifact { dry_run, plan } => {
            if dry_run {
                return Err(DashboardManifestError::Validation(
                    "dry-run dashboard migration artifacts cannot be served".to_string(),
                ));
            }
            plan.manifest
        }
    };
    manifest.validate()?;
    Ok(manifest)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashboardManifestError {
    Decode(String),
    Validation(String),
}

impl fmt::Display for DashboardManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(message) => {
                write!(formatter, "dashboard manifest decode failed: {message}")
            }
            Self::Validation(message) => write!(formatter, "invalid dashboard manifest: {message}"),
        }
    }
}

impl std::error::Error for DashboardManifestError {}

fn require_non_empty(field: &str, value: &str) -> Result<(), DashboardManifestError> {
    if value.trim().is_empty() {
        Err(DashboardManifestError::Validation(format!(
            "{field} must not be empty"
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> NativeDashboardManifest {
        NativeDashboardManifest {
            schema_version: DASHBOARD_MANIFEST_SCHEMA_VERSION,
            source_instance_id: "home".to_string(),
            generated_at_ms: 42,
            dashboards: vec![NativeDashboard {
                dashboard_id: "overview".to_string(),
                url_path: "lovelace".to_string(),
                title: "Overview".to_string(),
                icon: None,
                require_admin: false,
                show_in_sidebar: true,
                views: vec![NativeDashboardView {
                    view_id: "home".to_string(),
                    title: "Home".to_string(),
                    path: None,
                    icon: None,
                    cards: vec![NativeDashboardCard {
                        card_id: "lights".to_string(),
                        kind: NativeDashboardCardKind::EntityList,
                        source_type: "entities".to_string(),
                        title: Some("Lights".to_string()),
                        entity_ids: vec!["ha:light.kitchen".to_string()],
                    }],
                }],
            }],
            source_resources: Vec::new(),
        }
    }

    #[test]
    fn parses_raw_and_applied_migration_documents() {
        let expected = manifest();
        let raw = serde_json::to_vec(&expected).unwrap();
        assert_eq!(parse_dashboard_manifest(&raw).unwrap(), expected);

        let artifact = serde_json::json!({
            "schema_version": 1,
            "dry_run": false,
            "plan": {"manifest": expected},
            "receipt": {"migration_id": "migration-1"}
        });
        assert_eq!(
            parse_dashboard_manifest(&serde_json::to_vec(&artifact).unwrap()).unwrap(),
            manifest()
        );
    }

    #[test]
    fn rejects_dry_runs_and_duplicate_ids() {
        let dry_run = serde_json::json!({
            "dry_run": true,
            "plan": {"manifest": manifest()}
        });
        assert!(parse_dashboard_manifest(&serde_json::to_vec(&dry_run).unwrap()).is_err());

        let mut duplicate = manifest();
        duplicate.dashboards.push(duplicate.dashboards[0].clone());
        assert!(duplicate.validate().is_err());
    }

    #[test]
    fn summarizes_manifest_content() {
        assert_eq!(
            manifest().summary(),
            NativeDashboardManifestSummary {
                dashboards: 1,
                views: 1,
                cards: 1,
                entity_references: 1,
                source_resources: 0,
            }
        );
    }
}
