//! # Assurance Report Model & Serialization (VER01 / VER00 §9.1)
//!
//! Represents and serializes the complete machine-readable `assurance_report.json`.

#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

use crate::json::JsonValue;
use crate::matrix::WeaknessRow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilitiesSummary {
    pub declared: Vec<String>,
    pub analyzer: String,
    pub certificate: String,
    pub new_since_last_release: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyEvidence {
    pub cases: u64,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExhaustiveEvidence {
    pub cases: u64,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuzzEvidence {
    pub iterations: u64,
    pub status: String,
}

/// One assumption a claim no longer relies on, with the evidence that
/// discharged it (VER13 section 4.6).  Checked by rules AC-3 and AC-6..AC-9.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DischargedAssumption {
    /// Canonical catalogue id.
    pub id: String,
    /// Rung (`R0+`..`R4`) at which the evidence discharges it.
    pub rung: String,
    /// The stack the evidence is about; must equal the claim's stack (AC-7).
    pub stack: String,
    /// Manifest ids of the discharging evidence (AC-3).
    pub evidence: Vec<String>,
    /// For per-run evidence: the pinned build tuple (AC-9); empty otherwise.
    pub pinned: BTreeMap<String, String>,
    /// For per-run evidence: `for build <artefact_sha256>` (AC-9); empty otherwise.
    pub scope: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofEvidence {
    pub label: String,
    /// Catalogue ids the label still rests on, sorted (VER13 section 5.3).
    pub assumptions: Vec<String>,
    pub certificate: String,
    /// `"trivial-miter"` when any backing certificate is a collapsed miter (the
    /// proof is valid but vacuous at the SAT level), `"miter"` otherwise.
    /// Absent in legacy reports (parsed as `""`).
    pub certificate_kind: String,
    /// Which artefact the guarantee describes (`host-rustc`, `owned-rv32i`).
    pub stack: String,
    /// Ladder rung of the claim (`R0` = WIR self-composition today).
    pub rung: String,
    /// Assumptions removed by checked evidence (empty until AS-002).
    pub discharged: Vec<DischargedAssumption>,
    /// How many listed assumptions sit at each layer (derived, VER13 4.6).
    pub remaining_by_layer: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimEvidence {
    pub id: String,
    pub symbol: String,
    pub example: String,
    pub property: PropertyEvidence,
    /// Bounded-exhaustive suites (VER04h section 3); absent in legacy reports.
    pub exhaustive: Option<ExhaustiveEvidence>,
    pub fuzz: Option<FuzzEvidence>,
    pub proof: Option<ProofEvidence>,
    /// Manifest ids of every test and proof backing this claim.
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RungsSummary {
    pub e0: String,
    pub e1: String,
    pub e2: String,
    pub e3: String,
    pub e4: String,
    pub e5: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComputedExemption {
    pub id: String,
    pub kind: String,
    pub cwe: u32,
    pub approved: bool,
    pub rationale: String,
    pub evidence: Vec<String>,
    /// The catalogue ids the exemption is derived from (VER13 section 3.9,
    /// rule AC-13): an exemption is a record *about* assumptions, never a
    /// silent extra one.
    pub assumptions: Vec<String>,
}

/// The pin of the catalogue files a report was generated against (VER13
/// section 5.3).  `--check` fails when any digest differs from the files on
/// disk (AC-14, `CatalogueDrift`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogueRef {
    pub schema: String,
    pub path: String,
    pub sha256: String,
    pub evidence_kinds_sha256: String,
    pub non_claims_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssuranceReport {
    pub package: String,
    pub version: String,
    pub commit: String,
    /// The name of the audit profile the report was generated under
    /// (`assurance-profile.json`, `name`), recorded so that `--check` can hold the report to it.
    /// It is emitted as one string field, so the JSON stays deterministic and byte-stable.
    pub profile: String,
    /// SHA-256 (hex) of the exact bytes of the profile file the report was generated from,
    /// bound like `spec_sha256`: `--check` recomputes it, so an edited profile is refused.
    pub profile_sha256: String,
    /// The pinned assumption catalogue (VER13 section 5.3).
    pub catalogue: CatalogueRef,
    /// Report-scope assumptions: catalogue entries about the weakness matrix,
    /// the capability evidence and the verdict (`applies_to.claims` of
    /// `weakness-row`, `capability`, `report`), which no single claim owns.
    pub assumptions: Vec<String>,
    pub closure: Vec<String>,
    pub capabilities: CapabilitiesSummary,
    pub weaknesses: Vec<WeaknessRow>,
    pub claims: Vec<ClaimEvidence>,
    pub rungs: RungsSummary,
    pub public_symbols_without_claims: Vec<String>,
    pub exemptions: Vec<ComputedExemption>,
    pub verdict: String,
}

#[derive(Debug)]
pub enum ReportError {
    JsonParse(String),
    Validation(String),
}

impl fmt::Display for ReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReportError::JsonParse(e) => write!(f, "Report JSON parse error: {e}"),
            ReportError::Validation(e) => write!(f, "Report validation error: {e}"),
        }
    }
}

impl std::error::Error for ReportError {}

impl AssuranceReport {
    pub fn to_json_value(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("package".into(), JsonValue::String(self.package.clone()));
        map.insert("version".into(), JsonValue::String(self.version.clone()));
        map.insert("commit".into(), JsonValue::String(self.commit.clone()));
        map.insert("profile".into(), JsonValue::String(self.profile.clone()));
        map.insert("profile_sha256".into(), JsonValue::String(self.profile_sha256.clone()));

        let mut cat = BTreeMap::new();
        cat.insert("schema".into(), JsonValue::String(self.catalogue.schema.clone()));
        cat.insert("path".into(), JsonValue::String(self.catalogue.path.clone()));
        cat.insert("sha256".into(), JsonValue::String(self.catalogue.sha256.clone()));
        cat.insert(
            "evidence_kinds_sha256".into(),
            JsonValue::String(self.catalogue.evidence_kinds_sha256.clone()),
        );
        cat.insert(
            "non_claims_sha256".into(),
            JsonValue::String(self.catalogue.non_claims_sha256.clone()),
        );
        map.insert("catalogue".into(), JsonValue::Object(cat));
        map.insert("assumptions".into(), strings(&self.assumptions));

        let closure_arr = self
            .closure
            .iter()
            .map(|s| JsonValue::String(s.clone()))
            .collect();
        map.insert("closure".into(), JsonValue::Array(closure_arr));

        // Capabilities
        let mut cap_map = BTreeMap::new();
        let dec_arr = self
            .capabilities
            .declared
            .iter()
            .map(|s| JsonValue::String(s.clone()))
            .collect();
        cap_map.insert("declared".into(), JsonValue::Array(dec_arr));
        cap_map.insert(
            "analyzer".into(),
            JsonValue::String(self.capabilities.analyzer.clone()),
        );
        cap_map.insert(
            "certificate".into(),
            JsonValue::String(self.capabilities.certificate.clone()),
        );
        let new_arr = self
            .capabilities
            .new_since_last_release
            .iter()
            .map(|s| JsonValue::String(s.clone()))
            .collect();
        cap_map.insert("new_since_last_release".into(), JsonValue::Array(new_arr));
        map.insert("capabilities".into(), JsonValue::Object(cap_map));

        // Weaknesses
        let weak_arr = self
            .weaknesses
            .iter()
            .map(|w| w.to_json_value())
            .collect();
        map.insert("weaknesses".into(), JsonValue::Array(weak_arr));

        // Claims
        let mut claims_arr = Vec::new();
        for c in &self.claims {
            let mut c_map = BTreeMap::new();
            c_map.insert("id".into(), JsonValue::String(c.id.clone()));
            c_map.insert("symbol".into(), JsonValue::String(c.symbol.clone()));
            c_map.insert("example".into(), JsonValue::String(c.example.clone()));

            let mut prop_map = BTreeMap::new();
            prop_map.insert(
                "cases".into(),
                JsonValue::Number(c.property.cases as f64),
            );
            prop_map.insert(
                "status".into(),
                JsonValue::String(c.property.status.clone()),
            );
            c_map.insert("property".into(), JsonValue::Object(prop_map));

            if let Some(ex) = &c.exhaustive {
                let mut e_map = BTreeMap::new();
                e_map.insert("cases".into(), JsonValue::Number(ex.cases as f64));
                e_map.insert("status".into(), JsonValue::String(ex.status.clone()));
                c_map.insert("exhaustive".into(), JsonValue::Object(e_map));
            }
            c_map.insert(
                "evidence".into(),
                JsonValue::Array(c.evidence.iter().map(|e| JsonValue::String(e.clone())).collect()),
            );

            if let Some(fuzz) = &c.fuzz {
                let mut f_map = BTreeMap::new();
                f_map.insert("iterations".into(), JsonValue::Number(fuzz.iterations as f64));
                f_map.insert("status".into(), JsonValue::String(fuzz.status.clone()));
                c_map.insert("fuzz".into(), JsonValue::Object(f_map));
            } else {
                c_map.insert("fuzz".into(), JsonValue::Null);
            }

            if let Some(proof) = &c.proof {
                let mut p_map = BTreeMap::new();
                p_map.insert("label".into(), JsonValue::String(proof.label.clone()));
                let assump_arr = proof
                    .assumptions
                    .iter()
                    .map(|a| JsonValue::String(a.clone()))
                    .collect();
                p_map.insert("assumptions".into(), JsonValue::Array(assump_arr));
                p_map.insert(
                    "certificate".into(),
                    JsonValue::String(proof.certificate.clone()),
                );
                p_map.insert(
                    "certificate_kind".into(),
                    JsonValue::String(proof.certificate_kind.clone()),
                );
                p_map.insert("stack".into(), JsonValue::String(proof.stack.clone()));
                p_map.insert("rung".into(), JsonValue::String(proof.rung.clone()));
                p_map.insert(
                    "discharged".into(),
                    JsonValue::Array(proof.discharged.iter().map(discharged_to_json).collect()),
                );
                p_map.insert(
                    "remaining_by_layer".into(),
                    JsonValue::Object(
                        proof
                            .remaining_by_layer
                            .iter()
                            .map(|(k, v)| (k.clone(), JsonValue::Number(*v as f64)))
                            .collect(),
                    ),
                );
                c_map.insert("proof".into(), JsonValue::Object(p_map));
            } else {
                c_map.insert("proof".into(), JsonValue::Null);
            }

            claims_arr.push(JsonValue::Object(c_map));
        }
        map.insert("claims".into(), JsonValue::Array(claims_arr));

        // Rungs
        let mut r_map = BTreeMap::new();
        r_map.insert("E0".into(), JsonValue::String(self.rungs.e0.clone()));
        r_map.insert("E1".into(), JsonValue::String(self.rungs.e1.clone()));
        r_map.insert("E2".into(), JsonValue::String(self.rungs.e2.clone()));
        r_map.insert("E3".into(), JsonValue::String(self.rungs.e3.clone()));
        r_map.insert("E4".into(), JsonValue::String(self.rungs.e4.clone()));
        r_map.insert("E5".into(), JsonValue::String(self.rungs.e5.clone()));
        map.insert("rungs".into(), JsonValue::Object(r_map));

        // Public symbols without claims
        let sym_arr = self
            .public_symbols_without_claims
            .iter()
            .map(|s| JsonValue::String(s.clone()))
            .collect();
        map.insert(
            "public_symbols_without_claims".into(),
            JsonValue::Array(sym_arr),
        );

        // Exemptions
        let mut ex_arr = Vec::new();
        for ex in &self.exemptions {
            let mut ex_map = BTreeMap::new();
            ex_map.insert("id".into(), JsonValue::String(ex.id.clone()));
            ex_map.insert("kind".into(), JsonValue::String(ex.kind.clone()));
            ex_map.insert("cwe".into(), JsonValue::Number(ex.cwe as f64));
            ex_map.insert("approved".into(), JsonValue::Bool(ex.approved));
            ex_map.insert("rationale".into(), JsonValue::String(ex.rationale.clone()));
            let ev_arr = ex
                .evidence
                .iter()
                .map(|e| JsonValue::String(e.clone()))
                .collect();
            ex_map.insert("evidence".into(), JsonValue::Array(ev_arr));
            ex_map.insert("assumptions".into(), strings(&ex.assumptions));
            ex_arr.push(JsonValue::Object(ex_map));
        }
        map.insert("exemptions".into(), JsonValue::Array(ex_arr));

        map.insert("verdict".into(), JsonValue::String(self.verdict.clone()));

        JsonValue::Object(map)
    }

    pub fn to_json_string(&self) -> String {
        self.to_json_value().to_pretty_string(2)
    }

    pub fn from_json_str(s: &str) -> Result<Self, ReportError> {
        let val = JsonValue::parse(s).map_err(ReportError::JsonParse)?;
        Self::from_json_value(&val)
    }

    pub fn from_json_value(val: &JsonValue) -> Result<Self, ReportError> {
        let obj = val
            .as_object()
            .ok_or_else(|| ReportError::Validation("root must be a JSON object".into()))?;

        let package = obj
            .get("package")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ReportError::Validation("missing 'package'".into()))?
            .to_string();

        let version = obj
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.1.0")
            .to_string();

        let commit = obj
            .get("commit")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // The profile is required: a report that does not say which profile it was generated
        // under cannot be held to it, so it is refused rather than read as `comparison`.
        let profile: String = obj
            .get("profile")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ReportError::Validation("missing 'profile' (the audit profile the report was generated under)".into()))?
            .to_string();
        let profile_sha256: String = obj
            .get("profile_sha256")
            .and_then(|v| v.as_str())
            .filter(|h| h.len() == 64 && h.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
            .ok_or_else(|| {
                ReportError::Validation(
                    "missing or malformed 'profile_sha256' (64 lowercase hex digits: the digest of assurance-profile.json)".into(),
                )
            })?
            .to_string();

        // The catalogue pin and the report-scope assumptions are required:
        // a report without them predates VER13 and cannot be checked against
        // the catalogue, so it is refused rather than read as "no assumptions".
        let cat_obj = obj
            .get("catalogue")
            .and_then(|v| v.as_object())
            .ok_or_else(|| ReportError::Validation("missing 'catalogue' object (VER13 section 5.3)".into()))?;
        exact_keys(
            cat_obj,
            &["schema", "path", "sha256", "evidence_kinds_sha256", "non_claims_sha256"],
            "catalogue",
        )?;
        let cat_text = |k: &str| -> Result<String, ReportError> {
            cat_obj
                .get(k)
                .and_then(|v| v.as_str())
                .map(String::from)
                .ok_or_else(|| ReportError::Validation(format!("catalogue.{k} must be a string")))
        };
        let catalogue = CatalogueRef {
            schema: cat_text("schema")?,
            path: cat_text("path")?,
            sha256: cat_text("sha256")?,
            evidence_kinds_sha256: cat_text("evidence_kinds_sha256")?,
            non_claims_sha256: cat_text("non_claims_sha256")?,
        };
        let assumptions = strict_strings(obj.get("assumptions"), "assumptions")?;

        let closure = obj
            .get("closure")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let cap_obj = obj
            .get("capabilities")
            .and_then(|v| v.as_object())
            .ok_or_else(|| ReportError::Validation("missing 'capabilities'".into()))?;

        let declared = cap_obj
            .get("declared")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let analyzer = cap_obj
            .get("analyzer")
            .and_then(|v| v.as_str())
            .unwrap_or("pass")
            .to_string();

        let certificate = cap_obj
            .get("certificate")
            .and_then(|v| v.as_str())
            .unwrap_or("proved")
            .to_string();

        let new_since_last_release = cap_obj
            .get("new_since_last_release")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let capabilities = CapabilitiesSummary {
            declared,
            analyzer,
            certificate,
            new_since_last_release,
        };

        let weak_arr = obj
            .get("weaknesses")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ReportError::Validation("missing 'weaknesses'".into()))?;

        let mut weaknesses = Vec::with_capacity(weak_arr.len());
        for w_val in weak_arr {
            weaknesses.push(WeaknessRow::from_json_value(w_val).map_err(ReportError::Validation)?);
        }

        let claims_arr = obj
            .get("claims")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ReportError::Validation("missing 'claims'".into()))?;

        let mut claims = Vec::with_capacity(claims_arr.len());
        for c_val in claims_arr {
            let c_obj = c_val
                .as_object()
                .ok_or_else(|| ReportError::Validation("claim must be an object".into()))?;

            let id = c_obj
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let symbol = c_obj
                .get("symbol")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let example = c_obj
                .get("example")
                .and_then(|v| v.as_str())
                .unwrap_or("pass")
                .to_string();

            let prop_obj = c_obj
                .get("property")
                .and_then(|v| v.as_object())
                .ok_or_else(|| ReportError::Validation("claim missing 'property'".into()))?;

            let prop_cases = prop_obj
                .get("cases")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as u64;

            let prop_status = prop_obj
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("pass")
                .to_string();

            let property = PropertyEvidence {
                cases: prop_cases,
                status: prop_status,
            };

            let exhaustive = c_obj.get("exhaustive").and_then(|v| v.as_object()).map(|e| {
                ExhaustiveEvidence {
                    cases: e.get("cases").and_then(|v| v.as_i64()).unwrap_or(0) as u64,
                    status: e
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                }
            });
            let evidence: Vec<String> = c_obj
                .get("evidence")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let fuzz = if let Some(f_obj) = c_obj.get("fuzz").and_then(|v| v.as_object()) {
                let iters = f_obj
                    .get("iterations")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as u64;
                let status = f_obj
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("pass")
                    .to_string();
                Some(FuzzEvidence {
                    iterations: iters,
                    status,
                })
            } else {
                None
            };

            let proof = if let Some(p_obj) = c_obj.get("proof").and_then(|v| v.as_object()) {
                let label = p_obj
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("proved")
                    .to_string();
                // Strict: a missing or mistyped list is an error, never "no
                // assumptions" (an empty list is still rejected by AC-2).
                let assumptions = strict_strings(p_obj.get("assumptions"), "proof.assumptions")?;
                let stack = strict_text(p_obj.get("stack"), "proof.stack")?;
                let rung = strict_text(p_obj.get("rung"), "proof.rung")?;
                let discharged = match p_obj.get("discharged").and_then(|v| v.as_array()) {
                    Some(items) if items.len() <= MAX_REPORT_LIST => {
                        items.iter().map(discharged_from_json).collect::<Result<Vec<_>, _>>()?
                    }
                    _ => {
                        return Err(ReportError::Validation(
                            "proof.discharged must be a list of at most 4096 objects".into(),
                        ))
                    }
                };
                let remaining_by_layer = match p_obj.get("remaining_by_layer").and_then(|v| v.as_object()) {
                    Some(m) => {
                        let mut out = BTreeMap::new();
                        for (k, v) in m {
                            let n = v.as_i64().and_then(|n| u64::try_from(n).ok()).ok_or_else(|| {
                                ReportError::Validation(format!("proof.remaining_by_layer.{k} must be a count"))
                            })?;
                            out.insert(k.clone(), n);
                        }
                        out
                    }
                    None => {
                        return Err(ReportError::Validation(
                            "proof.remaining_by_layer must be an object".into(),
                        ))
                    }
                };
                let cert = p_obj
                    .get("certificate")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let certificate_kind = p_obj
                    .get("certificate_kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Some(ProofEvidence {
                    label,
                    assumptions,
                    certificate: cert,
                    certificate_kind,
                    stack,
                    rung,
                    discharged,
                    remaining_by_layer,
                })
            } else {
                None
            };

            claims.push(ClaimEvidence {
                id,
                symbol,
                example,
                property,
                exhaustive,
                fuzz,
                proof,
                evidence,
            });
        }

        let r_obj = obj
            .get("rungs")
            .and_then(|v| v.as_object())
            .ok_or_else(|| ReportError::Validation("missing 'rungs'".into()))?;

        let e0 = r_obj.get("E0").and_then(|v| v.as_str()).unwrap_or("pass").to_string();
        let e1 = r_obj.get("E1").and_then(|v| v.as_str()).unwrap_or("pass").to_string();
        let e2 = r_obj.get("E2").and_then(|v| v.as_str()).unwrap_or("not-live").to_string();
        let e3 = r_obj.get("E3").and_then(|v| v.as_str()).unwrap_or("pass").to_string();
        let e4 = r_obj.get("E4").and_then(|v| v.as_str()).unwrap_or("pass").to_string();
        let e5 = r_obj.get("E5").and_then(|v| v.as_str()).unwrap_or("pass").to_string();

        let rungs = RungsSummary {
            e0,
            e1,
            e2,
            e3,
            e4,
            e5,
        };

        let public_symbols_without_claims = obj
            .get("public_symbols_without_claims")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let ex_arr = obj
            .get("exemptions")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ReportError::Validation("missing 'exemptions'".into()))?;

        let mut exemptions = Vec::with_capacity(ex_arr.len());
        for ex_val in ex_arr {
            let ex_obj = ex_val
                .as_object()
                .ok_or_else(|| ReportError::Validation("exemption must be an object".into()))?;

            let id = ex_obj.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let kind = ex_obj.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let cwe = ex_obj.get("cwe").and_then(|v| v.as_u32()).unwrap_or(0);
            let approved = ex_obj.get("approved").and_then(|v| v.as_bool()).unwrap_or(false);
            let rationale = ex_obj.get("rationale").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let evidence = ex_obj
                .get("evidence")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let assumptions = strict_strings(ex_obj.get("assumptions"), "exemption.assumptions")?;

            exemptions.push(ComputedExemption {
                id,
                kind,
                cwe,
                approved,
                rationale,
                evidence,
                assumptions,
            });
        }

        let verdict = obj
            .get("verdict")
            .and_then(|v| v.as_str())
            .unwrap_or("report-only")
            .to_string();

        Ok(Self {
            package,
            version,
            commit,
            profile,
            profile_sha256,
            catalogue,
            assumptions,
            closure,
            capabilities,
            weaknesses,
            claims,
            rungs,
            public_symbols_without_claims,
            exemptions,
            verdict,
        })
    }
}

