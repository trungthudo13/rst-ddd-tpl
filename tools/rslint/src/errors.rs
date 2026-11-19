use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    error::Error as ToolError,
    path::Path,
};

use proc_macro2::{
    Delimiter,
    Span,
    TokenStream,
    TokenTree,
};
use syn::{
    Attribute,
    Expr,
    FnArg,
    Item,
    Meta,
    Pat,
    ReturnType,
    Token,
    Type,
    UseTree,
    punctuated::Punctuated,
    spanned::Spanned,
    visit::{
        self,
        Visit,
    },
};

use crate::{
    macro_template,
    rust_sources,
};

mod mappers;

const SHARED_FILE: &str = "apps/shared/src/errors.rs";
const SHARED_ERROR: &[&str] = &["shared", "errors", "Error"];
const HTTP_ADAPTER: &str = "src/api/error.rs";
const GRPC_MAPPER: &str = "src/internal/mappers/errors.rs";
const API_WRAPPER: &[&str] = &["app", "api", "error", "ApiError"];

type Names = Vec<String>;

#[derive(Clone, PartialEq, Eq)]
enum Shape {
    Path(Names, Vec<Shape>),
    Tuple(Vec<Shape>),
    Inferred,
    Other,
}

impl Shape {
    fn path(
        &self,
        expected: &[&str],
    ) -> bool {
        matches!(self, Self::Path(path, _) if same(path, expected))
    }
}

struct Source {
    path: String,
    scope: Names,
    syntax: syn::File,
}

#[derive(Clone)]
struct Alias {
    scope: Names,
    target: Names,
}

#[derive(Clone)]
struct TypeAlias {
    scope: Names,
    item: syn::ItemType,
}

#[derive(Clone, Default)]
struct Catalog {
    declarations: BTreeSet<Names>,
    imports: BTreeMap<Names, Alias>,
    globs: BTreeMap<Names, Vec<Names>>,
    types: BTreeMap<Names, TypeAlias>,
    functions: BTreeMap<Names, (Names, Type)>,
    variants: BTreeSet<String>,
    canonical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    path: String,
    line: usize,
    column: usize,
    reason: String,
}

fn same(
    path: &[String],
    expected: &[&str],
) -> bool {
    path.iter().map(String::as_str).eq(expected.iter().copied())
}

fn names(path: &syn::Path) -> Names {
    path.segments
        .iter()
        .map(|part| part.ident.to_string().trim_start_matches("r#").to_owned())
        .collect()
}

fn scoped(
    scope: &[String],
    name: &str,
) -> Names {
    let mut key = scope.to_vec();
    key.push(name.to_owned());
    key
}

fn service_source(path: &str) -> bool {
    path.starts_with("src/")
        || ["shared", "domain", "application", "infrastructure"]
            .iter()
            .any(|package| path.starts_with(&format!("apps/{package}/src/")))
}

fn source_scope(path: &str) -> Names {
    let parts: Vec<_> = path.split('/').collect();
    let start = parts.iter().position(|part| *part == "src").unwrap_or(0);
    let package = if start == 0 { "app" } else { parts[start - 1] };
    let mut scope = vec![package.to_owned()];
    for part in &parts[start + 1..] {
        let name = part.strip_suffix(".rs").unwrap_or(part);
        if !matches!(name, "lib" | "main" | "mod") {
            scope.push(name.to_owned());
        }
    }
    scope
}

fn test_meta(meta: &Meta) -> bool {
    match meta {
        | Meta::Path(path) => path.is_ident("test"),
        | Meta::List(list) => {
            let Ok(items) = list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            else {
                return false;
            };
            if list.path.is_ident("all") {
                items.iter().any(test_meta)
            }
            else if list.path.is_ident("any") {
                !items.is_empty() && items.iter().all(test_meta)
            }
            else {
                false
            }
        }
        | Meta::NameValue(_) => false,
    }
}

fn test_only(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("test")
            || (attribute.path().is_ident("cfg")
                && attribute.parse_args::<Meta>().is_ok_and(|meta| test_meta(&meta)))
    })
}

fn error_name(name: &str) -> bool {
    name.ends_with("Error") || name.ends_with("Exception")
}

fn error_derive(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("derive")
            && attribute
                .parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                .is_ok_and(|paths| {
                    paths
                        .iter()
                        .any(|path| path.segments.last().is_some_and(|part| part.ident == "Error"))
                })
    })
}

