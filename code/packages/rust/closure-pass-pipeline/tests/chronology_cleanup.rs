//! Real deep owned AST/metadata values run on a small caller stack in an
//! isolated process, because a recursive drop overflow aborts instead of Err.
use coding_adventures_closure_pass_pipeline::{
    Pass, PassContext, PassError, PassOutput, PassPipeline, PassStats,
};
use coding_adventures_correlation_vector::{
    CVLog, Contribution, GraphLimits, JournalPolicy, PassOutcome, PipelineOutcome,
};
use coding_adventures_javascript_ast::{
    dispose_program, Expression, ExpressionStatement, Identifier, MemberExpression, NumericLiteral,
    Program, ProgramItem, SourceType, Statement,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;
use std::{cell::RefCell, collections::HashMap};

fn deep_program(id: String) -> Program {
    let mut expression = Expression::NumericLiteral(NumericLiteral {
        cv: None,
        value: 1.0,
        raw: "1".into(),
    });
    for _ in 0..4096 {
        expression = Expression::MemberExpression(MemberExpression {
            cv: None,
            object: Box::new(expression),
            property: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "p".into(),
            })),
            computed: false,
        });
    }
    Program::new(id, EsVersion::Es2025, SourceType::Module).with_body(vec![ProgramItem::Statement(
        Statement::expression_statement(ExpressionStatement {
            cv: None,
            expression,
        }),
    )])
}
struct Candidate {
    program: RefCell<Option<Program>>,
    deep_first: bool,
}
impl Pass for Candidate {
    fn name(&self) -> &'static str {
        "candidate"
    }
    fn run(&self, _: PassContext<'_>) -> Result<PassOutput, PassError> {
        let mut metadata = serde_json::Value::Null;
        for _ in 0..65_536 {
            metadata = serde_json::Value::Array(vec![metadata]);
        }
        let deep = Contribution {
            source: "candidate".into(),
            tag: "deep".into(),
            meta: HashMap::from([("deep".into(), metadata)]),
        };
        let empty = Contribution {
            source: "candidate".into(),
            tag: "empty".into(),
            meta: HashMap::new(),
        };
        Ok(PassOutput {
            program: self.program.borrow_mut().take().unwrap(),
            contributions: if self.deep_first {
                vec![deep, empty]
            } else {
                vec![empty, deep]
            },
            changed: false,
            diagnostics: vec![],
            stats: PassStats::default(),
        })
    }
}
struct NeverEntered;
impl Pass for NeverEntered {
    fn name(&self) -> &'static str {
        "never"
    }
    fn run(&self, _: PassContext<'_>) -> Result<PassOutput, PassError> {
        panic!("rejected begin entered callback")
    }
}

#[test]
fn context_rejection_and_reserved_completion_preserve_deep_owned_cleanup() {
    const CHILD: &str = "CV03_CHRONOLOGY_CLEANUP_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "context_rejection_and_reserved_completion_preserve_deep_owned_cleanup",
                "--nocapture",
            ])
            .env(CHILD, "1");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let result = command.output().unwrap();
        assert!(
            result.status.success(),
            "small-stack child failed: {:?}\n{}",
            result.status,
            String::from_utf8_lossy(&result.stderr)
        );
        return;
    }
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            for cap in [1, 6] {
                let mut cv = CVLog::new_checked_chronology(GraphLimits {
                    max_events: cap,
                    ..Default::default()
                })
                .unwrap();
                let root = cv.try_create(None).unwrap();
                let mut pipeline = PassPipeline::new();
                pipeline.add(Box::new(NeverEntered));
                assert!(pipeline
                    .run(deep_program(root), &Sidecar::new(), &mut cv)
                    .is_err());
                cv.validate_graph().unwrap();
            }
            for deep_first in [true, false] {
                let mut cv = CVLog::new_checked_chronology(GraphLimits {
                    max_events: 9,
                    ..Default::default()
                })
                .unwrap();
                let root = cv.try_create(None).unwrap();
                let input = Program::new(root.clone(), EsVersion::Es2025, SourceType::Module);
                let mut pipeline = PassPipeline::new();
                pipeline.add(Box::new(Candidate {
                    program: RefCell::new(Some(deep_program(root))),
                    deep_first,
                }));
                assert!(pipeline.run(input, &Sidecar::new(), &mut cv).is_err());
                cv.validate_graph().unwrap(); // ends still exist after safely draining pending values
            }
            let mut cv = CVLog::new_checked_chronology(GraphLimits {
                max_events: 7,
                ..Default::default()
            })
            .unwrap();
            let root = cv.try_create(None).unwrap();
            let returned = cv
                .with_pipeline(|cv| {
                    cv.record_schedule(&[("candidate", JournalPolicy::OneShot)], 1)
                        .unwrap();
                    let value = cv
                        .with_pass(0, 0, |_| {
                            Ok::<_, (String, PassOutcome)>((
                                deep_program(root),
                                PassOutcome::Accepted { changed: false },
                            ))
                        })
                        .unwrap();
                    Ok::<_, (String, PipelineOutcome)>((value, PipelineOutcome::Converged))
                })
                .unwrap();
            // Both terminal records were guaranteed; the generic successful value
            // reaches the caller intact and is disposed through the actual AST API.
            assert_eq!(cv.journal().unwrap().events().len(), 6);
            dispose_program(returned);
            cv.validate_graph().unwrap();
        })
        .unwrap()
        .join()
        .unwrap();
}
