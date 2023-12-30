use clap::{Parser, Subcommand};

use hiallib::{base::*, pathlang::Path, pprint::pprint, *};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Ls {
        #[arg(short, long)]
        verbose: bool,
        #[arg(short, long, default_value = "0")]
        depth: Option<usize>,
        #[arg(short, long, default_value = "0")]
        breadth: Option<usize>,
        path: Option<String>,
    },
    Test {
        #[arg(short, long)]
        verbose: bool,
    },
}

fn main() -> Res<()> {
    let args = Cli::parse();

    match &args.command {
        None => {
            debug!("No command.")
        }
        Some(Commands::Test { verbose }) => {
            utils::log::set_verbose(*verbose);
            debug!("Command: test");
        }
        Some(Commands::Ls {
            verbose,
            depth,
            breadth,
            path,
        }) => {
            utils::log::set_verbose(*verbose);
            let depth = depth.unwrap_or(0);
            let breadth = breadth.unwrap_or(0);
            let path = path.as_deref().unwrap_or("");
            debug!("Command: print {}", path);
            let (path_start, path) = match Path::parse_with_starter(path) {
                Ok(x) => x,
                Err(HErr::User(msg)) => {
                    eprintln!("Bad path: {}", msg);
                    return Ok(());
                }
                Err(err) => return Err(err),
            };
            debug!("Root: {}", path_start);
            debug!("Path: {}", path);
            let root = path_start.eval()?;

            let mut anyfound = false;
            for cell in path.eval(root) {
                anyfound = true;
                match cell {
                    Ok(cell) => pprint(&cell, depth, breadth),
                    Err(err) => eprintln!("{:?}", err),
                }
            }
            if !anyfound {
                debug!("No matches.")
            }
        }
    }
    Ok(())
}