fn item_attributes(item: &Item) -> &[Attribute] {
    match item {
        | Item::Const(item) => &item.attrs,
        | Item::Enum(item) => &item.attrs,
        | Item::ExternCrate(item) => &item.attrs,
        | Item::Fn(item) => &item.attrs,
        | Item::ForeignMod(item) => &item.attrs,
        | Item::Impl(item) => &item.attrs,
        | Item::Macro(item) => &item.attrs,
        | Item::Mod(item) => &item.attrs,
        | Item::Static(item) => &item.attrs,
        | Item::Struct(item) => &item.attrs,
        | Item::Trait(item) => &item.attrs,
        | Item::TraitAlias(item) => &item.attrs,
        | Item::Type(item) => &item.attrs,
        | Item::Union(item) => &item.attrs,
        | _ => &[],
    }
}

impl Catalog {
    fn import(
        &mut self,
        scope: &[String],
        prefix: &[String],
        tree: &UseTree,
    ) {
        match tree {
            | UseTree::Path(tree) => {
                self.import(scope, &scoped(prefix, &tree.ident.to_string()), &tree.tree)
            }
            | UseTree::Name(tree) => {
                let name = tree.ident.to_string();
                let (alias, target) = if name == "self" {
                    let Some(alias) = prefix.last()
                    else {
                        return;
                    };
                    (alias.clone(), prefix.to_vec())
                }
                else {
                    (name.clone(), scoped(prefix, &name))
                };
                let imported = Alias {
                    scope: scope.to_vec(),
                    target,
                };
                self.imports.insert(scoped(scope, &alias), imported);
            }
            | UseTree::Rename(tree) => {
                let target = if tree.ident == "self" {
                    prefix.to_vec()
                }
                else {
                    scoped(prefix, &tree.ident.to_string())
                };
                let imported = Alias {
                    scope: scope.to_vec(),
                    target,
                };
                self.imports.insert(scoped(scope, &tree.rename.to_string()), imported);
            }
            | UseTree::Group(tree) => {
                for tree in &tree.items {
                    self.import(scope, prefix, tree);
                }
            }
            | UseTree::Glob(_) => {
                self.globs.entry(scope.to_vec()).or_default().push(prefix.to_vec())
            }
        }
    }

    fn collect(
        &mut self,
        scope: &[String],
        items: &[Item],
    ) {
        for item in items {
            match item {
                | Item::Mod(module) => {
                    let scope = scoped(scope, &module.ident.to_string());
                    self.declarations.insert(scope.clone());
                    if let Some((_, items)) = &module.content {
                        self.collect(&scope, items);
                    }
                }
                | Item::Use(item) => self.import(scope, &[], &item.tree),
                | Item::Type(item) => {
                    let key = scoped(scope, &item.ident.to_string());
                    self.declarations.insert(key.clone());
                    let alias = TypeAlias {
                        scope: scope.to_vec(),
                        item: item.clone(),
                    };
                    self.types.insert(key, alias);
                }
                | Item::Enum(item) => {
                    let key = scoped(scope, &item.ident.to_string());
                    self.declarations.insert(key.clone());
                    for variant in &item.variants {
                        self.declarations.insert(scoped(&key, &variant.ident.to_string()));
                    }
                    if same(&key, SHARED_ERROR) {
                        self.canonical = true;
                        self.variants
                            .extend(item.variants.iter().map(|variant| variant.ident.to_string()));
                    }
                }
                | Item::Struct(item) => {
                    self.declarations.insert(scoped(scope, &item.ident.to_string()));
                }
                | Item::Trait(item) => {
                    self.declarations.insert(scoped(scope, &item.ident.to_string()));
                }
                | Item::Fn(item) => {
                    let key = scoped(scope, &item.sig.ident.to_string());
                    self.declarations.insert(key.clone());
                    if let ReturnType::Type(_, ty) = &item.sig.output {
                        self.functions.insert(key, (scope.to_vec(), (**ty).clone()));
                    }
                }
                | _ => {}
            }
        }
    }

    fn known(
        &self,
        path: &[String],
    ) -> bool {
        self.declarations.contains(path) || self.imports.contains_key(path)
    }

