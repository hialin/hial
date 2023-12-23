use clap::{App, Arg, SubCommand};
use hiallib::{base::*, pathlang::Path, pprint::pprint, utils::log::set_verbose, *};

fn main() -> Res<()> {
    let args = parse_args()?;

    set_verbose(args.verbose);
    // debug!("{:?}", args);

    match args.command {
        Command::None => {
            debug!("No command.")
        }
        Command::Print(command) => {
            debug!("Command: print {}", &command.path);
            let (cell_repr, path) = match Path::parse_with_starter(&command.path) {
                Ok(x) => x,
                Err(HErr::User(msg)) => {
                    eprintln!("Bad path: {}", msg);
                    return Ok(());
                }
                Err(err) => return Err(err),
            };
            debug!("Root: {}", cell_repr);
            debug!("Path: {}", path);
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
                debug!("No matches.")
            }
        }
        Command::PerfTest(command) => {
            debug!("Command: perftest {:?}", &command);
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
    Print(ListCommand),
    PerfTest(PerfTestCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ListCommand {
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
            SubCommand::with_name("ls")
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

    if let Some(command) = matches.subcommand_matches("ls") {
        let path = command.value_of("path").unwrap_or("").to_string();
        let depth = guard_ok!(command.value_of("depth").unwrap_or("0").parse(), _x => {
            return HErr::User("cannot parse `depth` argument as integer".into()).into()
        });
        let breadth = guard_ok!(command.value_of("breadth").unwrap_or("0").parse(), _x => {
            return HErr::User("cannot parse `breadth` argument as integer".into()).into()
        });
        args.command = Command::Print(ListCommand {
            path,
            depth,
            breadth,
        });
    } else if let Some(command) = matches.subcommand_matches("perftest") {
        let alloc_count = guard_ok!(command.value_of("alloc_count").unwrap_or("0").parse(), _x => {
            return HErr::User("cannot parse `count` argument as integer".into()).into()
        });
        args.command = Command::PerfTest(PerfTestCommand { alloc_count });
    }

    Ok(args)
}
