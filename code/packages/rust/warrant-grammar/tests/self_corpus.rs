//! Run the authored syntax corpus and its adapter controls through TF01.
use coding_adventures_test_bootstrap::run::{self, Suite};
use coding_adventures_test_bootstrap::tfv;

mod support;
use support::{check, negative, Fault, CORPUS};

fn main() {
    let records = match tfv::parse_file(CORPUS) {
        Ok(records) => records,
        Err(_) => {
            eprintln!("invalid embedded syntax transport");
            std::process::exit(1);
        }
    };
    let mut suite = Suite::new();
    for record in &records {
        let record = record.clone();
        suite.add(record.id.clone(), move |ctx| {
            check(ctx, &record, Fault::None)
        });
    }
    for (id, case, fault) in [
        (
            "adapter/always-accept",
            "declarations/unknown-block",
            Fault::AlwaysAccept,
        ),
        (
            "adapter/forget-witnesses",
            "operators/multiply",
            Fault::ForgetWitnesses,
        ),
        (
            "adapter/wrong-precedence",
            "operators/multiply",
            Fault::WrongPrecedence,
        ),
    ] {
        let record = records
            .iter()
            .find(|r| r.id == case)
            .expect("control case exists")
            .clone();
        suite.add(id, move |ctx| negative(ctx, &record, fault));
    }
    std::process::exit(run::main(&suite));
}