    fn resolve(
        &self,
        scope: &[String],
        path: &[String],
        depth: usize,
    ) -> Names {
        if path.is_empty() || depth > 32 {
            return path.to_vec();
        }
        if path[0] == "crate" {
            let mut target = vec![scope[0].clone()];
            target.extend_from_slice(&path[1..]);
            return self.resolve_absolute(&target, depth + 1);
        }
        if path[0] == "self" || path[0] == "super" {
            let mut target = scope.to_vec();
            let mut index = 0;
            while index < path.len() && matches!(path[index].as_str(), "self" | "super") {
                if path[index] == "super" && target.len() > 1 {
                    target.pop();
                }
                index += 1;
            }
            target.extend_from_slice(&path[index..]);
            return self.resolve_absolute(&target, depth + 1);
        }
        for end in (1..=scope.len()).rev() {
            let candidate = scoped(&scope[..end], &path[0]);
            if let Some(alias) = self.imports.get(&candidate) {
                let mut target = self.resolve(&alias.scope, &alias.target, depth + 1);
                target.extend_from_slice(&path[1..]);
                return self.resolve_absolute(&target, depth + 1);
            }
            let mut candidate = scope[..end].to_vec();
            candidate.extend_from_slice(path);
            if self.known(&candidate) {
                return self.resolve_absolute(&candidate, depth + 1);
            }
            if let Some(globs) = self.globs.get(&scope[..end]) {
                for glob in globs {
                    let mut candidate = self.resolve(&scope[..end], glob, depth + 1);
                    candidate.extend_from_slice(path);
                    if self.known(&candidate) {
                        return self.resolve_absolute(&candidate, depth + 1);
                    }
                }
            }
        }
        let builtin = match path {
            | [name] if name == "Result" => {
                Some(vec!["std".into(), "result".into(), "Result".into()])
            }
            | [name] if name == "String" => {
                Some(vec!["std".into(), "string".into(), "String".into()])
            }
            | _ => None,
        };
        builtin.unwrap_or_else(|| self.resolve_absolute(path, depth + 1))
    }

    fn resolve_absolute(
        &self,
        path: &[String],
        depth: usize,
    ) -> Names {
        if depth > 32 {
            return path.to_vec();
        }
        for end in (1..=path.len()).rev() {
            if let Some(alias) = self.imports.get(&path[..end]) {
                let mut target = self.resolve(&alias.scope, &alias.target, depth + 1);
                target.extend_from_slice(&path[end..]);
                if target != path {
                    return self.resolve_absolute(&target, depth + 1);
                }
            }
        }
        path.to_vec()
    }
}

struct Inspector {
    catalog: Catalog,
    path: String,
    scope: Names,
    violations: Vec<Violation>,
    test: bool,
    associated: BTreeMap<String, Shape>,
    self_type: Option<Shape>,
    variables: BTreeMap<String, Shape>,
    origins: BTreeSet<String>,
    mapper_target: bool,
    impl_trait: Option<Names>,
    complete_match: bool,
}

impl Inspector {
    fn new(
        path: String,
        scope: Names,
        catalog: Catalog,
        test: bool,
    ) -> Self {
        Self {
            catalog,
            path,
            scope,
            violations: Vec::new(),
            test,
            associated: BTreeMap::new(),
            self_type: None,
            variables: BTreeMap::new(),
            origins: BTreeSet::new(),
            mapper_target: false,
            impl_trait: None,
            complete_match: false,
        }
    }

    fn report(
        &mut self,
        span: Span,
        reason: impl Into<String>,
    ) {
        let violation = mappers::diagnostics::violation(&self.path, span, reason.into());
        self.violations.push(violation);
    }

    fn shape(
        &self,
        ty: &Type,
    ) -> Shape {
        mappers::shapes::shape(&self.catalog, &self.scope, ty, &self.associated)
    }

    fn shared(
        &self,
        ty: &Shape,
    ) -> bool {
        ty.path(SHARED_ERROR)
    }

    fn status_contract(
        &self,
        success: &Shape,
        error: &Shape,
    ) -> bool {
        self.path.starts_with("src/internal/")
            && (self.path.contains("/handlers/") || self.path.ends_with("/service.rs"))
            && success.path(&["tonic", "Response"])
            && error.path(&["tonic", "Status"])
    }

    fn allowed(
        &self,
        error: &Shape,
    ) -> bool {
        self.shared(error)
            || (self.path.starts_with("src/api/") && error.path(API_WRAPPER))
            || error.path(&["std", "convert", "Infallible"])
            || error.path(&["core", "convert", "Infallible"])
    }

