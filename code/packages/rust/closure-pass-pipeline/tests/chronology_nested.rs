//! Same-name nested real pipelines reproduce the independent CV03 witness.
//! Callback counts and graph tags are observed inside Pass::run, before looking
//! at journal records. A caught child failure must remain explicit evidence.
use coding_adventures_closure_pass_pipeline::{
    Pass, PassContext, PassError, PassOutput, PassPipeline, PassStats,
};
use coding_adventures_correlation_vector::{CVLog, Contribution, GraphLimits};
use coding_adventures_javascript_ast::{Program, SourceType};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

struct Observer {
    outer: bool,
    fail: bool,
    recover: bool,
    timeline: Arc<Mutex<Vec<String>>>,
}
impl Pass for Observer {
    fn name(&self) -> &'static str {
        "same"
    }
    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        let role = if self.outer {
            "outer"
        } else if self.fail {
            "inner-failed"
        } else {
            "inner-ok"
        };
        self.timeline.lock().unwrap().push(format!("{role}:enter"));
        let id = ctx.program.cv.as_ref().unwrap();
        ctx.cv
            .contribute(id, "same", &format!("{role}:direct"), HashMap::new())
            .unwrap();
        if self.fail {
            return Err(PassError {
                pass_name: "same".into(),
                message: "nested failure witness".into(),
            });
        }
        if self.outer {
            let outer = ctx.cv.journal().unwrap().events().last().unwrap().context();
            let mut inner = PassPipeline::new();
            inner.add(Box::new(Observer {
                outer: false,
                fail: true,
                recover: false,
                timeline: self.timeline.clone(),
            }));
            let error = inner
                .run(ctx.program.clone(), ctx.sidecar, ctx.cv)
                .unwrap_err();
            assert_eq!(error.message, "nested failure witness");
            if !self.recover {
                return Err(error);
            }
            ctx.cv
                .contribute(id, "same", "outer:caught", HashMap::new())
                .unwrap();
            assert_eq!(
                ctx.cv.journal().unwrap().events().last().unwrap().context(),
                outer
            );
            let mut retry = PassPipeline::new();
            retry.add(Box::new(Observer {
                outer: false,
                fail: false,
                recover: false,
                timeline: self.timeline.clone(),
            }));
            retry.run(ctx.program.clone(), ctx.sidecar, ctx.cv).unwrap();
            ctx.cv
                .contribute(id, "same", "outer:after-retry", HashMap::new())
                .unwrap();
            assert_eq!(
                ctx.cv.journal().unwrap().events().last().unwrap().context(),
                outer
            );
        }
        Ok(PassOutput {
            program: ctx.program.clone(),
            contributions: vec![Contribution {
                source: "same".into(),
                tag: format!("{role}:returned"),
                meta: HashMap::new(),
            }],
            changed: false,
            diagnostics: vec![],
            stats: PassStats::default(),
        })
    }
}
#[test]
fn nested_caught_and_propagated_errors_have_actual_order_and_exact_charges() {
    for recover in [true, false] {
        // Independent C+D+J+S counts from the committed predecessor witness.
        let cap = if recover { 33 } else { 17 };
        let mut cv = CVLog::new_checked_chronology(GraphLimits {
            max_events: cap,
            ..Default::default()
        })
        .unwrap();
        let root = cv.try_create(None).unwrap();
        let timeline = Arc::new(Mutex::new(Vec::new()));
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(Observer {
            outer: true,
            fail: false,
            recover,
            timeline: timeline.clone(),
        }));
        let result = pipeline.run(
            Program::new(root.clone(), EsVersion::Es2025, SourceType::Module),
            &Sidecar::new(),
            &mut cv,
        );
        assert_eq!(result.is_ok(), recover);
        assert_eq!(timeline.lock().unwrap().len(), if recover { 3 } else { 2 });
        let tags: Vec<_> = cv
            .get(&root)
            .unwrap()
            .contributions
            .iter()
            .map(|c| c.tag.as_str())
            .collect();
        let expected = if recover {
            vec![
                "outer:direct",
                "inner-failed:direct",
                "outer:caught",
                "inner-ok:direct",
                "inner-ok:returned",
                "outer:after-retry",
                "outer:returned",
            ]
        } else {
            vec!["outer:direct", "inner-failed:direct"]
        };
        assert_eq!(tags, expected);
        let text = cv.to_json_string().unwrap();
        let wire: serde_json::Value = serde_json::from_str(&text).unwrap();
        let records = wire["journal"]["events"].as_array().unwrap();
        assert_eq!(records.len(), if recover { 23 } else { 13 });
        assert_eq!(
            records
                .iter()
                .filter(|r| r["event"]["scope"]["kind"] == "pass")
                .count(),
            if recover { 3 } else { 2 }
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| r["event"]["outcome"]["kind"] == "callback_failure")
                .count(),
            if recover { 2 } else { 4 }
        );
        for imported in [
            CVLog::from_json_string(&text).unwrap(),
            CVLog::from_checked_json(
                &text,
                GraphLimits {
                    max_events: cap,
                    ..Default::default()
                },
            )
            .unwrap(),
        ] {
            assert_eq!(imported.to_json_string().unwrap(), text);
        }
        assert!(
            cv.try_create(None).is_err(),
            "exact retained accounting leaves no event capacity"
        );
        assert_eq!(cv.to_json_string().unwrap(), text);
    }
}
