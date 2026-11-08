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
pub struct FuzzEvidence {
    pub iterations: u64,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofEvidence {
    pub label: String,
    pub assumptions: Vec<String>,
    pub certificate: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimEvidence {
    pub id: String,
    pub symbol: String,
    pub example: String,
    pub property: PropertyEvidence,
    pub fuzz: Option<FuzzEvidence>,
    pub proof: Option<ProofEvidence>,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssuranceReport {
    pub package: String,
    pub version: String,
    pub commit: String,
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
                let assumptions = p_obj
                    .get("assumptions")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let cert = p_obj
                    .get("certificate")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Some(ProofEvidence {
                    label,
                    assumptions,
                    certificate: cert,
                })
            } else {
                None
            };

            claims.push(ClaimEvidence {
                id,
                symbol,
                example,
                property,
                fuzz,
                proof,
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

            exemptions.push(ComputedExemption {
                id,
                kind,
                cwe,
                approved,
                rationale,
                evidence,
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