    fn check_shape(
        &mut self,
        shape: &Shape,
        span: Span,
    ) {
        if let Shape::Path(path, arguments) = shape
            && path.last().is_some_and(|name| name == "Result")
        {
            let standard = same(path, &["std", "result", "Result"])
                || same(path, &["core", "result", "Result"]);
            let allowed = standard
                && arguments.len() == 2
                && (self.allowed(&arguments[1])
                    || arguments[1] == Shape::Inferred
                    || self.status_contract(&arguments[0], &arguments[1]));
            if !allowed {
                self.report(
                    span,
                    "Result errors must use shared::Error; define a named variant in \
                     apps/shared/src/errors.rs and map provider failures explicitly",
                );
            }
        }
    }

    fn wrapper(
        &self,
        item: &syn::ItemStruct,
    ) -> bool {
        self.path == "src/api/error.rs"
            && item.ident == "ApiError"
            && matches!(&item.fields, syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 && self.shared(&self.shape(&fields.unnamed[0].ty)))
            && !error_derive(&item.attrs)
    }

    fn function(
        &mut self,
        signature: &syn::Signature,
        block: &syn::Block,
    ) {
        let variables = std::mem::take(&mut self.variables);
        let origins = std::mem::take(&mut self.origins);
        let mapper_target = self.mapper_target;
        self.mapper_target = (self.path == HTTP_ADAPTER
            && signature.ident == "into_response"
            && self.self_type.as_ref().is_some_and(|ty| ty.path(API_WRAPPER))
            && self
                .impl_trait
                .as_ref()
                .is_some_and(|path| same(path, &["axum", "response", "IntoResponse"])))
            || (self.path == GRPC_MAPPER && signature.ident == "map_error");
        for argument in &signature.inputs {
            match argument {
                | FnArg::Typed(argument) => {
                    self.bind(&argument.pat, self.shape(&argument.ty));
                }
                | FnArg::Receiver(_) => {
                    if let Some(ty) = self.self_type.clone() {
                        self.variables.insert("self".into(), ty);
                    }
                }
            }
        }
        if self.mapper_target {
            self.origins.extend(
                self.variables
                    .iter()
                    .filter(|(_, ty)| self.shared(ty) || ty.path(API_WRAPPER))
                    .map(|(name, _)| name.clone())
                    .collect::<Vec<_>>(),
            );
        }
        self.visit_signature(signature);
        self.visit_block(block);
        self.variables = variables;
        self.origins = origins;
        self.mapper_target = mapper_target;
    }

    fn input_origin(
        &self,
        expression: &Expr,
    ) -> bool {
        match expression {
            | Expr::Path(expression) => {
                expression.path.segments.len() == 1
                    && self.origins.contains(&expression.path.segments[0].ident.to_string())
            }
            | Expr::Reference(expression) => self.input_origin(&expression.expr),
            | Expr::Paren(expression) => self.input_origin(&expression.expr),
            | Expr::Unary(expression) if matches!(expression.op, syn::UnOp::Deref(_)) => {
                self.input_origin(&expression.expr)
            }
            | Expr::Field(expression) => self.input_origin(&expression.base),
            | Expr::MethodCall(expression)
                if matches!(
                    expression.method.to_string().as_str(),
                    "clone" | "as_ref" | "borrow" | "to_owned"
                ) =>
            {
                self.input_origin(&expression.receiver)
            }
            | _ => false,
        }
    }

    fn bind(
        &mut self,
        pattern: &Pat,
        shape: Shape,
    ) {
        match pattern {
            | Pat::Ident(pattern) => {
                self.variables.insert(pattern.ident.to_string(), shape.clone());
                if let Some((_, pattern)) = &pattern.subpat {
                    self.bind(pattern, shape);
                }
            }
            | Pat::Type(pattern) => self.bind(&pattern.pat, self.shape(&pattern.ty)),
            | Pat::Reference(pattern) => self.bind(&pattern.pat, shape),
            | Pat::Tuple(pattern) => {
                if let Shape::Tuple(types) = shape {
                    for (pattern, ty) in pattern.elems.iter().zip(types) {
                        self.bind(pattern, ty);
                    }
                }
            }
            | Pat::TupleStruct(pattern) => {
                if let Shape::Path(path, arguments) = shape {
                    let name = pattern.path.segments.last().map(|part| part.ident.to_string());
                    let index = match name.as_deref() {
                        | Some("Ok" | "Some") => Some(0),
                        | Some("Err") => Some(1),
                        | _ => None,
                    };
                    if path.last().is_some_and(|name| matches!(name.as_str(), "Result" | "Option"))
                        && let Some(ty) = index.and_then(|index| arguments.get(index))
                        && let Some(pattern) = pattern.elems.first()
                    {
                        self.bind(pattern, ty.clone());
                    }
                }
            }
            | _ => {}
        }
    }

