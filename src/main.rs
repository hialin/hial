use hiallib::{
    api::*,
    config::{self, ColorPalette},
    prog::{Program, ProgramParams},
    *,
};

#[derive(Clone, Debug, Default)]
struct Args {
    depth: Option<usize>,
    breadth: Option<usize>,
    color_palette: ColorPalette,
    program: String,
}

fn main() -> Res<()> {
    let args = parse_args(&config::load_main_config()?)?;

    if args.program.is_empty() {
        eprintln!("No program given.");
        return Ok(());
    }

    debug!("Command: run {}", args.program);
    let program = Program::parse(&args.program)?;
    let params = ProgramParams {
        print_depth: args.depth.unwrap_or(usize::MAX),
        print_breadth: args.breadth.unwrap_or(0),
        color_palette: args.color_palette,
    };
    program.run(params)?;
    Ok(())
}

fn parse_args(config: &config::MainConfig) -> Res<Args> {
    let mut args = Args::default();
    if let Some(palette) = config.color_palette {
        args.color_palette = palette;
    }

    let mut args_iter = std::env::args().skip(1).peekable();
    let mut in_flags = true;
    while let Some(a) = args_iter.next() {
        match a.as_str() {
            "-v" | "--verbose" if in_flags => {
                utils::log::set_verbose(true);
            }
            "-d" if in_flags => {
                args.depth = args_iter.next().and_then(|s| s.parse().ok());
            }
            "-b" if in_flags => {
                args.breadth = args_iter.next().and_then(|s| s.parse().ok());
            }
            "--no-color" if in_flags => {
                args.color_palette = ColorPalette::None;
            }
            "--color" if in_flags => {
                if let Some(palette) = args_iter.next() {
                    args.color_palette = match palette.as_str() {
                        "dark" => ColorPalette::Dark,
                        "light" => ColorPalette::Light,
                        _ => ColorPalette::None,
                    };
                }
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
