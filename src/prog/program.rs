use crate::{
    api::*,
    debug, pprint,
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

#[derive(Clone, Debug)]
pub struct ProgramParams {
    pub print_depth: usize,
    pub print_breadth: usize,
}

#[derive(Clone, Debug)]
pub enum Statement<'a> {
    PathWithStart(PathStart<'a>, Path<'a>),
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
            Statement::PathWithStart(start, path) => write!(f, "{}{}", start, path)?,
        }
        Ok(())
    }
}

impl<'a> Program<'a> {
    pub fn parse(input: &str) -> Res<Program> {
        super::parse_program::parse_program(input)
    }

    pub fn run(&self, params: ProgramParams) -> Res<()> {
        for statement in &self.0 {
            debug!("Running statement: {}", statement);
            match statement {
                Statement::PathWithStart(start, path) => {
                    ifdebug!(println!("-- PathWithStart: {} {}", start, path));
                    let root = start.eval()?;
                    let mut searcher = Searcher::new(root, path.clone());
                    let Some(rescell) = searcher.next() else {
                        continue;
                    };
                    match rescell {
                        Ok(cell) => {
                            pprint(&cell, params.print_depth, params.print_breadth);
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                        }
                    };
                }
            }
        }

        Ok(())
    }
}
