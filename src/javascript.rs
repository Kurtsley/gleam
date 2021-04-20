mod expression;
#[cfg(test)]
mod tests;

use crate::{ast::*, fs::Utf8Writer, line_numbers::LineNumbers, pretty::*};
use itertools::Itertools;

const INDENT: isize = 2;

pub type Output<'a> = Result<Document<'a>, Error>;

#[derive(Debug)]
pub struct Generator<'a> {
    line_numbers: &'a LineNumbers,
    module: &'a TypedModule,
}

impl<'a> Generator<'a> {
    pub fn new(line_numbers: &'a LineNumbers, module: &'a TypedModule) -> Self {
        Self {
            line_numbers,
            module,
        }
    }

    pub fn compile(&mut self) -> Output<'a> {
        let statements = self
            .module
            .statements
            .iter()
            .flat_map(|s| self.statement(s));
        let statements =
            Itertools::intersperse(statements, Ok(lines(2))).collect::<Result<Vec<_>, _>>()?;
        Ok(docvec!(r#""use strict";"#, lines(2), statements, line()))
    }

    pub fn statement(&mut self, statement: &'a TypedStatement) -> Option<Output<'a>> {
        match statement {
            Statement::TypeAlias { .. } => None,
            Statement::CustomType { .. } => None,
            Statement::Import { .. } => Some(unsupported("Importing modules")),
            Statement::ExternalType { .. } => None,
            Statement::ModuleConstant {
                public,
                name,
                value,
                ..
            } => Some(self.module_constant(*public, name, value)),
            Statement::Fn {
                arguments,
                name,
                body,
                public,
                ..
            } => Some(self.module_function(*public, name, arguments, body)),
            Statement::ExternalFn { .. } => Some(unsupported("Using an external function")),
        }
    }

    fn module_constant(
        &mut self,
        public: bool,
        name: &'a str,
        value: &'a TypedConstant,
    ) -> Output<'a> {
        let head = if public { "export const " } else { "const " };
        Ok(docvec![
            head,
            name,
            " = ",
            expression::constant_expression(value)?,
            ";",
        ])
    }

    fn module_function(
        &mut self,
        public: bool,
        name: &'a str,
        args: &'a [TypedArg],
        body: &'a TypedExpr,
    ) -> Output<'a> {
        let head = if public {
            "export function "
        } else {
            "function "
        };
        Ok(docvec![
            head,
            name,
            fun_args(args),
            " {",
            docvec![line(), expression::Generator::compile(body)?]
                .nest(INDENT)
                .group(),
            line(),
            "}",
        ])
    }
}

pub fn module(
    module: &TypedModule,
    line_numbers: &LineNumbers,
    writer: &mut impl Utf8Writer,
) -> Result<(), crate::Error> {
    Generator::new(line_numbers, module)
        .compile()
        .map_err(crate::Error::JavaScript)?
        .pretty_print(80, writer)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Unsupported { feature: String },
}

fn unsupported<M: ToString, T>(label: M) -> Result<T, Error> {
    Err(Error::Unsupported {
        feature: label.to_string(),
    })
}

fn fun_args<'a>(args: &'a [TypedArg]) -> Document<'a> {
    wrap_args(args.iter().map(|a| match &a.names {
        ArgNames::Discard { .. } | ArgNames::LabelledDiscard { .. } => "_".to_doc(),
        ArgNames::Named { name } | ArgNames::NamedLabelled { name, .. } => name.to_doc(),
    }))
}

fn wrap_args<'a, I>(args: I) -> Document<'a>
where
    I: Iterator<Item = Document<'a>>,
{
    break_("", "")
        .append(concat(Itertools::intersperse(args, break_(",", ", "))))
        .nest(INDENT)
        .append(break_("", ""))
        .surround("(", ")")
        .group()
}