use std::{
    collections::BTreeMap,
    error::Error,
    path::Path,
};

use proc_macro2::{
    Delimiter,
    Group,
    Span,
    TokenStream,
    TokenTree,
};
use syn::visit::{
    self,
    Visit,
};

use crate::{
    macro_template,
    rust_sources,
};

#[derive(Default)]
struct Declarations {
    positions: BTreeMap<(usize, usize), String>,
}

impl Declarations {
    fn report(
        &mut self,
        span: Span,
        name: &syn::Ident,
    ) {
        let position = span.start();
        self.positions.insert((position.line, position.column + 1), name.to_string());
    }

    fn macro_tokens(
        &mut self,
        tokens: TokenStream,
    ) {
        if let Ok(file) = syn::parse2::<syn::File>(tokens.clone()) {
            self.visit_file(&file);
        }
        else if let Ok(expression) = syn::parse2::<syn::Expr>(tokens.clone()) {
            self.visit_expr(&expression);
        }
        else if let Ok(block) = syn::parse2::<syn::Block>(TokenStream::from(TokenTree::Group(
            Group::new(Delimiter::Brace, tokens.clone()),
        ))) {
            self.visit_block(&block);
        }
        else {
            for token in tokens {
                if let TokenTree::Group(group) = token {
                    self.macro_tokens(group.stream());
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for Declarations {
    fn visit_item_enum(
        &mut self,
        item: &'ast syn::ItemEnum,
    ) {
        self.report(item.enum_token.span, &item.ident);
        visit::visit_item_enum(self, item);
    }

    fn visit_item_const(
        &mut self,
        item: &'ast syn::ItemConst,
    ) {
        self.report(item.const_token.span, &item.ident);
        visit::visit_item_const(self, item);
    }

    fn visit_impl_item_const(
        &mut self,
        item: &'ast syn::ImplItemConst,
    ) {
        self.report(item.const_token.span, &item.ident);
        visit::visit_impl_item_const(self, item);
    }

    fn visit_trait_item_const(
        &mut self,
        item: &'ast syn::TraitItemConst,
    ) {
        self.report(item.const_token.span, &item.ident);
        visit::visit_trait_item_const(self, item);
    }

    fn visit_macro(
        &mut self,
        value: &'ast syn::Macro,
    ) {
        self.macro_tokens(value.tokens.clone());
    }

    fn visit_item_macro(
        &mut self,
        item: &'ast syn::ItemMacro,
    ) {
        if item.mac.path.is_ident("macro_rules") {
            let tokens: Vec<_> = item.mac.tokens.clone().into_iter().collect();
            for arm in tokens.windows(3) {
                if let [TokenTree::Punct(equal), TokenTree::Punct(arrow), TokenTree::Group(group)] =
                    arm
                    && equal.as_char() == '='
                    && arrow.as_char() == '>'
                {
                    self.macro_tokens(macro_template(group.stream()));
                }
            }
        }
        else {
            self.visit_macro(&item.mac);
        }
    }
}

fn checked_source(path: &str) -> bool {
    ["src/", "apps/domain/", "apps/application/", "apps/infrastructure/"]
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

fn declarations(source: &str) -> Result<BTreeMap<(usize, usize), String>, syn::Error> {
    let syntax = syn::parse_file(source)?;
    let mut declarations = Declarations::default();
    declarations.visit_file(&syntax);
    Ok(declarations.positions)
}

pub(crate) fn check_shared_values(root: &Path) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;
    for (path, source) in rust_sources(root)? {
        if !checked_source(&path) {
            continue;
        }
        for ((line, column), name) in
            declarations(&source).map_err(|error| format!("{path}: {error}"))?
        {
            eprintln!(
                "{path}:{line}:{column}: error: define enum or constant `{name}` in \
                 apps/shared/src/ and import or re-export it here [repo::shared_values]"
            );
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests;
