fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let command = if args.is_empty() {
        "cat /hello".into()
    } else {
        args.join(" ")
    };
    println!("{:#}", unix_history_simulator::run_command(&command));
}