    fn expression(
        &self,
        expression: &Expr,
    ) -> Option<Shape> {
        match expression {
            | Expr::Path(expression) => {
                if expression.path.segments.len() == 1
                    && let Some(variable) =
                        self.variables.get(&expression.path.segments[0].ident.to_string())
                {
                    return Some(variable.clone());
                }
                let target = self.catalog.resolve(&self.scope, &names(&expression.path), 0);
                (target.len() == SHARED_ERROR.len() + 1
                    && same(&target[..SHARED_ERROR.len()], SHARED_ERROR))
                .then(|| {
                    Shape::Path(
                        SHARED_ERROR.iter().map(|part| (*part).to_owned()).collect(),
                        Vec::new(),
                    )
                })
            }
            | Expr::Reference(expression) => self.expression(&expression.expr),
            | Expr::Paren(expression) => self.expression(&expression.expr),
            | Expr::Unary(expression) if matches!(expression.op, syn::UnOp::Deref(_)) => {
                self.expression(&expression.expr)
            }
            | Expr::MethodCall(expression)
                if matches!(
                    expression.method.to_string().as_str(),
                    "clone" | "as_ref" | "borrow" | "to_owned"
                ) =>
            {
                self.expression(&expression.receiver)
            }
            | Expr::Field(expression) => {
                let parent = self.expression(&expression.base)?;
                if parent.path(API_WRAPPER)
                    && matches!(&expression.member, syn::Member::Unnamed(index) if index.index == 0)
                {
                    Some(Shape::Path(
                        SHARED_ERROR.iter().map(|part| (*part).to_owned()).collect(),
                        Vec::new(),
                    ))
                }
                else {
                    None
                }
            }
            | Expr::Call(expression) => {
                if let Some(ty) = self.expression(&expression.func) {
                    return Some(ty);
                }
                let Expr::Path(path) = &*expression.func
                else {
                    return None;
                };
                let target = self.catalog.resolve(&self.scope, &names(&path.path), 0);
                self.catalog
                    .functions
                    .get(&target)
                    .map(|(scope, ty)| self.catalog.shape(scope, ty, &self.associated))
            }
            | Expr::Try(expression) => {
                match self.expression(&expression.expr)? {
                    | Shape::Path(path, arguments)
                        if path.last().is_some_and(|name| name == "Result") =>
                    {
                        arguments.first().cloned()
                    }
                    | _ => None,
                }
            }
            | _ => None,
        }
    }

    fn variant(
        &self,
        path: &syn::Path,
    ) -> Option<String> {
        let mut path = names(path);
        if path.first().is_some_and(|name| name == "Self")
            && self.self_type.as_ref().is_some_and(|ty| self.shared(ty))
        {
            let mut target: Names = SHARED_ERROR.iter().map(|part| (*part).to_owned()).collect();
            target.extend_from_slice(&path[1..]);
            path = target;
        }
        let path = self.catalog.resolve(&self.scope, &path, 0);
        (path.len() == SHARED_ERROR.len() + 1 && same(&path[..SHARED_ERROR.len()], SHARED_ERROR))
            .then(|| path.last().unwrap().clone())
    }

    fn check_variant(
        &mut self,
        path: &syn::Path,
    ) {
        if let Some(variant) = self.variant(path)
            && variant.chars().next().is_some_and(char::is_uppercase)
            && !self.catalog.variants.contains(&variant)
        {
            self.report(
                path.span(),
                format!(
                    "undefined shared error variant {variant}; define it in \
                     apps/shared/src/errors.rs and add explicit transport actions"
                ),
            );
        }
    }