// ---------------------------------------------------------------------------
// Strict helpers for the VER13 fields.  The older fields above keep their
// lenient defaults for compatibility; everything the assumption rules read is
// parsed strictly, because a dropped or mistyped id would be a silent drop.
// ---------------------------------------------------------------------------

/// Upper bound on the length of any id list in a report.
pub const MAX_REPORT_LIST: usize = 4096;

fn strings(items: &[String]) -> JsonValue {
    JsonValue::Array(items.iter().map(|s| JsonValue::String(s.clone())).collect())
}

fn strict_text(v: Option<&JsonValue>, what: &str) -> Result<String, ReportError> {
    v.and_then(|x| x.as_str())
        .map(String::from)
        .ok_or_else(|| ReportError::Validation(format!("{what} must be a string")))
}

/// A list of strings; absence, a non-list, or a non-string item is an error.
fn strict_strings(v: Option<&JsonValue>, what: &str) -> Result<Vec<String>, ReportError> {
    let items = v
        .and_then(|x| x.as_array())
        .ok_or_else(|| ReportError::Validation(format!("{what} must be a list of strings")))?;
    if items.len() > MAX_REPORT_LIST {
        return Err(ReportError::Validation(format!("{what} has more than {MAX_REPORT_LIST} items")));
    }
    items
        .iter()
        .map(|x| {
            x.as_str()
                .map(String::from)
                .ok_or_else(|| ReportError::Validation(format!("{what} items must be strings")))
        })
        .collect()
}

