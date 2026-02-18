use crate::{
    api::*,
    config::ColorPalette,
    debug,
    pprint::pprint,
    prog::{searcher::Searcher, *},
};
use std::fmt::{Display, Formatter};

macro_rules! ifdebug {
    ( $body:expr ) => {
        // $body
    };
}

#[derive(Clone, Debug)]
pub struct Program<'a>(pub(crate) Vec<Statement<'a>>);

#[derive(Clone, Debug, Default)]
pub struct ProgramParams {
    pub print_depth: usize,
    pub print_breadth: usize,
    pub color_palette: ColorPalette,
}

#[derive(Clone, Debug)]
pub enum Statement<'a> {
    Path(PathStart<'a>, Path<'a>),
    Assignment(PathStart<'a>, Path<'a>, OwnValue),
}

#[derive(Clone, Debug)]
pub struct Executor<'a> {
    program: Program<'a>,
    statement_index: usize,
    searcher: Option<Searcher<'a>>,
}

impl<'a> Display for Program<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for statement in &self.0 {
            writeln!(f, "{}", statement)?;
        }
        Ok(())
    }
}

impl<'a> Display for Statement<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Statement::Path(start, path) => write!(f, "{}{}", start, path)?,
            Statement::Assignment(start, path, value) => {
                write!(f, "{}{} = {}", start, path, value)?
            }
        }
        Ok(())
    }
}

impl<'a> Program<'a> {
    pub fn parse(input: &str) -> Res<Program<'_>> {
        let input = input.trim();
        super::parse_program::parse_program(input)
    }

    pub fn run(&self, params: ProgramParams) -> Res<()> {
        for statement in &self.0 {
            debug!("Running statement: {}", statement);
            match statement {
                Statement::Assignment(start, path, value) => {
                    ifdebug!(println!("-- Assignment: {}{} = {}", start, path, value));
                    let searcher = Searcher::new(start.eval()?, path.clone());
                    for cell in searcher {
                        cell?.write().value(value.clone())?;
                    }
                }
                Statement::Path(start, path) => {
                    ifdebug!(println!("-- PathWithStart: {} {}", start, path));
                    let searcher = Searcher::new(start.eval()?, path.clone());
                    for cell in searcher {
                        pprint(
                            &cell?,
                            params.print_depth,
                            params.print_breadth,
                            params.color_palette,
                        );
                    }
                }
            }
        }

        Ok(())
    }
}
