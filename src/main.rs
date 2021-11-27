use clap::{App, Arg, SubCommand};
use hiallib::{base::*, pathlang::Path, pprint::pprint, *};

fn main() -> Res<()> {
    let args = parse_args()?;

    set_verbose(args.verbose);
    // verbose!("{:?}", args);

    match args.command {
        Command::None => {
            verbose!("No command.")
        }
        Command::Print(command) => {
            verbose!("Command: print {}", &command.path);
            let (cell_repr, path) = match Path::parse_with_starter(&command.path) {
                Ok(x) => x,
                Err(HErr::BadPath(msg)) => {
                    eprintln!("Bad path: {}", msg);
                    return Ok(());
                }
                Err(err) => return Err(err),
            };
            verbose!("Root: {}", cell_repr);
            verbose!("Path: {}", path);
            let root = cell_repr.eval()?;

            let mut anyfound = false;
            for cell in path.eval(root) {
                anyfound = true;
                match cell {
                    Ok(cell) => pprint(&cell, command.depth, command.breadth),
                    Err(err) => eprintln!("{:?}", err),
                }
            }
            if !anyfound {
                verbose!("No matches.")
            }
        }
        Command::PerfTest(command) => {
            verbose!("Command: perftest {:?}", &command);
            perftests::perftests(command.alloc_count);
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct Args {
    verbose: bool,
    command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    None,
    Print(PrintCommand),
    PerfTest(PerfTestCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrintCommand {
    path: String,
    depth: usize,
    breadth: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PerfTestCommand {
    alloc_count: usize,
}

fn parse_args() -> Res<Args> {
    let matches = App::new("hial")
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Verbose description of operations"),
        )
        .subcommand(
            SubCommand::with_name("print")
                .about("shows the tree of the path result")
                .arg(
                    Arg::with_name("path")
                        .help("hial path to evaluate and print the result of")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("depth")
                        .help("maximum depth")
                        .short("d")
                        .long("depth")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("breadth")
                        .help("maximum breadth")
                        .short("b")
                        .long("breadth")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("perftest")
                .about("runs some performance tests")
                .arg(
                    Arg::with_name("alloc_count")
                        .help("numbers of allocations to count")
                        .required(true)
                        .short("c")
                        .long("alloc_count")
                        .takes_value(true),
                ),
        )
        .get_matches();

    let mut args = Args {
        verbose: false,
        command: Command::None,
    };

    args.verbose = matches.is_present("verbose");

    if let Some(command) = matches.subcommand_matches("print") {
        let path = command.value_of("path").unwrap_or("").to_string();
        let depth = guard_ok!(command.value_of("depth").unwrap_or("0").parse(), _x => {
            return HErr::BadArgument("cannot parse `depth` argument as integer".into()).into()
        });
        let breadth = guard_ok!(command.value_of("breadth").unwrap_or("0").parse(), _x => {
            return HErr::BadArgument("cannot parse `breadth` argument as integer".into()).into()
        });
        args.command = Command::Print(PrintCommand {
            path,
            depth,
            breadth,
        });
    } else if let Some(command) = matches.subcommand_matches("perftest") {
        let alloc_count = guard_ok!(command.value_of("alloc_count").unwrap_or("0").parse(), _x => {
            return HErr::BadArgument("cannot parse `count` argument as integer".into()).into()
        });
        args.command = Command::PerfTest(PerfTestCommand { alloc_count });
    }

    Ok(args)
}
