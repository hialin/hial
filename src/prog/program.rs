use crate::{
    api::*,
    config::ColorPalette,
    debug,
    pprint::pprint,
    prog::{searcher::Searcher, *},
};
use std::collections::HashMap;
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
    VarBind(String, PathStart<'a>, Path<'a>),
}

#[derive(Clone, Debug)]
pub struct Executor<'a> {
    program: Program<'a>,
    statement_index: usize,
    searcher: Option<Searcher<'a>>,
}

#[derive(Clone, Debug, Default)]
pub struct ExecutionContext {
    vars: HashMap<String, Xell>,
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
            Statement::VarBind(name, start, path) => write!(f, "${} := {}{}", name, start, path)?,
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
        let mut ctx = ExecutionContext::default();
        self.run_in_context(params, &mut ctx)
    }

    pub fn run_in_context(&self, params: ProgramParams, ctx: &mut ExecutionContext) -> Res<()> {
        for statement in &self.0 {
            debug!("Running statement: {}", statement);
            match statement {
                Statement::VarBind(name, start, path) => {
                    let value = Self::eval_to_single_cell(ctx, start, path.clone())?;
                    ctx.vars.insert(name.clone(), value);
                }
                Statement::Assignment(start, path, value) => {
                    ifdebug!(println!("-- Assignment: {}{} = {}", start, path, value));
                    let searcher = Searcher::new(Self::resolve_start(ctx, start)?, path.clone());
                    for cell in searcher {
                        cell?.write().value(value.clone())?;
                    }
                }
                Statement::Path(start, path) => {
                    ifdebug!(println!("-- PathWithStart: {} {}", start, path));
                    let searcher = Searcher::new(Self::resolve_start(ctx, start)?, path.clone());
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

    fn resolve_start(ctx: &ExecutionContext, start: &PathStart<'a>) -> Res<Xell> {
        match start {
            PathStart::Var(name) => ctx
                .vars
                .get(name)
                .cloned()
                .ok_or_else(|| inputerr(format!("undefined variable :{}", name))),
            _ => start.eval(),
        }
    }

    fn eval_to_single_cell(
        ctx: &ExecutionContext,
        start: &PathStart<'a>,
        path: Path<'a>,
    ) -> Res<Xell> {
        let mut searcher = Searcher::new(Self::resolve_start(ctx, start)?, path.clone());
        let first = match searcher.next() {
            Some(Ok(cell)) => cell,
            Some(Err(err)) => return Err(err),
            None => return noresm(format!("{}{}", start, path)),
        };
        Ok(first)
    }
}
