use skysheet_numeric_validation::{corpus, runner};

fn main() {
    let root = corpus::root();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = if args == ["--freeze-corpus"] {
        corpus::freeze(&root).map(|()| 0)
    } else {
        runner::run(&root, &args)
    };
    match result {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("tool_error: {error}");
            std::process::exit(2);
        }
    }
}
