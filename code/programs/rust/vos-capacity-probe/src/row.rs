//! # `row`: one measured row, its knob and its JSON form
//!
//! A [`Row`] is one `(experiment, knob, seed)` measurement, the unit of the results file
//! (VOS30 section 9.6).  The JSON is written and read with the repository's
//! `coding_adventures_bounded_json`; nothing here parses text by hand.
//!
//! ```text
//!   Knob   the experiment's sweep point:  { "length": 100, "divergence": 0.25, "cut": "every8" }
//!   Row    knob + mode + seed + every measurement + the verdict fields the tool computes
//! ```

use coding_adventures_bounded_json::{parse, serialize, JsonNumber, JsonValue};

/// One value of a knob.
#[derive(Clone, Debug, PartialEq)]
pub enum Kv {
    /// An integer.
    I(i64),
    /// A float.
    F(f64),
    /// A string.
    S(String),
}

/// A sweep point: ordered key/value pairs.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Knob(pub Vec<(String, Kv)>);

impl Knob {
    /// Build a knob from pairs.
    pub fn of(pairs: &[(&str, Kv)]) -> Knob {
        Knob(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        )
    }
    /// An integer field (0 if absent or not an integer).
    pub fn int(&self, key: &str) -> i64 {
        match self.0.iter().find(|(k, _)| k == key) {
            Some((_, Kv::I(i))) => *i,
            Some((_, Kv::F(f))) => *f as i64,
            _ => 0,
        }
    }
    /// A float field.
    pub fn f(&self, key: &str) -> f64 {
        match self.0.iter().find(|(k, _)| k == key) {
            Some((_, Kv::I(i))) => *i as f64,
            Some((_, Kv::F(f))) => *f,
            _ => 0.0,
        }
    }
    /// A string field (empty if absent).
    pub fn s(&self, key: &str) -> &str {
        match self.0.iter().find(|(k, _)| k == key) {
            Some((_, Kv::S(s))) => s,
            _ => "",
        }
    }
    /// The JSON object.
    pub fn to_json(&self) -> JsonValue {
        JsonValue::Object(
            self.0
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        match v {
                            Kv::I(i) => JsonValue::Number(JsonNumber::Integer(*i)),
                            Kv::F(f) => JsonValue::Number(JsonNumber::Float(*f)),
                            Kv::S(s) => JsonValue::String(s.clone()),
                        },
                    )
                })
                .collect(),
        )
    }
    /// Read a knob from its JSON object.
    pub fn from_json(v: &JsonValue) -> Result<Knob, String> {
        let JsonValue::Object(pairs) = v else {
            return Err("knob is not an object".into());
        };
        let mut out = Vec::new();
        for (k, x) in pairs {
            out.push((
                k.clone(),
                match x {
                    JsonValue::Number(JsonNumber::Integer(i)) => Kv::I(*i),
                    JsonValue::Number(JsonNumber::Float(f)) => Kv::F(*f),
                    JsonValue::String(s) => Kv::S(s.clone()),
                    _ => return Err(format!("knob field {k} is not a number or a string")),
                },
            ));
        }
        Ok(Knob(out))
    }
    /// A canonical text for grouping rows by knob.
    pub fn key(&self) -> String {
        serialize(&self.to_json()).unwrap_or_default()
    }
}

/// The G3 control record of a row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Control {
    /// `sat` (a solver control), `caught` (an enumeration control), `refused`, or a failure.
    pub result: String,
    /// The model satisfies the control CNF, re-evaluated by a plain evaluator.
    pub model_checks_cnf: bool,
    /// The counterexample replayed on the concrete machines.
    pub replayed: bool,
}

impl Control {
    /// A control that did not run.
    pub fn none(why: &str) -> Control {
        Control {
            result: why.to_string(),
            model_checks_cnf: false,
            replayed: false,
        }
    }
    /// Whether the control did its job.
    pub fn ok(&self) -> bool {
        matches!(self.result.as_str(), "sat" | "caught") && self.model_checks_cnf && self.replayed
    }
}

