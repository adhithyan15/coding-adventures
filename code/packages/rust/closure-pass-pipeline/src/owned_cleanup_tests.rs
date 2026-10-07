//! Owned AST cleanup regressions use a child process because recursive drop
//! stack overflow aborts the process instead of returning a Rust panic.
use super::*;
use coding_adventures_javascript_ast::{
    Expression, ExpressionStatement, Identifier, MemberExpression, NumericLiteral, ProgramItem,
    SourceType, Statement,
};
use coding_adventures_javascript_tokens::EsVersion;
use std::cell::RefCell;

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

struct CandidatePass(RefCell<Option<Program>>, bool);
impl Pass for CandidatePass {
    fn name(&self) -> &'static str {
        "candidate"
    }
    fn run(&self, _: PassContext<'_>) -> Result<PassOutput, PassError> {
        Ok(PassOutput {
            program: self.0.borrow_mut().take().unwrap(),
            contributions: if self.1 {
                vec![Contribution {
                    source: "candidate".into(),
                    tag: "examined".into(),
                    meta: HashMap::new(),
                }]
            } else {
                vec![]
            },
            changed: false,
            diagnostics: vec![],
            stats: PassStats::default(),
        })
    }
}
struct ErrorPass(bool);
impl Pass for ErrorPass {
    fn name(&self) -> &'static str {
        "failure"
    }
    fn depends_on(&self) -> &[&'static str] {
        if self.0 {
            &["missing"]
        } else {
            &[]
        }
    }
    fn run(&self, _: PassContext<'_>) -> Result<PassOutput, PassError> {
        Err(PassError {
            pass_name: "failure".into(),
            message: "injected pass error".into(),
        })
    }
}

#[test]
fn checked_cv_owned_program_cleanup_is_iterative() {
    const CHILD: &str = "CV02_OWNED_PROGRAM_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "owned_cleanup_tests::checked_cv_owned_program_cleanup_is_iterative",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "isolated AST cleanup failed: {:?}\n{}",
            result.status,
            String::from_utf8_lossy(&result.stderr)
        );
        return;
    }
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            for scenario in 0..4 {
                let mut cv =
                    CVLog::new_checked_compact(coding_adventures_correlation_vector::GraphLimits {
                        max_events: 0,
                        ..Default::default()
                    })
                    .unwrap();
                let id = cv.try_create(None).unwrap();
                let flat = Program::new(id.clone(), EsVersion::Es2025, SourceType::Module);
                let mut pipeline = PassPipeline::new();
                eprintln!("owned cleanup scenario {scenario}");
                let input = match scenario {
                    0 => {
                        pipeline.add(Box::new(CandidatePass(
                            RefCell::new(Some(deep_program(id))),
                            true,
                        )));
                        flat
                    }
                    1 | 2 => {
                        pipeline.add(Box::new(ErrorPass(scenario == 2)));
                        deep_program(id)
                    }
                    _ => {
                        pipeline.add(Box::new(CandidatePass(RefCell::new(Some(flat)), false)));
                        deep_program(id)
                    }
                };
                let result = pipeline.run(input, &Sidecar::new(), &mut cv);
                match scenario {
                    0 => assert!(result.unwrap_err().message.contains("events limit")),
                    1 => assert_eq!(result.unwrap_err().message, "injected pass error"),
                    2 => assert!(result.is_err()),
                    _ => assert!(result.unwrap().program.body.is_empty()),
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