fn exact_keys(
    obj: &BTreeMap<String, JsonValue>,
    keys: &[&str],
    what: &str,
) -> Result<(), ReportError> {
    for k in obj.keys() {
        if !keys.contains(&k.as_str()) {
            return Err(ReportError::Validation(format!("{what} has unknown field '{k}'")));
        }
    }
    Ok(())
}

fn discharged_to_json(d: &DischargedAssumption) -> JsonValue {
    let mut m = BTreeMap::new();
    m.insert("id".into(), JsonValue::String(d.id.clone()));
    m.insert("rung".into(), JsonValue::String(d.rung.clone()));
    m.insert("stack".into(), JsonValue::String(d.stack.clone()));
    m.insert("evidence".into(), strings(&d.evidence));
    if !d.pinned.is_empty() {
        m.insert(
            "pinned".into(),
            JsonValue::Object(d.pinned.iter().map(|(k, v)| (k.clone(), JsonValue::String(v.clone()))).collect()),
        );
    }
    if !d.scope.is_empty() {
        m.insert("scope".into(), JsonValue::String(d.scope.clone()));
    }
    JsonValue::Object(m)
}

fn discharged_from_json(v: &JsonValue) -> Result<DischargedAssumption, ReportError> {
    let o = v
        .as_object()
        .ok_or_else(|| ReportError::Validation("proof.discharged items must be objects".into()))?;
    exact_keys(o, &["id", "rung", "stack", "evidence", "pinned", "scope"], "proof.discharged item")?;
    let pinned = match o.get("pinned") {
        None => BTreeMap::new(),
        Some(p) => crate::catalogue::parse_pinned_object(p, "proof.discharged", false)
            .map_err(|e| ReportError::Validation(e.0))?,
    };
    Ok(DischargedAssumption {
        id: strict_text(o.get("id"), "proof.discharged.id")?,
        rung: strict_text(o.get("rung"), "proof.discharged.rung")?,
        stack: strict_text(o.get("stack"), "proof.discharged.stack")?,
        evidence: strict_strings(o.get("evidence"), "proof.discharged.evidence")?,
        pinned,
        scope: match o.get("scope") {
            None => String::new(),
            Some(s) => strict_text(Some(s), "proof.discharged.scope")?,
        },
    })
}
