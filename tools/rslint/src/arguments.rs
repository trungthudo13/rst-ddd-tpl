use std::{
    collections::BTreeSet,
    error::Error,
    path::Path,
};

use proc_macro2::{
    Delimiter,
    Group,
    TokenStream,
    TokenTree,
};
use syn::{
    Expr,
    Token,
    parse::{
        ParseStream,
        Parser,
    },
    spanned::Spanned,
    visit::{
        self,
        Visit,
    },
};

use crate::rust_sources;

#[derive(Default)]
struct Arguments {
    positions: BTreeSet<(usize, usize)>,
}

struct InlineStructs<'a> {
    positions: &'a mut BTreeSet<(usize, usize)>,
}

impl<'a> InlineStructs<'a> {
    fn new(positions: &'a mut BTreeSet<(usize, usize)>) -> Self {
        Self {
            positions,
        }
    }
}

fn macro_expressions(value: &syn::Macro) -> Vec<Expr> {
    if value.path.segments.last().is_some_and(|segment| {
        matches!(
            segment.ident.to_string().as_str(),
            "matches" | "assert_matches"
        )
    }) {
        return (|input: ParseStream<'_>| {
            let matched = input.parse::<Expr>()?;
            input.parse::<Token![,]>()?;
            input.call(syn::Pat::parse_multi_with_leading_vert)?;
            let mut expressions = vec![matched];
            if input.peek(Token![if]) {
                input.parse::<Token![if]>()?;
                expressions.push(input.parse::<Expr>()?);
            }
            let _: TokenStream = input.parse()?;
            Ok(expressions)
        })
        .parse2(value.tokens.clone())
        .unwrap_or_default();
    }
    let array = TokenStream::from(TokenTree::Group(Group::new(
        Delimiter::Bracket,
        value.tokens.clone(),
    )));
    syn::parse2::<Expr>(array).into_iter().collect()
}

impl<'ast> Visit<'ast> for InlineStructs<'_> {
    fn visit_expr_struct(
        &mut self,
        expression: &'ast syn::ExprStruct,
    ) {
        let start = expression.path.span().start();
        self.positions.insert((start.line, start.column + 1));
    }

    fn visit_local(
        &mut self,
        _: &'ast syn::Local,
    ) {
    }

    fn visit_item(
        &mut self,
        _: &'ast syn::Item,
    ) {
    }

    fn visit_expr_closure(
        &mut self,
        _: &'ast syn::ExprClosure,
    ) {
    }

    fn visit_expr_async(
        &mut self,
        _: &'ast syn::ExprAsync,
    ) {
    }

    fn visit_macro(
        &mut self,
        value: &'ast syn::Macro,
    ) {
        for expression in macro_expressions(value) {
            self.visit_expr(&expression);
        }
    }
}

impl<'ast> Visit<'ast> for Arguments {
    fn visit_expr_call(
        &mut self,
        expression: &'ast syn::ExprCall,
    ) {
        for argument in &expression.args {
            InlineStructs::new(&mut self.positions).visit_expr(argument);
        }
        visit::visit_expr_call(self, expression);
    }

    fn visit_expr_method_call(
        &mut self,
        expression: &'ast syn::ExprMethodCall,
    ) {
        for argument in &expression.args {
            InlineStructs::new(&mut self.positions).visit_expr(argument);
        }
        visit::visit_expr_method_call(self, expression);
    }

    fn visit_macro(
        &mut self,
        value: &'ast syn::Macro,
    ) {
        for expression in macro_expressions(value) {
            self.visit_expr(&expression);
        }
    }
}

fn positions(source: &str) -> Result<BTreeSet<(usize, usize)>, syn::Error> {
    let syntax = syn::parse_file(source)?;
    let mut arguments = Arguments::default();
    arguments.visit_file(&syntax);
    Ok(arguments.positions)
}

pub(crate) fn check_arguments(root: &Path) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;
    for (path, source) in rust_sources(root)? {
        for (line, column) in positions(&source).map_err(|error| format!("{path}: {error}"))? {
            eprintln!(
                "{path}:{line}:{column}: error: assign this struct construction to a local \
                 variable before passing it as an argument [repo::named_arguments]"
            );
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests;