    fn pattern_variants(
        &self,
        pattern: &Pat,
        variants: &mut BTreeSet<String>,
    ) {
        let path = match pattern {
            | Pat::Path(pattern) => Some(&pattern.path),
            | Pat::TupleStruct(pattern) => Some(&pattern.path),
            | Pat::Struct(pattern) => Some(&pattern.path),
            | Pat::Or(pattern) => {
                for pattern in &pattern.cases {
                    self.pattern_variants(pattern, variants);
                }
                None
            }
            | Pat::Paren(pattern) => {
                self.pattern_variants(&pattern.pat, variants);
                None
            }
            | Pat::Reference(pattern) => {
                self.pattern_variants(&pattern.pat, variants);
                None
            }
            | Pat::Ident(pattern) => {
                if let Some((_, pattern)) = &pattern.subpat {
                    self.pattern_variants(pattern, variants);
                }
                else if let Some(variant) = self.variant(&syn::Path::from(pattern.ident.clone()))
                {
                    variants.insert(variant);
                }
                None
            }
            | _ => None,
        };
        if let Some(variant) = path.and_then(|path| self.variant(path)) {
            variants.insert(variant);
        }
    }

    fn catch_all(
        &self,
        pattern: &Pat,
    ) -> bool {
        match pattern {
            | Pat::Wild(_) => true,
            | Pat::Ident(pattern) => {
                pattern.subpat.as_ref().map_or_else(
                    || self.variant(&syn::Path::from(pattern.ident.clone())).is_none(),
                    |(_, pattern)| self.catch_all(pattern),
                )
            }
            | Pat::Or(pattern) => pattern.cases.iter().any(|pattern| self.catch_all(pattern)),
            | Pat::Paren(pattern) => self.catch_all(&pattern.pat),
            | Pat::Reference(pattern) => self.catch_all(&pattern.pat),
            | _ => false,
        }
    }