/// One measured row.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// Experiment id (`A1`, `B3`, ...).
    pub experiment: String,
    /// The sweep point.
    pub knob: Knob,
    /// `monolithic`, `incremental`, `certified`, `split` or `enum`.
    pub mode: String,
    /// `quick` or `full`.
    pub tier: String,
    /// The seed of this row.
    pub seed: u64,
    /// Seeds run for the knob.
    pub seeds: u32,
    /// SHA-256 of the DIMACS text.
    pub instance_sha256: String,
    /// SHA-256 of the DRAT text.
    pub drat_sha256: Option<String>,
    /// SHA-256 of the control's DIMACS.
    pub control_instance_sha256: String,
    /// AND nodes in the miter cone.
    pub ands: u64,
    /// CNF variables.
    pub vars: u64,
    /// CNF clauses.
    pub clauses: u64,
    /// Sensitive declared bits.
    pub entropy_bits: u64,
    /// Declared bits.
    pub declared_bits: u64,
    /// The verdict word.
    pub result: String,
    /// The section 4.5 enum, when collapsed.
    pub collapsed_reason: Option<String>,
    /// The G3 control.
    pub control: Control,
    /// Conflicts (summed over steps).
    pub conflicts: u64,
    /// Propagations.
    pub propagations: u64,
    /// The worst obligation's conflicts, for chains and splits.
    pub worst_step_conflicts: Option<u64>,
    /// Obligations of a chain or split (1 for a monolithic row).
    pub steps: u64,
    /// Wall time of the solves.
    pub solve_ms: f64,
    /// Wall time of the worst obligation's solve.
    pub worst_step_solve_ms: f64,
    /// DRAT bytes of the worst obligation.
    pub drat_bytes: Option<u64>,
    /// Checker wall time.
    pub check_ms: Option<f64>,
    /// Checker work of the worst obligation.
    pub check_work: Option<u64>,
    /// Accepted under the default limits (every obligation).
    pub pass_default_limits: Option<bool>,
    /// Peak resident set of the child, KiB (set by the parent from `/usr/bin/time`).
    pub peak_rss_kb: u64,
    /// Computed by the tool from section 8.3.
    pub green: bool,
    /// Which limit ended the row.
    pub cap: String,
    /// A refusal reason or an anomaly.
    pub note: String,
    /// Experiment-specific numbers.
    pub metrics: Vec<(String, f64)>,
    /// The row budget in seconds when it is not the 120 s default (VOS-D095, VOS30 section 9.5):
    /// a chain row records its 600 s here; `None` means 120, so a row that does not change keeps
    /// its recorded bytes.
    pub wall_s: Option<u64>,
}

impl Row {
    /// A blank row of an experiment, to be filled in.
    pub fn new(experiment: &str, knob: Knob, mode: &str, seed: u64, seeds: u32) -> Row {
        Row {
            experiment: experiment.to_string(),
            knob,
            mode: mode.to_string(),
            tier: "full".to_string(),
            seed,
            seeds,
            instance_sha256: String::new(),
            drat_sha256: None,
            control_instance_sha256: String::new(),
            ands: 0,
            vars: 0,
            clauses: 0,
            entropy_bits: 0,
            declared_bits: 0,
            result: "crashed".to_string(),
            collapsed_reason: None,
            control: Control::none("not run"),
            conflicts: 0,
            propagations: 0,
            worst_step_conflicts: None,
            steps: 1,
            solve_ms: 0.0,
            worst_step_solve_ms: 0.0,
            drat_bytes: None,
            check_ms: None,
            check_work: None,
            pass_default_limits: None,
            peak_rss_kb: 0,
            green: false,
            cap: "none".to_string(),
            note: String::new(),
            metrics: Vec::new(),
            wall_s: None,
        }
    }

