use hiallib::{
    api::*,
    prog::{Program, ProgramParams},
    *,
};

#[derive(Clone, Debug, Default)]
struct Args {
    depth: Option<usize>,
    breadth: Option<usize>,
    program: String,
}

fn main() -> Res<()> {
    let args = parse_args()?;

    if args.program.is_empty() {
        eprintln!("No program given.");
        return Ok(());
    }

    debug!("Command: run {}", args.program);
    let program = Program::parse(&args.program)?;
    let params = ProgramParams {
        print_depth: args.depth.unwrap_or(0),
        print_breadth: args.breadth.unwrap_or(0),
    };
    program.run(params)?;
    Ok(())
}

fn parse_args() -> Res<Args> {
    let mut args = Args::default();

    let mut args_iter = std::env::args().skip(1).peekable();
    let mut in_flags = true;
    while let Some(a) = args_iter.next() {
        match a.as_str() {
            "-v" | "--verbose" if in_flags => {
                utils::log::set_verbose(true);
            }
            "-d" if in_flags => {
                args.depth = args_iter.peek().and_then(|s| s.parse().ok());
            }
            "-b" if in_flags => {
                args.breadth = args_iter.peek().and_then(|s| s.parse().ok());
            }
            "--" if in_flags => {
                in_flags = false;
            }
            _ => {
                in_flags = false;
                args.program += a.as_str();
                args.program += " ";
            }
        }
    }

    Ok(args)
}