    fn macro_tokens(
        &mut self,
        tokens: TokenStream,
    ) {
        if let Ok(file) = syn::parse2::<syn::File>(tokens.clone()) {
            self.catalog.collect(&self.scope, &file.items);
            self.visit_file(&file);
        }
        else if let Ok(expression) = syn::parse2::<Expr>(tokens.clone()) {
            self.visit_expr(&expression);
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

impl<'ast> Visit<'ast> for Inspector {
    fn visit_item(
        &mut self,
        item: &'ast Item,
    ) {
        let test = self.test;
        self.test |= test_only(item_attributes(item));
        visit::visit_item(self, item);
        self.test = test;
    }

    fn visit_item_mod(
        &mut self,
        item: &'ast syn::ItemMod,
    ) {
        let test = self.test;
        self.test |= test_only(&item.attrs);
        self.scope.push(item.ident.to_string());
        visit::visit_item_mod(self, item);
        self.scope.pop();
        self.test = test;
    }

    fn visit_item_enum(
        &mut self,
        item: &'ast syn::ItemEnum,
    ) {
        let key = scoped(&self.scope, &item.ident.to_string());
        let canonical = self.path == SHARED_FILE && same(&key, SHARED_ERROR);
        if canonical {
            if self.test {
                self.report(
                    item.ident.span(),
                    "the canonical shared Error enum must be available in production builds",
                );
            }
            if !matches!(item.vis, syn::Visibility::Public(_)) {
                self.report(
                    item.ident.span(),
                    "the canonical shared Error enum must be public",
                );
            }
            if item.attrs.iter().any(|attribute| attribute.path().is_ident("non_exhaustive")) {
                self.report(
                    item.ident.span(),
                    "shared Error must remain exhaustive so every added variant requires \
                     transport handling",
                );
            }
        }
        else if error_name(&item.ident.to_string()) || error_derive(&item.attrs) {
            self.report(
                item.ident.span(),
                "error variants must be defined in the single shared::Error enum in \
                 apps/shared/src/errors.rs",
            );
        }
        visit::visit_item_enum(self, item);
    }

    fn visit_item_struct(
        &mut self,
        item: &'ast syn::ItemStruct,
    ) {
        if (error_name(&item.ident.to_string()) || error_derive(&item.attrs)) && !self.wrapper(item)
        {
            self.report(
                item.ident.span(),
                "custom error structs are forbidden; define an error variant in \
                 apps/shared/src/errors.rs",
            );
        }
        visit::visit_item_struct(self, item);
    }

    fn visit_item_type(
        &mut self,
        item: &'ast syn::ItemType,
    ) {
        if error_name(&item.ident.to_string()) && !self.shared(&self.shape(&item.ty)) {
            self.report(
                item.ident.span(),
                "error aliases must resolve to shared::Error; define a named shared error variant \
                 instead",
            );
        }
        visit::visit_item_type(self, item);
    }

    fn visit_item_impl(
        &mut self,
        item: &'ast syn::ItemImpl,
    ) {
        let associated = std::mem::take(&mut self.associated);
        let self_type = self.self_type.take();
        let impl_trait = self.impl_trait.take();
        self.self_type = Some(self.shape(&item.self_ty));
        for member in &item.items {
            if let syn::ImplItem::Type(member) = member
                && matches!(member.ident.to_string().as_str(), "Error" | "Err")
            {
                let ty = self.shape(&member.ty);
                if !self.test && !self.allowed(&ty) {
                    self.report(
                        member.ty.span(),
                        "associated error types must resolve to shared::Error",
                    );
                }
                self.associated.insert(member.ident.to_string(), ty);
            }
        }
        if let Some((_, path, _)) = &item.trait_ {
            let path = self.catalog.resolve(&self.scope, &names(path), 0);
            self.impl_trait = Some(path.clone());
            if (same(&path, &["std", "error", "Error"]) || same(&path, &["core", "error", "Error"]))
                && !self.self_type.as_ref().is_some_and(|ty| self.shared(ty))
            {
                self.report(
                    item.impl_token.span(),
                    "only shared::Error may implement the service error contract",
                );
            }
        }
        visit::visit_item_impl(self, item);
        self.associated = associated;
        self.self_type = self_type;
        self.impl_trait = impl_trait;
    }

    fn visit_expr_path(
        &mut self,
        expression: &'ast syn::ExprPath,
    ) {
        self.check_variant(&expression.path);
        visit::visit_expr_path(self, expression);
    }

    fn visit_expr_struct(
        &mut self,
        expression: &'ast syn::ExprStruct,
    ) {
        self.check_variant(&expression.path);
        visit::visit_expr_struct(self, expression);
    }

    fn visit_pat_tuple_struct(
        &mut self,
        pattern: &'ast syn::PatTupleStruct,
    ) {
        self.check_variant(&pattern.path);
        visit::visit_pat_tuple_struct(self, pattern);
    }

    fn visit_type(
        &mut self,
        ty: &'ast Type,
    ) {
        if !self.test {
            self.check_shape(&self.shape(ty), ty.span());
        }
        visit::visit_type(self, ty);
    }

    fn visit_item_fn(
        &mut self,
        item: &'ast syn::ItemFn,
    ) {
        let test = self.test;
        self.test |= test_only(&item.attrs);
        self.function(&item.sig, &item.block);
        self.test = test;
    }

    fn visit_impl_item_fn(
        &mut self,
        item: &'ast syn::ImplItemFn,
    ) {
        let test = self.test;
        self.test |= test_only(&item.attrs);
        self.function(&item.sig, &item.block);
        self.test = test;
    }

    fn visit_trait_item_fn(
        &mut self,
        item: &'ast syn::TraitItemFn,
    ) {
        let test = self.test;
        self.test |= test_only(&item.attrs);
        if let Some(block) = &item.default {
            self.function(&item.sig, block);
        }
        else {
            self.visit_signature(&item.sig);
        }
        self.test = test;
    }

    fn visit_block(
        &mut self,
        block: &'ast syn::Block,
    ) {
        let catalog = self.catalog.clone();
        let variables = self.variables.clone();
        let origins = self.origins.clone();
        let items: Vec<_> = block
            .stmts
            .iter()
            .filter_map(|statement| {
                if let syn::Stmt::Item(item) = statement { Some(item.clone()) } else { None }
            })
            .collect();
        self.catalog.collect(&self.scope, &items);
        visit::visit_block(self, block);
        self.catalog = catalog;
        self.variables = variables;
        self.origins = origins;
    }

    fn visit_local(
        &mut self,
        local: &'ast syn::Local,
    ) {
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Pat::Ident(pattern) = &local.pat {
                if self.input_origin(&init.expr) {
                    self.origins.insert(pattern.ident.to_string());
                }
                else {
                    self.origins.remove(&pattern.ident.to_string());
                }
            }
            if let Some(ty) = self.expression(&init.expr) {
                self.bind(&local.pat, ty);
            }
            if let Some((_, diverge)) = &init.diverge {
                self.visit_expr(diverge);
            }
        }
        if let Pat::Type(pattern) = &local.pat {
            self.bind(&pattern.pat, self.shape(&pattern.ty));
        }
        self.visit_pat(&local.pat);
    }

    fn visit_expr_closure(
        &mut self,
        expression: &'ast syn::ExprClosure,
    ) {
        let variables = self.variables.clone();
        for input in &expression.inputs {
            if let Pat::Type(pattern) = input {
                self.bind(&pattern.pat, self.shape(&pattern.ty));
            }
        }
        visit::visit_expr_closure(self, expression);
        self.variables = variables;
    }

    fn visit_expr_match(
        &mut self,
        expression: &'ast syn::ExprMatch,
    ) {
        let mut variants = BTreeSet::new();
        for arm in &expression.arms {
            self.pattern_variants(&arm.pat, &mut variants);
        }
        let ty = self.expression(&expression.expr);
        let shared = !variants.is_empty() || ty.as_ref().is_some_and(|ty| self.shared(ty));
        if shared && !self.test {
            let mut covered = BTreeSet::new();
            for arm in &expression.arms {
                if self.catch_all(&arm.pat) {
                    self.report(
                        arm.pat.span(),
                        "shared Error matches must name every variant explicitly; wildcard and \
                         catch-all bindings hide newly added errors",
                    );
                }
                if arm.guard.is_none() {
                    self.pattern_variants(&arm.pat, &mut covered);
                }
            }
            let missing: Vec<_> = self.catalog.variants.difference(&covered).cloned().collect();
            if missing.is_empty() && self.mapper_target && self.input_origin(&expression.expr) {
                self.complete_match = true;
            }
            else if !missing.is_empty() {
                self.report(
                    expression.match_token.span(),
                    format!(
                        "shared Error handling is missing explicit unguarded variants: {}; define \
                         an action for every error",
                        missing.join(", ")
                    ),
                );
            }
        }
        self.visit_expr(&expression.expr);
        for arm in &expression.arms {
            let variables = self.variables.clone();
            self.visit_pat(&arm.pat);
            if let Some(ty) = &ty {
                self.bind(&arm.pat, ty.clone());
            }
            if let Some((_, guard)) = &arm.guard {
                self.visit_expr(guard);
            }
            self.visit_expr(&arm.body);
            self.variables = variables;
        }
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
            for token in item.mac.tokens.clone() {
                if let TokenTree::Group(group) = token
                    && group.delimiter() == Delimiter::Brace
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

fn violations(sources: &[Source]) -> Vec<Violation> {
    let mut catalog = Catalog::default();
    for source in sources {
        catalog.collect(&source.scope, &source.syntax.items);
    }
    let mut violations = Vec::new();
    let tests = mappers::shapes::test_files(sources);
    if !catalog.canonical {
        let violation = Violation {
            path: SHARED_FILE.into(),
            line: 1,
            column: 1,
            reason: "define the single public service Error enum here; all service Result errors \
                     must use shared::Error"
                .into(),
        };
        violations.push(violation);
    }
    if !sources.iter().any(|source| source.path == HTTP_ADAPTER) {
        let violation = Violation {
            path: HTTP_ADAPTER.into(),
            line: 1,
            column: 1,
            reason: "define the central HTTP error adapter with explicit handling for every \
                     shared::Error variant"
                .into(),
        };
        violations.push(violation);
    }
    for source in sources {
        let test = tests.contains(Path::new(&source.path));
        let mut inspector = Inspector::new(
            source.path.clone(),
            source.scope.clone(),
            catalog.clone(),
            test,
        );
        inspector.visit_file(&source.syntax);
        if matches!(source.path.as_str(), HTTP_ADAPTER | GRPC_MAPPER) && !inspector.complete_match {
            inspector.report(
                Span::call_site(),
                "the central transport error adapter must explicitly match every shared::Error \
                 variant without wildcard arms",
            );
        }
        violations.extend(inspector.violations);
    }
    violations.sort_by(|left, right| {
        (&left.path, left.line, left.column, &left.reason).cmp(&(
            &right.path,
            right.line,
            right.column,
            &right.reason,
        ))
    });
    violations.dedup();
    violations
}

pub(crate) fn check_errors(root: &Path) -> Result<usize, Box<dyn ToolError>> {
    let mut sources = Vec::new();
    for (path, source) in rust_sources(root)? {
        if service_source(&path) {
            let syntax = syn::parse_file(&source).map_err(|error| format!("{path}: {error}"))?;
            let source = Source {
                scope: source_scope(&path),
                path,
                syntax,
            };
            sources.push(source);
        }
    }
    let violations = violations(&sources);
    for violation in &violations {
        eprintln!(
            "{}:{}:{}: error: {} [repo::shared_errors]",
            violation.path, violation.line, violation.column, violation.reason
        );
    }
    Ok(violations.len())
}

#[cfg(test)]
mod tests;