    /// A metric by name.
    pub fn metric(&self, name: &str) -> Option<f64> {
        self.metrics
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| *v)
    }

    /// The JSON object of the row.
    pub fn to_json(&self) -> JsonValue {
        let int =
            |v: u64| JsonValue::Number(JsonNumber::Integer(i64::try_from(v).unwrap_or(i64::MAX)));
        let flt =
            |v: f64| JsonValue::Number(JsonNumber::Float(if v.is_finite() { v } else { 0.0 }));
        let ostr = |v: &Option<String>| v.clone().map(JsonValue::String).unwrap_or(JsonValue::Null);
        let oint = |v: Option<u64>| v.map(int).unwrap_or(JsonValue::Null);
        let o = |pairs: Vec<(&str, JsonValue)>| {
            JsonValue::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
        };
        let mut fields = vec![
            ("experiment", JsonValue::String(self.experiment.clone())),
            ("knob", self.knob.to_json()),
            ("mode", JsonValue::String(self.mode.clone())),
            ("tier", JsonValue::String(self.tier.clone())),
            ("seed", int(self.seed)),
            ("seeds", int(self.seeds as u64)),
            (
                "instance_sha256",
                JsonValue::String(self.instance_sha256.clone()),
            ),
            ("drat_sha256", ostr(&self.drat_sha256)),
            (
                "control_instance_sha256",
                JsonValue::String(self.control_instance_sha256.clone()),
            ),
            ("ands", int(self.ands)),
            ("vars", int(self.vars)),
            ("clauses", int(self.clauses)),
            ("entropy_bits", int(self.entropy_bits)),
            ("declared_bits", int(self.declared_bits)),
            ("result", JsonValue::String(self.result.clone())),
            ("collapsed_reason", ostr(&self.collapsed_reason)),
            (
                "control",
                o(vec![
                    ("result", JsonValue::String(self.control.result.clone())),
                    (
                        "model_checks_cnf",
                        JsonValue::Bool(self.control.model_checks_cnf),
                    ),
                    ("replayed", JsonValue::Bool(self.control.replayed)),
                ]),
            ),
            ("conflicts", int(self.conflicts)),
            ("propagations", int(self.propagations)),
            ("worst_step_conflicts", oint(self.worst_step_conflicts)),
            ("steps", int(self.steps)),
            ("solve_ms", flt(self.solve_ms)),
            ("worst_step_solve_ms", flt(self.worst_step_solve_ms)),
            ("drat_bytes", oint(self.drat_bytes)),
            (
                "check_ms",
                self.check_ms.map(flt).unwrap_or(JsonValue::Null),
            ),
            ("check_work", oint(self.check_work)),
            (
                "pass_default_limits",
                self.pass_default_limits
                    .map(JsonValue::Bool)
                    .unwrap_or(JsonValue::Null),
            ),
            ("peak_rss_kb", int(self.peak_rss_kb)),
            ("green", JsonValue::Bool(self.green)),
            ("cap", JsonValue::String(self.cap.clone())),
            ("note", JsonValue::String(self.note.clone())),
        ];
        // Omitted when it is the default 120 s, so recorded rows keep their bytes.
        if let Some(w) = self.wall_s {
            fields.push(("wall_s", int(w)));
        }
        fields.push((
            "metrics",
            JsonValue::Object(
                self.metrics
                    .iter()
                    .map(|(k, v)| (k.clone(), flt(*v)))
                    .collect(),
            ),
        ));
        o(fields)
    }

    /// Read a row back from its JSON.
    pub fn from_json(v: &JsonValue) -> Result<Row, String> {
        let JsonValue::Object(pairs) = v else {
            return Err("row is not an object".into());
        };
        let get = |k: &str| {
            pairs
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, x)| x)
                .ok_or_else(|| format!("row has no field {k}"))
        };
        let s = |k: &str| -> Result<String, String> {
            match get(k)? {
                JsonValue::String(x) => Ok(x.clone()),
                _ => Err(format!("{k} is not a string")),
            }
        };
        let os = |k: &str| -> Result<Option<String>, String> {
            match get(k)? {
                JsonValue::String(x) => Ok(Some(x.clone())),
                JsonValue::Null => Ok(None),
                _ => Err(format!("{k} is not a string or null")),
            }
        };
        let u = |k: &str| -> Result<u64, String> {
            match get(k)? {
                JsonValue::Number(JsonNumber::Integer(i)) if *i >= 0 => Ok(*i as u64),
                _ => Err(format!("{k} is not a non-negative integer")),
            }
        };
        let ou = |k: &str| -> Result<Option<u64>, String> {
            match get(k)? {
                JsonValue::Null => Ok(None),
                JsonValue::Number(JsonNumber::Integer(i)) if *i >= 0 => Ok(Some(*i as u64)),
                _ => Err(format!("{k} is not a non-negative integer or null")),
            }
        };
        let fl = |x: &JsonValue| -> Option<f64> {
            match x {
                JsonValue::Number(JsonNumber::Integer(i)) => Some(*i as f64),
                JsonValue::Number(JsonNumber::Float(f)) => Some(*f),
                _ => None,
            }
        };
        let f = |k: &str| -> Result<f64, String> {
            fl(get(k)?).ok_or_else(|| format!("{k} is not a number"))
        };
        let of = |k: &str| -> Result<Option<f64>, String> {
            match get(k)? {
                JsonValue::Null => Ok(None),
                x => fl(x)
                    .map(Some)
                    .ok_or_else(|| format!("{k} is not a number or null")),
            }
        };
        let b = |x: &JsonValue, k: &str| -> Result<bool, String> {
            match x {
                JsonValue::Bool(b) => Ok(*b),
                _ => Err(format!("{k} is not a bool")),
            }
        };
        let JsonValue::Object(cp) = get("control")? else {
            return Err("control is not an object".into());
        };
        let cget = |k: &str| {
            cp.iter()
                .find(|(n, _)| n == k)
                .map(|(_, x)| x)
                .ok_or_else(|| format!("control has no field {k}"))
        };
        let control = Control {
            result: match cget("result")? {
                JsonValue::String(x) => x.clone(),
                _ => return Err("control.result is not a string".into()),
            },
            model_checks_cnf: b(cget("model_checks_cnf")?, "control.model_checks_cnf")?,
            replayed: b(cget("replayed")?, "control.replayed")?,
        };
        let metrics = match get("metrics")? {
            JsonValue::Object(m) => m
                .iter()
                .filter_map(|(k, x)| fl(x).map(|n| (k.clone(), n)))
                .collect(),
            _ => return Err("metrics is not an object".into()),
        };
        Ok(Row {
            experiment: s("experiment")?,
            knob: Knob::from_json(get("knob")?)?,
            mode: s("mode")?,
            tier: s("tier")?,
            seed: u("seed")?,
            seeds: u32::try_from(u("seeds")?).map_err(|_| "seeds does not fit in 32 bits")?,
            instance_sha256: s("instance_sha256")?,
            drat_sha256: os("drat_sha256")?,
            control_instance_sha256: s("control_instance_sha256")?,
            ands: u("ands")?,
            vars: u("vars")?,
            clauses: u("clauses")?,
            entropy_bits: u("entropy_bits")?,
            declared_bits: u("declared_bits")?,
            result: s("result")?,
            collapsed_reason: os("collapsed_reason")?,
            control,
            conflicts: u("conflicts")?,
            propagations: u("propagations")?,
            worst_step_conflicts: ou("worst_step_conflicts")?,
            steps: u("steps")?,
            solve_ms: f("solve_ms")?,
            worst_step_solve_ms: f("worst_step_solve_ms")?,
            drat_bytes: ou("drat_bytes")?,
            check_ms: of("check_ms")?,
            check_work: ou("check_work")?,
            pass_default_limits: match get("pass_default_limits")? {
                JsonValue::Null => None,
                JsonValue::Bool(x) => Some(*x),
                _ => return Err("pass_default_limits is not a bool or null".into()),
            },
            peak_rss_kb: u("peak_rss_kb")?,
            green: b(get("green")?, "green")?,
            cap: s("cap")?,
            note: s("note")?,
            metrics,
            wall_s: match pairs.iter().find(|(n, _)| n == "wall_s") {
                None => None,
                Some((_, JsonValue::Number(JsonNumber::Integer(i)))) if *i >= 0 => Some(*i as u64),
                Some(_) => return Err("wall_s is not a non-negative integer".into()),
            },
        })
    }

    /// One-line JSON text.
    pub fn to_line(&self) -> String {
        serialize(&self.to_json()).unwrap_or_default()
    }

    /// Parse a row from a JSON line.
    pub fn from_line(line: &str) -> Result<Row, String> {
        let v = parse(line).map_err(|e| format!("not JSON: {e}"))?;
        Row::from_json(&v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Row {
        let mut r = Row::new(
            "B3",
            Knob::of(&[
                ("length", Kv::I(100)),
                ("divergence", Kv::F(0.25)),
                ("cut", Kv::S("every8".into())),
            ]),
            "certified",
            3,
            5,
        );
        r.instance_sha256 = "ab".repeat(32);
        r.drat_sha256 = Some("cd".repeat(32));
        r.result = "unsat".into();
        r.control = Control {
            result: "sat".into(),
            model_checks_cnf: true,
            replayed: true,
        };
        r.worst_step_conflicts = Some(12);
        r.check_ms = Some(1.5);
        r.pass_default_limits = Some(true);
        r.metrics = vec![("diverged".into(), 25.0)];
        r
    }

    #[test]
    fn a_row_round_trips_through_json() {
        let r = sample();
        let back = Row::from_line(&r.to_line()).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn wall_s_is_omitted_when_default_round_trips_and_is_strict() {
        let mut r = sample();
        assert!(!r.to_line().contains("wall_s"));
        r.wall_s = Some(600);
        assert!(r.to_line().contains("\"wall_s\":600"));
        assert_eq!(Row::from_line(&r.to_line()).unwrap(), r);
        let bad = r.to_line().replace("\"wall_s\":600", "\"wall_s\":\"600\"");
        assert!(Row::from_line(&bad).unwrap_err().contains("wall_s"));
        let neg = r.to_line().replace("\"wall_s\":600", "\"wall_s\":-1");
        assert!(Row::from_line(&neg).is_err());
    }

    #[test]
    fn a_missing_or_mistyped_field_is_a_parse_error_not_a_default() {
        let mut v = sample().to_json();
        if let JsonValue::Object(p) = &mut v {
            p.retain(|(k, _)| k != "ands");
        }
        assert!(Row::from_json(&v).unwrap_err().contains("ands"));
        let line = sample()
            .to_line()
            .replace("\"seed\":3", "\"seed\":\"three\"");
        assert!(Row::from_line(&line).is_err());
        assert!(Row::from_line("not json").is_err());
        assert!(Row::from_json(&JsonValue::Null).is_err());
    }

    #[test]
    fn knob_accessors_and_the_grouping_key() {
        let k = Knob::of(&[("n", Kv::I(8)), ("x", Kv::F(0.5)), ("s", Kv::S("a".into()))]);
        assert_eq!((k.int("n"), k.f("x"), k.s("s")), (8, 0.5, "a"));
        assert_eq!((k.int("zzz"), k.f("zzz"), k.s("zzz")), (0, 0.0, ""));
        assert_eq!(k.f("n"), 8.0);
        assert_eq!(k.int("x"), 0);
        assert_eq!(k.key(), "{\"n\":8,\"x\":0.5,\"s\":\"a\"}");
        assert_eq!(Knob::from_json(&k.to_json()).unwrap(), k);
        assert!(Knob::from_json(&JsonValue::Null).is_err());
        assert!(Knob::from_json(&JsonValue::Object(vec![(
            "a".into(),
            JsonValue::Bool(true)
        )]))
        .is_err());
    }

    #[test]
    fn a_float_knob_that_is_a_whole_number_keeps_its_type() {
        // divergence 1.0 and 0.0 are real floats in the plan; they must not come back as integers.
        for d in [0.0, 1.0, 0.25] {
            let r = Row::new(
                "B2",
                Knob::of(&[("divergence", Kv::F(d))]),
                "monolithic",
                1,
                5,
            );
            let back = Row::from_line(&r.to_line()).unwrap();
            assert_eq!(back.knob, r.knob, "divergence {d}: {}", r.to_line());
        }
    }

    #[test]
    fn a_control_is_ok_only_when_it_did_its_job() {
        assert!(Control {
            result: "sat".into(),
            model_checks_cnf: true,
            replayed: true
        }
        .ok());
        assert!(Control {
            result: "caught".into(),
            model_checks_cnf: true,
            replayed: true
        }
        .ok());
        assert!(!Control {
            result: "sat".into(),
            model_checks_cnf: true,
            replayed: false
        }
        .ok());
        assert!(!Control {
            result: "unsat".into(),
            model_checks_cnf: true,
            replayed: true
        }
        .ok());
        assert!(!Control::none("not run").ok());
    }
}
