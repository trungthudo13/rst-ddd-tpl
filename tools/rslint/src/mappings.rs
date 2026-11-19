use std::{
    borrow::Cow,
    collections::{
        BTreeMap,
        BTreeSet,
    },
    error::Error,
    fs,
    path::{
        Component,
        Path,
        PathBuf,
    },
};

use proc_macro2::{
    Delimiter,
    Group,
    Span,
    TokenStream,
    TokenTree,
};
use syn::{
    Attribute,
    Expr,
    FnArg,
    GenericArgument,
    Item,
    Member,
    Meta,
    Pat,
    PathArguments,
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

use crate::rust_sources;

pub(crate) struct Source {
    pub(crate) path: PathBuf,
    pub(crate) scope: Vec<String>,
    pub(crate) syntax: syn::File,
}

#[derive(Clone)]
struct Model {
    fields: BTreeMap<String, Type>,
    data: bool,
}

#[derive(Clone, Default)]
struct Catalog {
    models: BTreeMap<Vec<String>, Model>,
    variants: BTreeMap<Vec<String>, BTreeMap<String, Type>>,
    types: BTreeMap<Vec<String>, (Vec<String>, syn::ItemType)>,
    functions: BTreeMap<Vec<String>, (Vec<String>, Type)>,
    aliases: BTreeMap<Vec<String>, (Vec<String>, Vec<String>)>,
    globs: BTreeMap<Vec<String>, Vec<Vec<String>>>,
}

#[derive(Clone, Debug)]
pub(crate) struct Violation {
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) reason: &'static str,
}

fn identifier(value: &syn::Ident) -> String {
    value.to_string().trim_start_matches("r#").to_owned()
}

fn path_names(path: &syn::Path) -> Vec<String> {
    path.segments.iter().map(|segment| identifier(&segment.ident)).collect()
}

fn normalized(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            | Component::ParentDir => {
                result.pop();
            }
            | Component::CurDir => {}
            | component => result.push(component),
        }
    }
    result
}

fn mapper_path(path: &Path) -> bool {
    normalized(path).parent().is_some_and(|parent| {
        parent.components().any(|component| component.as_os_str() == "mappers")
    })
}

fn source_scope(path: &Path) -> Vec<String> {
    let components: Vec<_> =
        path.iter().map(|value| value.to_string_lossy().into_owned()).collect();
    let source = components.iter().position(|component| component == "src");
    let (package, start) = match source {
        | Some(index) if index > 0 => (components[index - 1].replace('-', "_"), index + 1),
        | Some(_) => ("app".into(), 1),
        | None => ("app".into(), 0),
    };
    let mut scope = vec![package];
    for component in &components[start..] {
        let name = component.strip_suffix(".rs").unwrap_or(component);
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
        | Item::Use(item) => &item.attrs,
        | _ => &[],
    }
}

fn module_edges(
    source: &Path,
    directory: &Path,
    items: &[Item],
    inherited_test: bool,
    edges: &mut Vec<(PathBuf, PathBuf, bool)>,
) {
    for item in items {
        let Item::Mod(module) = item
        else {
            continue;
        };
        let testing = inherited_test || test_only(&module.attrs);
        if let Some((_, items)) = &module.content {
            module_edges(
                source,
                &directory.join(identifier(&module.ident)),
                items,
                testing,
                edges,
            );
        }
        else {
            let explicit = module.attrs.iter().find_map(|attribute| {
                if !attribute.path().is_ident("path") {
                    return None;
                }
                let Meta::NameValue(value) = &attribute.meta
                else {
                    return None;
                };
                let Expr::Lit(value) = &value.value
                else {
                    return None;
                };
                let syn::Lit::Str(value) = &value.lit
                else {
                    return None;
                };
                Some(value.value())
            });
            let target = explicit.map_or_else(
                || directory.join(format!("{}.rs", identifier(&module.ident))),
                |path| source.parent().unwrap_or(Path::new("")).join(path),
            );
            edges.push((source.to_owned(), normalized(&target), testing));
        }
    }
}

pub(crate) fn test_files(sources: &[Source]) -> BTreeSet<PathBuf> {
    let mut edges = Vec::new();
    let mut integration = BTreeSet::new();
    for source in sources {
        let stem = source.path.file_stem().unwrap_or_default();
        let parent = source.path.parent().unwrap_or(Path::new(""));
        let directory = if matches!(stem.to_str(), Some("lib" | "main" | "mod")) {
            parent.to_owned()
        }
        else {
            parent.join(stem)
        };
        module_edges(
            &source.path,
            &directory,
            &source.syntax.items,
            false,
            &mut edges,
        );
        let components: Vec<_> = source.path.iter().collect();
        if components
            .iter()
            .position(|value| *value == "tests")
            .is_some_and(|index| !components[..index].iter().any(|value| *value == "src"))
        {
            integration.insert(source.path.clone());
        }
    }
    let targets: BTreeSet<_> = edges.iter().map(|(_, target, _)| target.clone()).collect();
    let mut production: BTreeSet<_> = sources
        .iter()
        .filter(|source| !targets.contains(&source.path) && !integration.contains(&source.path))
        .map(|source| source.path.clone())
        .collect();
    let mut testing = integration;
    loop {
        let before = (production.len(), testing.len());
        for (source, target, test) in &edges {
            if production.contains(source) {
                if *test {
                    testing.insert(target.clone());
                }
                else {
                    production.insert(target.clone());
                }
            }
            if testing.contains(source) {
                testing.insert(target.clone());
            }
        }
        if before == (production.len(), testing.len()) {
            break;
        }
    }
    testing.difference(&production).cloned().collect()
}

fn qualify(
    scope: &[String],
    names: &[String],
) -> Vec<String> {
    let mut base = scope.to_vec();
    let mut start = 0;
    match names.first().map(String::as_str) {
        | Some("crate") => {
            base.truncate(1);
            start = 1;
        }
        | Some("self") => {
            start = 1;
        }
        | Some("super") => {
            while names.get(start).is_some_and(|name| name == "super") {
                if base.len() > 1 {
                    base.pop();
                }
                start += 1;
            }
        }
        | _ => return names.to_vec(),
    }
    [base, names[start..].to_vec()].concat()
}

impl Catalog {
    fn collect(
        &mut self,
        scope: &[String],
        items: &[Item],
    ) {
        for item in items {
            if test_only(item_attributes(item)) {
                continue;
            }
            match item {
                | Item::Mod(module) if !test_only(&module.attrs) => {
                    if let Some((_, items)) = &module.content {
                        self.collect(&[scope, &[identifier(&module.ident)]].concat(), items);
                    }
                }
                | Item::Use(item) => self.import(scope, &[], &item.tree),
                | Item::Type(item) => {
                    self.types.insert(
                        [scope, &[identifier(&item.ident)]].concat(),
                        (scope.to_vec(), item.clone()),
                    );
                    if let Type::Path(target) = &*item.ty {
                        self.aliases.insert(
                            [scope, &[identifier(&item.ident)]].concat(),
                            (scope.to_vec(), qualify(scope, &path_names(&target.path))),
                        );
                    }
                }
                | Item::Struct(item) => {
                    let fields = item
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(index, field)| {
                            (
                                field
                                    .ident
                                    .as_ref()
                                    .map(identifier)
                                    .unwrap_or_else(|| index.to_string()),
                                field.ty.clone(),
                            )
                        })
                        .collect();
                    let representation = scope
                        .iter()
                        .any(|name| matches!(name.as_str(), "schemas" | "models" | "pb"));
                    let data = representation
                        || (!item.fields.is_empty()
                            && !item.fields.iter().any(|field| runtime_type(&field.ty)));
                    let model = Model {
                        fields,
                        data,
                    };
                    self.models.insert([scope, &[identifier(&item.ident)]].concat(), model);
                }
                | Item::Enum(item) => {
                    for variant in &item.variants {
                        let fields = variant
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(index, field)| {
                                (
                                    field
                                        .ident
                                        .as_ref()
                                        .map(identifier)
                                        .unwrap_or_else(|| index.to_string()),
                                    field.ty.clone(),
                                )
                            })
                            .collect();
                        self.variants.insert(
                            [scope, &[identifier(&item.ident), identifier(&variant.ident)]]
                                .concat(),
                            fields,
                        );
                    }
                    let model = Model {
                        fields: BTreeMap::new(),
                        data: true,
                    };
                    self.models.insert([scope, &[identifier(&item.ident)]].concat(), model);
                }
                | Item::Fn(item) => {
                    if let ReturnType::Type(_, output) = &item.sig.output {
                        self.functions.insert(
                            [scope, &[identifier(&item.sig.ident)]].concat(),
                            (scope.to_vec(), (**output).clone()),
                        );
                    }
                }
                | Item::Impl(item) => {
                    if let Some(owner) = type_path(&item.self_ty) {
                        let owner = self.resolve(scope, &path_names(owner));
                        for member in &item.items {
                            if let syn::ImplItem::Fn(method) = member
                                && let ReturnType::Type(_, output) = &method.sig.output
                            {
                                let output = substitute_type(
                                    output,
                                    &BTreeMap::from([("Self".into(), (*item.self_ty).clone())]),
                                );
                                self.functions.insert(
                                    [owner.as_slice(), &[identifier(&method.sig.ident)]].concat(),
                                    (scope.to_vec(), output),
                                );
                            }
                        }
                    }
                }
                | _ => {}
            }
        }
    }

    fn import(
        &mut self,
        scope: &[String],
        prefix: &[String],
        tree: &UseTree,
    ) {
        match tree {
            | UseTree::Path(path) => {
                self.import(
                    scope,
                    &[prefix, &[identifier(&path.ident)]].concat(),
                    &path.tree,
                )
            }
            | UseTree::Group(group) => {
                for tree in &group.items {
                    self.import(scope, prefix, tree);
                }
            }
            | UseTree::Name(name) => {
                let name = identifier(&name.ident);
                let target = if name == "self" {
                    prefix.to_vec()
                }
                else {
                    [prefix, std::slice::from_ref(&name)].concat()
                };
                let alias = target.last().cloned().unwrap_or(name);
                self.aliases.insert(
                    [scope, &[alias]].concat(),
                    (scope.to_vec(), qualify(scope, &target)),
                );
            }
            | UseTree::Rename(rename) => {
                let target = [prefix, &[identifier(&rename.ident)]].concat();
                self.aliases.insert(
                    [scope, &[identifier(&rename.rename)]].concat(),
                    (scope.to_vec(), qualify(scope, &target)),
                );
            }
            | UseTree::Glob(_) => {
                self.globs.entry(scope.to_vec()).or_default().push(qualify(scope, prefix));
            }
        }
    }

    fn resolve(
        &self,
        scope: &[String],
        names: &[String],
    ) -> Vec<String> {
        self.resolve_inner(scope, names, true, &mut BTreeSet::new())
    }

    fn resolve_inner(
        &self,
        scope: &[String],
        names: &[String],
        allow_globs: bool,
        active: &mut BTreeSet<Vec<String>>,
    ) -> Vec<String> {
        if names.is_empty() {
            return names.to_vec();
        }
        let qualified = qualify(scope, names);
        if qualified != names {
            return self.resolve_inner(&[], &qualified, allow_globs, active);
        }
        for length in (1..=names.len()).rev() {
            if let Some(target) =
                self.expand_alias(&names[..length], &names[length..], allow_globs, active)
            {
                return target;
            }
        }
        let local = [scope, names].concat();
        if self.models.contains_key(&local) {
            return local;
        }
        let alias = [scope, &names[..1]].concat();
        let shadows_external = names.len() > 1
            && self.aliases.get(&alias).is_some_and(|(_, target)| {
                target.first() == names.first()
                    && !self.models.keys().any(|name| name.starts_with(target))
            });
        if !scope.is_empty()
            && !shadows_external
            && let Some(target) = self.expand_alias(&alias, &names[1..], allow_globs, active)
        {
            return target;
        }
        if self
            .models
            .keys()
            .chain(self.functions.keys())
            .chain(self.variants.keys())
            .chain(self.types.keys())
            .any(|name| name.starts_with(&local))
        {
            return local;
        }
        if let Some(globs) = self.globs.get(scope).filter(|_| allow_globs) {
            for glob in globs {
                let candidate =
                    self.resolve_inner(&[], &[glob.as_slice(), names].concat(), false, active);
                if self.models.contains_key(&candidate)
                    || self.functions.contains_key(&candidate)
                    || self.variants.contains_key(&candidate)
                    || self.types.contains_key(&candidate)
                    || conversion_trait(&candidate)
                {
                    return candidate;
                }
            }
        }
        names.to_vec()
    }

    fn expand_alias(
        &self,
        alias: &[String],
        suffix: &[String],
        allow_globs: bool,
        active: &mut BTreeSet<Vec<String>>,
    ) -> Option<Vec<String>> {
        let (origin, target) = self.aliases.get(alias)?;
        if !active.insert(alias.to_vec()) {
            return None;
        }
        let target = self.resolve_inner(origin, target, allow_globs, active);
        let result = self.resolve_inner(&[], &[target.as_slice(), suffix].concat(), false, active);
        active.remove(alias);
        Some(result)
    }

    fn classify_runtime_fields(&mut self) {
        loop {
            let runtime: Vec<_> = self
                .models
                .iter()
                .filter_map(|(name, model)| {
                    if !model.data {
                        return None;
                    }
                    let scope = &name[..name.len() - 1];
                    model
                        .fields
                        .values()
                        .any(|ty| self.runtime_field(scope, ty))
                        .then_some(name.clone())
                })
                .collect();
            if runtime.is_empty() {
                break;
            }
            for name in runtime {
                self.models.get_mut(&name).unwrap().data = false;
            }
        }
    }

    fn runtime_field(
        &self,
        scope: &[String],
        ty: &Type,
    ) -> bool {
        let expanded = self.declared_type(scope, ty);
        let ty = &expanded;
        if runtime_type(ty)
            || type_path(ty)
                .and_then(|path| self.model(scope, path))
                .is_some_and(|(_, model)| !model.data)
        {
            return true;
        }
        type_children(ty).into_iter().any(|child| self.runtime_field(scope, child))
    }

    fn declared_type(
        &self,
        scope: &[String],
        ty: &Type,
    ) -> Type {
        self.expand_type(scope, ty, &mut BTreeSet::new())
    }

    fn type_alias(
        &self,
        scope: &[String],
        names: &[String],
        active: &mut BTreeSet<Vec<String>>,
    ) -> Option<(Vec<String>, Vec<String>, syn::ItemType)> {
        let names = qualify(scope, names);
        for candidate in [names.clone(), [scope, names.as_slice()].concat()] {
            if let Some((origin, item)) = self.types.get(&candidate) {
                return Some((candidate, origin.clone(), item.clone()));
            }
            for length in (1..=candidate.len()).rev() {
                let key = &candidate[..length];
                if let Some((origin, target)) = self.aliases.get(key)
                    && active.insert(key.to_vec())
                {
                    let found = self.type_alias(
                        origin,
                        &[target.as_slice(), &candidate[length..]].concat(),
                        active,
                    );
                    active.remove(key);
                    if found.is_some() {
                        return found;
                    }
                }
            }
        }
        if let Some(globs) = self.globs.get(scope) {
            for glob in globs {
                let candidate = [glob.as_slice(), names.as_slice()].concat();
                if let Some((origin, item)) = self.types.get(&candidate) {
                    return Some((candidate, origin.clone(), item.clone()));
                }
            }
        }
        None
    }

    fn expand_type(
        &self,
        scope: &[String],
        ty: &Type,
        active: &mut BTreeSet<Vec<String>>,
    ) -> Type {
        if let Type::Path(path) = ty
            && let Some((key, origin, alias)) =
                self.type_alias(scope, &path_names(&path.path), &mut BTreeSet::new())
            && active.insert(key.clone())
        {
            let arguments = path.path.segments.last().map(|segment| &segment.arguments);
            let arguments: Vec<_> = match arguments {
                | Some(PathArguments::AngleBracketed(arguments)) => {
                    arguments
                        .args
                        .iter()
                        .filter_map(|argument| {
                            match argument {
                                | GenericArgument::Type(ty) => {
                                    Some(self.expand_type(scope, ty, active))
                                }
                                | _ => None,
                            }
                        })
                        .collect()
                }
                | _ => Vec::new(),
            };
            let parameters = alias
                .generics
                .type_params()
                .map(|parameter| identifier(&parameter.ident))
                .zip(arguments)
                .collect();
            let value = substitute_type(&alias.ty, &parameters);
            let result = self.expand_type(&origin, &value, active);
            active.remove(&key);
            return result;
        }
        let mut ty = ty.clone();
        transform_type_children(&mut ty, &mut |child| self.expand_type(scope, child, active));
        match &mut ty {
            | Type::Path(path) => {
                let names = self.resolve(scope, &path_names(&path.path));
                if let Ok(mut resolved) = syn::parse_str::<syn::Path>(&names.join("::")) {
                    if let (Some(old), Some(new)) =
                        (path.path.segments.last(), resolved.segments.last_mut())
                    {
                        new.arguments = old.arguments.clone();
                    }
                    path.path = resolved;
                }
            }
            | _ => {}
        }
        ty
    }

    fn function_type(
        &self,
        scope: &[String],
        path: &[String],
    ) -> Option<Type> {
        let name = self.resolve(scope, path);
        let (origin, output) = self.functions.get(&name)?;
        Some(self.declared_type(origin, output))
    }

    fn model(
        &self,
        scope: &[String],
        path: &syn::Path,
    ) -> Option<(Vec<String>, &Model)> {
        let mut names = self.resolve(scope, &path_names(path));
        loop {
            if let Some(model) = self.models.get(&names) {
                return Some((names, model));
            }
            if names.len() <= 1 {
                return None;
            }
            names.pop();
        }
    }
}

fn transform_type_children(
    ty: &mut Type,
    transform: &mut impl FnMut(&Type) -> Type,
) {
    match ty {
        | Type::Path(path) => {
            for segment in &mut path.path.segments {
                if let PathArguments::AngleBracketed(arguments) = &mut segment.arguments {
                    for argument in &mut arguments.args {
                        if let GenericArgument::Type(ty) = argument {
                            *ty = transform(ty);
                        }
                    }
                }
            }
        }
        | Type::Reference(value) => *value.elem = transform(&value.elem),
        | Type::Array(value) => *value.elem = transform(&value.elem),
        | Type::Slice(value) => *value.elem = transform(&value.elem),
        | Type::Paren(value) => *value.elem = transform(&value.elem),
        | Type::Tuple(value) => {
            for ty in &mut value.elems {
                *ty = transform(ty);
            }
        }
        | _ => {}
    }
}

fn substitute_type(
    ty: &Type,
    parameters: &BTreeMap<String, Type>,
) -> Type {
    if let Type::Path(path) = ty
        && path.path.segments.len() == 1
        && let Some(value) = parameters.get(&identifier(&path.path.segments[0].ident))
    {
        return value.clone();
    }
    let mut ty = ty.clone();
    transform_type_children(&mut ty, &mut |child| substitute_type(child, parameters));
    ty
}

fn runtime_type(ty: &Type) -> bool {
    (match ty {
        | Type::TraitObject(_) | Type::ImplTrait(_) | Type::BareFn(_) => true,
        | Type::Path(path) => {
            path.path.segments.iter().any(|segment| {
                matches!(
                    identifier(&segment.ident).as_str(),
                    "Arc" | "Rc" | "Mutex" | "RwLock" | "Client" | "Router"
                )
            })
        }
        | _ => false,
    }) || type_children(ty).into_iter().any(runtime_type)
}

fn type_children(ty: &Type) -> Vec<&Type> {
    match ty {
        | Type::Path(path) => {
            path.path
                .segments
                .iter()
                .flat_map(|segment| {
                    match &segment.arguments {
                        | PathArguments::AngleBracketed(arguments) => {
                            arguments
                                .args
                                .iter()
                                .filter_map(|argument| {
                                    match argument {
                                        | GenericArgument::Type(ty) => Some(ty),
                                        | _ => None,
                                    }
                                })
                                .collect()
                        }
                        | _ => Vec::new(),
                    }
                })
                .collect()
        }
        | Type::Reference(ty) => vec![&ty.elem],
        | Type::Array(ty) => vec![&ty.elem],
        | Type::Slice(ty) => vec![&ty.elem],
        | Type::Paren(ty) => vec![&ty.elem],
        | Type::Tuple(ty) => ty.elems.iter().collect(),
        | _ => Vec::new(),
    }
}

fn conversion_trait(path: &[String]) -> bool {
    path.last().is_some_and(|name| {
        matches!(
            name.as_str(),
            "From"
                | "TryFrom"
                | "Into"
                | "TryInto"
                | "FromStr"
                | "AsRef"
                | "AsMut"
                | "Borrow"
                | "BorrowMut"
                | "ToOwned"
                | "ToString"
                | "IntoResponse"
                | "FromIterator"
        )
    })
}

#[derive(Clone, Default)]
struct Binding {
    ty: Option<Type>,
    projections: BTreeSet<Vec<String>>,
    pure: bool,
    direct: bool,
    collection: bool,
    tuple: Option<Vec<Binding>>,
}

#[derive(Clone)]
struct Inspector<'a> {
    catalog: Cow<'a, Catalog>,
    scope: Vec<String>,
    self_type: Option<syn::Path>,
    variables: BTreeMap<String, Type>,
    projections: BTreeMap<String, BTreeSet<Vec<String>>>,
    bindings: BTreeMap<String, Binding>,
    violations: Vec<Violation>,
}

impl<'a> Inspector<'a> {
    fn new(
        catalog: &'a Catalog,
        scope: Vec<String>,
    ) -> Self {
        Self {
            catalog: Cow::Borrowed(catalog),
            scope,
            self_type: None,
            variables: BTreeMap::new(),
            projections: BTreeMap::new(),
            bindings: BTreeMap::new(),
            violations: Vec::new(),
        }
    }

    fn report(
        &mut self,
        span: Span,
        reason: &'static str,
    ) {
        let start = span.start();
        if !self
            .violations
            .iter()
            .any(|violation| violation.line == start.line && violation.column == start.column + 1)
        {
            let violation = Violation {
                line: start.line,
                column: start.column + 1,
                reason,
            };
            self.violations.push(violation);
        }
    }

    fn model(
        &self,
        path: &syn::Path,
    ) -> Option<(Vec<String>, &Model)> {
        if path.segments.first().is_some_and(|segment| segment.ident == "Self") {
            self.self_type.as_ref().and_then(|path| self.catalog.model(&self.scope, path))
        }
        else {
            self.catalog.model(&self.scope, path)
        }
    }

    fn expression_type(
        &self,
        expression: &Expr,
    ) -> Option<Type> {
        match expression {
            | Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
                self.variables.get(&identifier(&path.path.segments[0].ident)).cloned()
            }
            | Expr::Field(field) => {
                let base = self.expression_type(&field.base)?;
                if let (Type::Tuple(tuple), Member::Unnamed(index)) =
                    (transparent_type(&base), &field.member)
                {
                    return tuple.elems.iter().nth(index.index as usize).cloned();
                }
                let path = type_path(&base)?;
                let (owner, model) = self.model(path)?;
                let name = match &field.member {
                    | Member::Named(name) => identifier(name),
                    | Member::Unnamed(index) => index.index.to_string(),
                };
                model
                    .fields
                    .get(&name)
                    .map(|ty| self.catalog.declared_type(&owner[..owner.len() - 1], ty))
            }
            | Expr::Reference(value) => self.expression_type(&value.expr),
            | Expr::Paren(value) => self.expression_type(&value.expr),
            | Expr::Group(value) => self.expression_type(&value.expr),
            | Expr::Return(value) => {
                value.expr.as_ref().and_then(|value| self.expression_type(value))
            }
            | Expr::Try(value) => {
                self.expression_type(&value.expr).and_then(|ty| success_type(&ty))
            }
            | Expr::Await(value) => self.expression_type(&value.base),
            | Expr::Unary(value) => self.expression_type(&value.expr),
            | Expr::Block(value) => {
                self.block_value(&value.block, |scope, expression| {
                    scope.expression_type(expression)
                })
                .flatten()
            }
            | Expr::Call(call) => {
                let Expr::Path(path) = &*call.func
                else {
                    return None;
                };
                if path.path.is_ident("Some") || path.path.is_ident("Ok") {
                    let ty = self.expression_type(call.args.first()?)?;
                    return Some(
                        if path.path.is_ident("Some") {
                            syn::parse_quote!(Option<#ty>)
                        }
                        else {
                            syn::parse_quote!(Result<#ty, ()>)
                        },
                    );
                }
                if let Some((owner, _)) = self.variant(&path.path) {
                    return syn::parse_str::<Type>(&owner.join("::")).ok();
                }
                self.catalog.function_type(&self.scope, &path_names(&path.path))
            }
            | Expr::MethodCall(call)
                if matches!(
                    identifier(&call.method).as_str(),
                    "clone"
                        | "to_owned"
                        | "as_ref"
                        | "as_mut"
                        | "iter"
                        | "iter_mut"
                        | "into_iter"
                        | "filter"
                        | "inspect"
                        | "take"
                        | "skip"
                        | "rev"
                        | "cloned"
                        | "copied"
                        | "by_ref"
                        | "step_by"
                ) =>
            {
                self.expression_type(&call.receiver)
            }
            | Expr::MethodCall(call) if call.method == "enumerate" => {
                let element = element_type(&self.expression_type(&call.receiver)?)?;
                Some(syn::parse_quote!(Vec<(usize, #element)>))
            }
            | Expr::MethodCall(call)
                if matches!(identifier(&call.method).as_str(), "unwrap" | "expect") =>
            {
                self.expression_type(&call.receiver).and_then(|ty| success_type(&ty))
            }
            | Expr::MethodCall(call) => {
                let owner = self.expression_type(&call.receiver)?;
                let method = identifier(&call.method);
                if type_path(&owner).and_then(|path| path.segments.last()).is_some_and(|segment| {
                    matches!(identifier(&segment.ident).as_str(), "BTreeMap" | "HashMap")
                }) && matches!(
                    method.as_str(),
                    "values" | "values_mut" | "into_values" | "keys" | "into_keys"
                ) {
                    let index = usize::from(matches!(
                        method.as_str(),
                        "values" | "values_mut" | "into_values"
                    ));
                    let arguments = type_arguments(&owner);
                    let element = arguments.get(index)?;
                    return Some(syn::parse_quote!(Vec<#element>));
                }
                let (owner, _) = self.model(type_path(&owner)?)?;
                self.catalog.function_type(&[], &[owner.as_slice(), &[method]].concat())
            }
            | Expr::Struct(value) => {
                let path = syn::TypePath {
                    qself: None,
                    path: value.path.clone(),
                };
                Some(Type::Path(path))
            }
            | Expr::Tuple(tuple) => {
                let elements = tuple
                    .elems
                    .iter()
                    .map(|value| self.expression_type(value))
                    .collect::<Option<Punctuated<Type, Token![,]>>>()?;
                let tuple = syn::TypeTuple {
                    paren_token: tuple.paren_token,
                    elems: elements,
                };
                Some(Type::Tuple(tuple))
            }
            | _ => None,
        }
    }

    fn variant(
        &self,
        path: &syn::Path,
    ) -> Option<(Vec<String>, BTreeMap<String, Type>)> {
        let names = path_names(path);
        let names = if names.first().is_some_and(|name| name == "Self") {
            let owner = self.self_type.as_ref()?;
            [self.catalog.resolve(&self.scope, &path_names(owner)), names[1..].to_vec()].concat()
        }
        else {
            self.catalog.resolve(&self.scope, &names)
        };
        self.catalog
            .variants
            .get(&names)
            .map(|fields| (names[..names.len() - 1].to_vec(), fields.clone()))
    }

    fn is_projection(
        &self,
        expression: &Expr,
        target: &[String],
    ) -> bool {
        self.projected_models(expression).iter().any(|name| name != target)
    }

    fn projected_models(
        &self,
        expression: &Expr,
    ) -> BTreeSet<Vec<String>> {
        match expression {
            | Expr::Field(field) => {
                let mut sources = self.projected_models(&field.base);
                if let Some((name, model)) = self
                    .expression_type(&field.base)
                    .as_ref()
                    .and_then(type_path)
                    .and_then(|path| self.model(path))
                    && model.data
                {
                    sources.insert(name);
                }
                sources
            }
            | Expr::Path(path) if path.path.segments.len() == 1 => {
                self.projections
                    .get(&identifier(&path.path.segments[0].ident))
                    .cloned()
                    .unwrap_or_default()
            }
            | Expr::MethodCall(call) => self.projected_models(&call.receiver),
            | Expr::Reference(value) => self.projected_models(&value.expr),
            | Expr::Try(value) => self.projected_models(&value.expr),
            | Expr::Await(value) => self.projected_models(&value.base),
            | Expr::Paren(value) => self.projected_models(&value.expr),
            | Expr::Group(value) => self.projected_models(&value.expr),
            | Expr::Return(value) => {
                value.expr.as_ref().map(|value| self.projected_models(value)).unwrap_or_default()
            }
            | Expr::Block(value) => {
                self.block_value(&value.block, |scope, expression| {
                    scope.projected_models(expression)
                })
                .unwrap_or_default()
            }
            | Expr::Call(call) => {
                call.args.iter().flat_map(|argument| self.projected_models(argument)).collect()
            }
            | Expr::Tuple(tuple) => {
                tuple.elems.iter().flat_map(|value| self.projected_models(value)).collect()
            }
            | Expr::Array(array) => {
                array.elems.iter().flat_map(|value| self.projected_models(value)).collect()
            }
            | Expr::Repeat(value) => self.projected_models(&value.expr),
            | Expr::Macro(value) => {
                vector_literal(&value.mac, self)
                    .map(|value| self.projected_models(&value))
                    .unwrap_or_default()
            }
            | _ => BTreeSet::new(),
        }
    }

    fn bind(
        &mut self,
        pattern: &Pat,
        ty: Type,
    ) {
        let ty = self.catalog.declared_type(&self.scope, &ty);
        match pattern {
            | Pat::Ident(name) => {
                let name = identifier(&name.ident);
                self.variables.insert(name.clone(), ty.clone());
                self.projections.remove(&name);
                self.bindings.remove(&name);
                if let Pat::Ident(pattern) = pattern
                    && let Some((_, subpattern)) = &pattern.subpat
                {
                    self.bind(subpattern, ty);
                }
            }
            | Pat::Type(pattern) => self.bind(&pattern.pat, (*pattern.ty).clone()),
            | Pat::Reference(pattern) => self.bind(&pattern.pat, ty),
            | Pat::Tuple(pattern) => {
                if let Type::Tuple(ty) = transparent_type(&ty) {
                    for (pattern, index) in tuple_patterns(&pattern.elems, ty.elems.len()) {
                        self.bind(pattern, ty.elems[index].clone());
                    }
                }
            }
            | Pat::Struct(pattern) => {
                let model = self.variant(&pattern.path).or_else(|| {
                    type_path(&ty)
                        .and_then(|path| self.model(path))
                        .map(|(owner, model)| (owner, model.fields.clone()))
                });
                if let Some((owner, fields)) = model {
                    for field in &pattern.fields {
                        let name = match &field.member {
                            | Member::Named(name) => identifier(name),
                            | Member::Unnamed(index) => index.index.to_string(),
                        };
                        if let Some(ty) = fields.get(&name) {
                            self.bind(
                                &field.pat,
                                self.catalog.declared_type(&owner[..owner.len() - 1], ty),
                            );
                            self.bind_projections(&field.pat, BTreeSet::from([owner.clone()]));
                        }
                    }
                }
            }
            | Pat::TupleStruct(pattern) => {
                if matches!(
                    pattern
                        .path
                        .segments
                        .last()
                        .map(|segment| identifier(&segment.ident))
                        .as_deref(),
                    Some("Some" | "Ok" | "Err")
                ) {
                    let index = usize::from(
                        pattern.path.segments.last().is_some_and(|segment| segment.ident == "Err"),
                    );
                    if let Some(ty) = type_arguments(&ty).get(index) {
                        for (pattern, _) in tuple_patterns(&pattern.elems, 1) {
                            self.bind(pattern, (*ty).clone());
                        }
                    }
                }
                else {
                    let model = self.variant(&pattern.path).or_else(|| {
                        type_path(&ty)
                            .and_then(|path| self.model(path))
                            .map(|(owner, model)| (owner, model.fields.clone()))
                    });
                    if let Some((owner, fields)) = model {
                        for (pattern, index) in tuple_patterns(&pattern.elems, fields.len()) {
                            if let Some(ty) = fields.get(&index.to_string()) {
                                self.bind(
                                    pattern,
                                    self.catalog.declared_type(&owner[..owner.len() - 1], ty),
                                );
                                self.bind_projections(pattern, BTreeSet::from([owner.clone()]));
                            }
                        }
                    }
                }
            }
            | Pat::Slice(pattern) => {
                if let Some(element) = element_type(&ty) {
                    for pattern in &pattern.elems {
                        self.bind(pattern, element.clone());
                    }
                }
            }
            | Pat::Or(pattern) => {
                if let Some(pattern) = pattern.cases.first() {
                    self.bind(pattern, ty);
                }
            }
            | Pat::Paren(pattern) => self.bind(&pattern.pat, ty),
            | _ => {}
        }
    }

    fn bind_projections(
        &mut self,
        pattern: &Pat,
        sources: BTreeSet<Vec<String>>,
    ) {
        match pattern {
            | Pat::Ident(name) => {
                self.projections.insert(identifier(&name.ident), sources);
            }
            | Pat::Type(pattern) => self.bind_projections(&pattern.pat, sources),
            | Pat::Reference(pattern) => self.bind_projections(&pattern.pat, sources),
            | Pat::Tuple(pattern) => {
                for pattern in &pattern.elems {
                    self.bind_projections(pattern, sources.clone());
                }
            }
            | Pat::TupleStruct(pattern) => {
                for pattern in &pattern.elems {
                    self.bind_projections(pattern, sources.clone());
                }
            }
            | Pat::Struct(pattern) => {
                for field in &pattern.fields {
                    self.bind_projections(&field.pat, sources.clone());
                }
            }
            | Pat::Paren(pattern) => self.bind_projections(&pattern.pat, sources),
            | _ => {}
        }
    }

    fn bind_local(
        &mut self,
        local: &syn::Local,
    ) {
        let value = local.init.as_ref().map(|init| self.binding(&init.expr)).unwrap_or_default();
        self.bind_value(&local.pat, value);
    }

    fn binding(
        &self,
        expression: &Expr,
    ) -> Binding {
        match expression {
            | Expr::Path(path) if path.path.segments.len() == 1 => {
                if let Some(value) = self.bindings.get(&identifier(&path.path.segments[0].ident)) {
                    return value.clone();
                }
            }
            | Expr::Paren(value) => return self.binding(&value.expr),
            | Expr::Group(value) => return self.binding(&value.expr),
            | Expr::Return(value) => {
                return value.expr.as_ref().map(|value| self.binding(value)).unwrap_or_default();
            }
            | Expr::Block(value) => {
                return self
                    .block_value(&value.block, |scope, expression| scope.binding(expression))
                    .unwrap_or_default();
            }
            | _ => {}
        }
        Binding {
            ty: self.expression_type(expression),
            projections: self.projected_models(expression),
            pure: pure_projection_part(expression, self),
            direct: direct_projection(expression, self),
            collection: self.scalar_collection_projection(expression),
            tuple: if let Expr::Tuple(tuple) = expression {
                Some(tuple.elems.iter().map(|value| self.binding(value)).collect())
            }
            else {
                None
            },
        }
    }

    fn bind_value(
        &mut self,
        pattern: &Pat,
        mut value: Binding,
    ) {
        match pattern {
            | Pat::Type(pattern) => {
                value.ty = Some(self.catalog.declared_type(&self.scope, &pattern.ty));
                self.bind_value(&pattern.pat, value);
                return;
            }
            | Pat::Reference(pattern) => {
                self.bind_value(&pattern.pat, value);
                return;
            }
            | Pat::Paren(pattern) => {
                self.bind_value(&pattern.pat, value);
                return;
            }
            | Pat::Tuple(pattern) if value.tuple.is_some() => {
                let elements = value.tuple.as_ref().unwrap();
                for (pattern, index) in tuple_patterns(&pattern.elems, elements.len()) {
                    let mut element = elements[index].clone();
                    if let Some(Type::Tuple(ty)) = value.ty.as_ref().map(transparent_type) {
                        element.ty = ty.elems.get(index).cloned();
                    }
                    self.bind_value(pattern, element);
                }
                return;
            }
            | _ => {}
        }
        self.bind(
            pattern,
            value.ty.clone().unwrap_or_else(|| syn::parse_quote!(_)),
        );
        if !value.projections.is_empty() || matches!(pattern, Pat::Ident(_)) {
            self.bind_projections(pattern, value.projections.clone());
        }
        if let Pat::Ident(name) = pattern {
            if let Some((_, subpattern)) = &name.subpat {
                self.bind_value(subpattern, value.clone());
            }
            self.bindings.insert(identifier(&name.ident), value);
        }
    }

    fn block_value<T>(
        &self,
        block: &syn::Block,
        evaluate: impl FnOnce(&Self, &Expr) -> T,
    ) -> Option<T> {
        let expression = tail_expression(block)?;
        let mut scope = self.clone();
        scope.block_catalog(block);
        for statement in &block.stmts[..block.stmts.len() - 1] {
            match statement {
                | syn::Stmt::Local(local) => {
                    let init = local.init.as_ref()?;
                    let value = scope.binding(&init.expr);
                    if init.diverge.is_some() || !value.pure {
                        return None;
                    }
                    scope.bind_value(&local.pat, value);
                }
                | syn::Stmt::Item(Item::Use(_) | Item::Type(_)) => {}
                | _ => return None,
            }
        }
        pure_projection_part(expression, &scope).then(|| evaluate(&scope, expression))
    }

    fn block_catalog(
        &mut self,
        block: &syn::Block,
    ) -> Cow<'a, Catalog> {
        let previous = self.catalog.clone();
        let items: Vec<_> = block
            .stmts
            .iter()
            .filter_map(|statement| {
                match statement {
                    | syn::Stmt::Item(item) => Some(item.clone()),
                    | _ => None,
                }
            })
            .collect();
        if !items.is_empty() {
            self.catalog.to_mut().collect(&self.scope, &items);
            self.catalog.to_mut().classify_runtime_fields();
        }
        previous
    }

    fn scoped(
        &mut self,
        visit: impl FnOnce(&mut Self),
    ) {
        let variables = self.variables.clone();
        let projections = self.projections.clone();
        let bindings = self.bindings.clone();
        visit(self);
        self.variables = variables;
        self.projections = projections;
        self.bindings = bindings;
    }

    fn condition(
        &mut self,
        condition: &Expr,
    ) {
        match condition {
            | Expr::Let(value) => {
                self.visit_expr(&value.expr);
                self.bind_value(&value.pat, self.binding(&value.expr));
            }
            | Expr::Binary(value) if matches!(value.op, syn::BinOp::And(_)) => {
                self.condition(&value.left);
                self.condition(&value.right);
            }
            | Expr::Paren(value) => self.condition(&value.expr),
            | value => self.visit_expr(value),
        }
    }

    fn enum_translation(
        &self,
        expression: &syn::ExprMatch,
    ) -> bool {
        let Some((input, _)) = self
            .expression_type(&expression.expr)
            .as_ref()
            .and_then(type_path)
            .and_then(|path| self.model(path))
        else {
            return false;
        };
        let variants = self
            .catalog
            .variants
            .keys()
            .filter(|variant| variant.len() == input.len() + 1 && variant.starts_with(&input))
            .count();
        if variants == 0 || expression.arms.len() != variants {
            return false;
        }
        let mut sources = BTreeSet::new();
        let mut outputs = BTreeSet::new();
        let mut target = None;
        for arm in &expression.arms {
            if arm.guard.is_some() {
                return false;
            }
            let pattern = match &arm.pat {
                | Pat::Path(value) => &value.path,
                | Pat::TupleStruct(value) => &value.path,
                | Pat::Struct(value) => &value.path,
                | _ => return false,
            };
            let Some((owner, _)) = self.variant(pattern)
            else {
                return false;
            };
            if owner != input || !sources.insert(path_names(pattern).last().cloned()) {
                return false;
            }
            let body = if let Expr::Block(block) = &*arm.body {
                single_expression(&block.block)
            }
            else {
                Some(&*arm.body)
            };
            let output = match body {
                | Some(Expr::Path(value)) => &value.path,
                | Some(Expr::Call(value))
                    if value.args.iter().all(|value| direct_projection(value, self)) =>
                {
                    let Expr::Path(path) = &*value.func
                    else {
                        return false;
                    };
                    &path.path
                }
                | _ => return false,
            };
            let Some((owner, _)) = self.variant(output)
            else {
                return false;
            };
            if owner == input
                || target.as_ref().is_some_and(|target| target != &owner)
                || !outputs.insert(path_names(output).last().cloned())
            {
                return false;
            }
            target = Some(owner);
        }
        target.is_some()
    }

    fn scalar_collection_projection(
        &self,
        expression: &Expr,
    ) -> bool {
        match expression {
            | Expr::Path(path) if path.path.segments.len() == 1 => {
                self.bindings
                    .get(&identifier(&path.path.segments[0].ident))
                    .is_some_and(|value| value.collection)
            }
            | Expr::Return(value) => {
                value.expr.as_ref().is_some_and(|value| self.scalar_collection_projection(value))
            }
            | Expr::Try(value) => self.scalar_collection_projection(&value.expr),
            | Expr::Paren(value) => self.scalar_collection_projection(&value.expr),
            | Expr::Group(value) => self.scalar_collection_projection(&value.expr),
            | Expr::Reference(value) => self.scalar_collection_projection(&value.expr),
            | Expr::Block(value) => {
                self.block_value(&value.block, |scope, expression| {
                    scope.scalar_collection_projection(expression)
                })
                .unwrap_or(false)
            }
            | Expr::Array(array) => {
                array.elems.iter().all(|value| direct_projection(value, self))
                    && array.elems.iter().any(|value| !self.projected_models(value).is_empty())
            }
            | Expr::Repeat(value) => {
                direct_projection(&value.expr, self)
                    && !self.projected_models(&value.expr).is_empty()
            }
            | Expr::Macro(value) => {
                vector_literal(&value.mac, self)
                    .is_some_and(|value| self.scalar_collection_projection(&value))
            }
            | Expr::Call(call) => {
                let Expr::Path(path) = &*call.func
                else {
                    return false;
                };
                let names = self.catalog.resolve(&self.scope, &path_names(&path.path));
                let wrapper = path.path.is_ident("Ok")
                    || path.path.is_ident("Some")
                    || matches!(names.as_slice(), [.., owner, method] if owner == "Vec" && method == "from");
                wrapper
                    && call
                        .args
                        .first()
                        .is_some_and(|value| self.scalar_collection_projection(value))
            }
            | Expr::MethodCall(call)
                if matches!(
                    identifier(&call.method).as_str(),
                    "to_vec" | "into_iter" | "iter" | "cloned" | "copied"
                ) =>
            {
                self.scalar_collection_projection(&call.receiver)
            }
            | Expr::MethodCall(collect) if collect.method == "collect" => {
                let Expr::MethodCall(map) = &*collect.receiver
                else {
                    return false;
                };
                if map.method != "map" {
                    return self.scalar_collection_projection(&collect.receiver);
                }
                let Some(Expr::Closure(closure)) = map.args.first()
                else {
                    return false;
                };
                let Some(element) =
                    self.expression_type(&map.receiver).and_then(|ty| element_type(&ty))
                else {
                    return false;
                };
                if !type_path(&element)
                    .and_then(|path| self.model(path))
                    .is_some_and(|(_, model)| model.data)
                {
                    return false;
                }
                let mut scope = self.clone();
                if let Some(pattern) = closure.inputs.first() {
                    scope.bind(pattern, element);
                }
                direct_projection(&closure.body, &scope)
                    && !scope.projected_models(&closure.body).is_empty()
            }
            | _ => false,
        }
    }

    fn returns_model(
        &self,
        ty: &Type,
    ) -> bool {
        if type_path(ty).and_then(|path| self.model(path)).is_some_and(|(_, model)| model.data) {
            return true;
        }
        match ty {
            | Type::Path(ty) => ty.path.segments.iter().any(|segment| {
                let PathArguments::AngleBracketed(arguments) = &segment.arguments else { return false };
                if segment.ident == "Result" {
                    return arguments.args.iter().find_map(|argument| match argument {
                        | GenericArgument::Type(ty) => Some(self.returns_model(ty)),
                        | _ => None,
                    }).unwrap_or(false);
                }
                arguments.args.iter().any(|argument| matches!(argument, GenericArgument::Type(ty) if self.returns_model(ty)))
            }),
            | Type::Tuple(ty) => ty.elems.iter().any(|ty| self.returns_model(ty)),
            | Type::Reference(ty) => self.returns_model(&ty.elem),
            | _ => false,
        }
    }

    fn inspect_function(
        &mut self,
        signature: &syn::Signature,
        block: &syn::Block,
    ) {
        let previous = std::mem::take(&mut self.variables);
        let previous_projections = std::mem::take(&mut self.projections);
        let previous_bindings = std::mem::take(&mut self.bindings);
        let previous_catalog = self.block_catalog(block);
        for input in &signature.inputs {
            match input {
                | FnArg::Typed(input) => self.bind(&input.pat, (*input.ty).clone()),
                | FnArg::Receiver(_) => {
                    if let Some(path) = &self.self_type {
                        let path = syn::TypePath {
                            qself: None,
                            path: path.clone(),
                        };
                        self.variables.insert("self".into(), Type::Path(path));
                    }
                }
            }
        }
        let name = identifier(&signature.ident);
        let returns_model =
            matches!(&signature.output, ReturnType::Type(_, ty) if self.returns_model(ty));
        let receives_model = signature.inputs.iter().any(|input| {
            matches!(input, FnArg::Typed(input) if type_path(&input.ty)
                .and_then(|path| self.model(path)).is_some_and(|(_, model)| model.data))
        });
        if name.starts_with("from_") && receives_model && returns_model && !delegates(block) {
            self.report(
                signature.ident.span(),
                "conversion helper must be implemented under mappers/",
            );
        }
        else if let Some(expression) = single_expression(block) {
            let output = match &signature.output {
                | ReturnType::Type(_, ty) => type_path(ty),
                | ReturnType::Default => None,
            };
            let output = output.and_then(|path| self.model(path));
            let output_model = output.as_ref().filter(|(_, model)| model.data);
            let input = expression_root(expression).and_then(|name| self.variables.get(name));
            let input = input.and_then(type_path).and_then(|path| self.model(path));
            if output_model.is_some()
                && input
                    .as_ref()
                    .zip(output_model)
                    .is_some_and(|((input, _), (output, _))| input != output)
                && !delegates(block)
            {
                self.report(
                    signature.ident.span(),
                    "model projection must be implemented under mappers/",
                );
            }
        }
        if let Some(Expr::Match(value)) = single_expression(block) {
            let output = match &signature.output {
                | ReturnType::Type(_, ty) => type_path(ty),
                | ReturnType::Default => None,
            };
            let output = output
                .map(|path| self.catalog.resolve(&self.scope, &path_names(path)))
                .unwrap_or_default();
            let string_conversion = matches!(name.as_str(), "parse" | "as_str")
                && value.arms.iter().all(|arm| structural_value(&arm.body));
            let transport_error = output.iter().any(|name| name == "tonic")
                && output.last().is_some_and(|name| name == "Status");
            if string_conversion || transport_error {
                self.report(
                    signature.ident.span(),
                    "representation match must be implemented under mappers/",
                );
            }
        }
        let analysis = self.clone();
        visit::visit_block(self, block);
        let output = match &signature.output {
            | ReturnType::Type(_, ty) => Some(self.catalog.declared_type(&self.scope, ty)),
            | ReturnType::Default => None,
        };
        let projection = analysis.block_value(block, |scope, value| {
            (
                output.as_ref().is_some_and(sequence_type)
                    && scope.scalar_collection_projection(value),
                output.as_ref().is_some_and(tuple_type)
                    && direct_projection(value, scope)
                    && !scope.projected_models(value).is_empty(),
            )
        });
        if projection.is_some_and(|(collection, _)| collection) {
            self.report(
                signature.ident.span(),
                "collection projection must be implemented under mappers/",
            );
        }
        if projection.is_some_and(|(_, tuple)| tuple) {
            self.report(
                signature.ident.span(),
                "tuple projection must be implemented under mappers/",
            );
        }
        self.variables = previous;
        self.projections = previous_projections;
        self.bindings = previous_bindings;
        self.catalog = previous_catalog;
    }

    fn inspect_macro_tokens(
        &mut self,
        tokens: TokenStream,
    ) {
        if let Ok(file) = syn::parse2::<syn::File>(tokens.clone()) {
            let previous = self.catalog.clone();
            self.catalog.to_mut().collect(&self.scope, &file.items);
            for item in &file.items {
                self.visit_item(item);
            }
            self.catalog = previous;
            if !file.items.is_empty() {
                return;
            }
        }
        let array = TokenStream::from(TokenTree::Group(Group::new(
            Delimiter::Bracket,
            tokens.clone(),
        )));
        if let Ok(expression) = syn::parse2::<Expr>(array) {
            self.visit_expr(&expression);
            return;
        }
        if let Ok(expression) = syn::parse2::<Expr>(tokens.clone()) {
            self.visit_expr(&expression);
            return;
        }
        for token in tokens {
            if let TokenTree::Group(group) = token {
                self.inspect_macro_tokens(group.stream());
            }
        }
    }
}

fn tuple_patterns(
    patterns: &Punctuated<Pat, Token![,]>,
    length: usize,
) -> impl Iterator<Item = (&Pat, usize)> {
    let rest = patterns.iter().position(|pattern| matches!(pattern, Pat::Rest(_)));
    patterns.iter().enumerate().filter_map(move |(index, pattern)| {
        let index = if rest.is_some_and(|rest| index > rest) {
            length.checked_sub(patterns.len() - index)?
        }
        else {
            index
        };
        (!matches!(pattern, Pat::Rest(_)) && index < length).then_some((pattern, index))
    })
}

fn type_path(ty: &Type) -> Option<&syn::Path> {
    match ty {
        | Type::Path(ty) => Some(&ty.path),
        | Type::Reference(ty) => type_path(&ty.elem),
        | Type::Paren(ty) => type_path(&ty.elem),
        | _ => None,
    }
}

fn element_type(ty: &Type) -> Option<Type> {
    match ty {
        | Type::Reference(ty) => element_type(&ty.elem),
        | Type::Slice(ty) => Some((*ty.elem).clone()),
        | Type::Array(ty) => Some((*ty.elem).clone()),
        | Type::Path(ty) => {
            let segment = ty.path.segments.last()?;
            let PathArguments::AngleBracketed(arguments) = &segment.arguments
            else {
                return None;
            };
            if matches!(identifier(&segment.ident).as_str(), "BTreeMap" | "HashMap") {
                let types: Vec<_> = arguments
                    .args
                    .iter()
                    .filter_map(|argument| {
                        match argument {
                            | GenericArgument::Type(ty) => Some(ty),
                            | _ => None,
                        }
                    })
                    .collect();
                if let [key, value, ..] = types.as_slice() {
                    return Some(syn::parse_quote!((#key, #value)));
                }
            }
            arguments.args.iter().find_map(|argument| {
                match argument {
                    | GenericArgument::Type(ty) => Some(ty.clone()),
                    | _ => None,
                }
            })
        }
        | _ => None,
    }
}

fn transparent_type(ty: &Type) -> &Type {
    match ty {
        | Type::Reference(value) => transparent_type(&value.elem),
        | Type::Paren(value) => transparent_type(&value.elem),
        | _ => ty,
    }
}

fn type_arguments(ty: &Type) -> Vec<&Type> {
    let Some(segment) = type_path(ty).and_then(|path| path.segments.last())
    else {
        return Vec::new();
    };
    let PathArguments::AngleBracketed(arguments) = &segment.arguments
    else {
        return Vec::new();
    };
    arguments
        .args
        .iter()
        .filter_map(|argument| {
            match argument {
                | GenericArgument::Type(ty) => Some(ty),
                | _ => None,
            }
        })
        .collect()
}

fn success_type(ty: &Type) -> Option<Type> {
    let path = type_path(ty)?;
    matches!(
        identifier(&path.segments.last()?.ident).as_str(),
        "Result" | "Option"
    )
    .then(|| element_type(ty))
    .flatten()
}

fn sequence_type(ty: &Type) -> bool {
    if let Some(success) = success_type(ty) {
        return sequence_type(&success);
    }
    matches!(transparent_type(ty), Type::Array(_) | Type::Slice(_))
        || type_path(ty)
            .and_then(|path| path.segments.last())
            .is_some_and(|segment| segment.ident == "Vec")
}

fn tuple_type(ty: &Type) -> bool {
    if let Some(success) = success_type(ty) {
        return tuple_type(&success);
    }
    matches!(transparent_type(ty), Type::Tuple(_))
}

fn tail_expression(block: &syn::Block) -> Option<&Expr> {
    match block.stmts.last()? {
        | syn::Stmt::Expr(expression, None) => Some(expression),
        | syn::Stmt::Expr(Expr::Return(value), _) => value.expr.as_deref(),
        | _ => None,
    }
}

fn direct_projection(
    expression: &Expr,
    inspector: &Inspector<'_>,
) -> bool {
    match expression {
        | Expr::Path(path) if path.path.segments.len() == 1 => {
            inspector
                .bindings
                .get(&identifier(&path.path.segments[0].ident))
                .is_none_or(|value| value.direct)
        }
        | Expr::Path(_) | Expr::Field(_) | Expr::Lit(_) => true,
        | Expr::Reference(value) => direct_projection(&value.expr, inspector),
        | Expr::Paren(value) => direct_projection(&value.expr, inspector),
        | Expr::Group(value) => direct_projection(&value.expr, inspector),
        | Expr::Return(value) => {
            value.expr.as_ref().is_some_and(|value| direct_projection(value, inspector))
        }
        | Expr::Block(value) => {
            inspector
                .block_value(&value.block, |scope, expression| {
                    direct_projection(expression, scope)
                })
                .unwrap_or(false)
        }
        | Expr::Tuple(value) => {
            !value.elems.is_empty()
                && value.elems.iter().all(|value| direct_projection(value, inspector))
        }
        | Expr::Array(value) => value.elems.iter().all(|value| direct_projection(value, inspector)),
        | Expr::Call(call) if matches!(&*call.func, Expr::Path(path) if path.path.is_ident("Some") || path.path.is_ident("Ok")) => {
            call.args.iter().all(|value| direct_projection(value, inspector))
        }
        | Expr::MethodCall(call) => {
            matches!(
                identifier(&call.method).as_str(),
                "clone"
                    | "to_owned"
                    | "to_string"
                    | "as_str"
                    | "to_vec"
                    | "as_ref"
                    | "as_mut"
                    | "as_deref"
                    | "as_deref_mut"
            ) && direct_projection(&call.receiver, inspector)
        }
        | _ => false,
    }
}

fn vector_literal(
    value: &syn::Macro,
    inspector: &Inspector<'_>,
) -> Option<Expr> {
    let names = inspector.catalog.resolve(&inspector.scope, &path_names(&value.path));
    let vector = match names.as_slice() {
        | [name] => name == "vec",
        | [owner, name] => matches!(owner.as_str(), "std" | "alloc") && name == "vec",
        | _ => false,
    };
    if !vector {
        return None;
    }
    let array = TokenStream::from(TokenTree::Group(Group::new(
        Delimiter::Bracket,
        value.tokens.clone(),
    )));
    syn::parse2(array).ok()
}

fn pure_projection_block(
    block: &syn::Block,
    inspector: &Inspector<'_>,
) -> bool {
    inspector.block_value(block, |_, _| ()).is_some()
}

fn pure_projection_part(
    expression: &Expr,
    inspector: &Inspector<'_>,
) -> bool {
    match expression {
        | Expr::Path(path) if path.path.segments.len() == 1 => {
            inspector
                .bindings
                .get(&identifier(&path.path.segments[0].ident))
                .is_none_or(|value| value.pure)
        }
        | Expr::Path(_) | Expr::Lit(_) => true,
        | Expr::Field(value) => pure_projection_part(&value.base, inspector),
        | Expr::Reference(value) => pure_projection_part(&value.expr, inspector),
        | Expr::Paren(value) => pure_projection_part(&value.expr, inspector),
        | Expr::Group(value) => pure_projection_part(&value.expr, inspector),
        | Expr::Try(value) => pure_projection_part(&value.expr, inspector),
        | Expr::Await(value) => pure_projection_part(&value.base, inspector),
        | Expr::Return(value) => {
            value.expr.as_ref().is_some_and(|value| pure_projection_part(value, inspector))
        }
        | Expr::Block(value) => pure_projection_block(&value.block, inspector),
        | Expr::Closure(value) => pure_projection_part(&value.body, inspector),
        | Expr::Tuple(value) => {
            value.elems.iter().all(|value| pure_projection_part(value, inspector))
        }
        | Expr::Array(value) => {
            value.elems.iter().all(|value| pure_projection_part(value, inspector))
        }
        | Expr::Repeat(value) => {
            pure_projection_part(&value.expr, inspector)
                && pure_projection_part(&value.len, inspector)
        }
        | Expr::Macro(value) => {
            vector_literal(&value.mac, inspector)
                .is_some_and(|value| pure_projection_part(&value, inspector))
        }
        | Expr::Call(value) => {
            matches!(&*value.func, Expr::Path(_))
                && value.args.iter().all(|value| pure_projection_part(value, inspector))
        }
        | Expr::MethodCall(call) => {
            matches!(
                identifier(&call.method).as_str(),
                "clone"
                    | "to_owned"
                    | "to_string"
                    | "as_str"
                    | "as_ref"
                    | "as_mut"
                    | "as_deref"
                    | "as_deref_mut"
                    | "to_vec"
                    | "iter"
                    | "iter_mut"
                    | "into_iter"
                    | "map"
                    | "collect"
                    | "cloned"
                    | "copied"
                    | "unwrap"
                    | "expect"
            ) && pure_projection_part(&call.receiver, inspector)
                && call.args.iter().all(|value| pure_projection_part(value, inspector))
        }
        | _ => false,
    }
}

fn field_projection(expression: &Expr) -> bool {
    match expression {
        | Expr::Field(_) => true,
        | Expr::MethodCall(call) => field_projection(&call.receiver),
        | Expr::Reference(value) => field_projection(&value.expr),
        | Expr::Try(value) => field_projection(&value.expr),
        | _ => false,
    }
}

fn single_expression(block: &syn::Block) -> Option<&Expr> {
    match block.stmts.as_slice() {
        | [syn::Stmt::Expr(Expr::Return(value), _)] => value.expr.as_deref(),
        | [syn::Stmt::Expr(expression, _)] => Some(expression),
        | _ => None,
    }
}

fn expression_root(expression: &Expr) -> Option<&str> {
    match expression {
        | Expr::Path(path) if path.path.is_ident("self") => Some("self"),
        | Expr::Field(value) => expression_root(&value.base),
        | Expr::MethodCall(value) => expression_root(&value.receiver),
        | Expr::Try(value) => expression_root(&value.expr),
        | Expr::Reference(value) => expression_root(&value.expr),
        | Expr::Paren(value) => expression_root(&value.expr),
        | _ => None,
    }
}

fn delegates(block: &syn::Block) -> bool {
    fn delegated(value: &Expr) -> bool {
        match value {
            | Expr::Call(call) => {
                if let Expr::Path(path) = &*call.func {
                    return path.path.segments.iter().any(|segment| segment.ident == "mappers");
                }
                false
            }
            | Expr::MethodCall(call)
                if matches!(identifier(&call.method).as_str(), "into" | "try_into") =>
            {
                true
            }
            | Expr::MethodCall(call) => delegated(&call.receiver),
            | Expr::Try(value) => delegated(&value.expr),
            | Expr::Paren(value) => delegated(&value.expr),
            | _ => false,
        }
    }
    single_expression(block).is_some_and(delegated)
}

fn structural_value(expression: &Expr) -> bool {
    match expression {
        | Expr::Lit(_) | Expr::Path(_) => true,
        | Expr::Call(call) => call.args.iter().all(structural_value),
        | Expr::Macro(_) => true,
        | Expr::Block(block) => single_expression(&block.block).is_some_and(structural_value),
        | _ => false,
    }
}

impl<'ast> Visit<'ast> for Inspector<'_> {
    fn visit_expr_for_loop(
        &mut self,
        expression: &'ast syn::ExprForLoop,
    ) {
        self.visit_expr(&expression.expr);
        let element = self.expression_type(&expression.expr).and_then(|ty| element_type(&ty));
        self.scoped(|this| {
            if let Some(element) = element {
                this.bind(&expression.pat, element);
            }
            this.visit_block(&expression.body);
        });
    }

    fn visit_expr_match(
        &mut self,
        expression: &'ast syn::ExprMatch,
    ) {
        self.visit_expr(&expression.expr);
        if self.enum_translation(expression) {
            self.report(
                expression.match_token.span(),
                "enum translation must be implemented under mappers/",
            );
        }
        let ty = self.expression_type(&expression.expr);
        for arm in &expression.arms {
            self.scoped(|this| {
                if let Some(ty) = &ty {
                    this.bind(&arm.pat, ty.clone());
                }
                if let Some((_, guard)) = &arm.guard {
                    this.condition(guard);
                }
                this.visit_expr(&arm.body);
            });
        }
    }

    fn visit_expr_if(
        &mut self,
        expression: &'ast syn::ExprIf,
    ) {
        self.scoped(|this| {
            this.condition(&expression.cond);
            this.visit_block(&expression.then_branch);
        });
        if let Some((_, otherwise)) = &expression.else_branch {
            self.visit_expr(otherwise);
        }
    }

    fn visit_expr_while(
        &mut self,
        expression: &'ast syn::ExprWhile,
    ) {
        self.scoped(|this| {
            this.condition(&expression.cond);
            this.visit_block(&expression.body);
        });
    }

    fn visit_expr_call(
        &mut self,
        expression: &'ast syn::ExprCall,
    ) {
        if let Expr::Path(path) = &*expression.func {
            let target = self.variant(&path.path).map(|(owner, _)| owner).or_else(|| {
                let (owner, model) = self.model(&path.path)?;
                (model.data
                    && !model.fields.is_empty()
                    && model.fields.keys().all(|field| field.parse::<usize>().is_ok())
                    && self.catalog.resolve(&self.scope, &path_names(&path.path)) == owner)
                    .then_some(owner)
            });
            if let Some(target) = target
                && target.as_slice() != ["shared", "errors", "Error"]
                && expression.args.iter().any(|value| self.is_projection(value, &target))
            {
                self.report(
                    path.path.span(),
                    "variant or tuple model assembly must be implemented under mappers/",
                );
            }
        }
        visit::visit_expr_call(self, expression);
    }
    fn visit_block(
        &mut self,
        block: &'ast syn::Block,
    ) {
        let previous_catalog = self.block_catalog(block);
        let variables = self.variables.clone();
        let projections = self.projections.clone();
        let bindings = self.bindings.clone();
        visit::visit_block(self, block);
        self.variables = variables;
        self.projections = projections;
        self.bindings = bindings;
        self.catalog = previous_catalog;
    }
    fn visit_item(
        &mut self,
        item: &'ast Item,
    ) {
        if !test_only(item_attributes(item)) {
            visit::visit_item(self, item);
        }
    }

    fn visit_item_mod(
        &mut self,
        item: &'ast syn::ItemMod,
    ) {
        self.scope.push(identifier(&item.ident));
        visit::visit_item_mod(self, item);
        self.scope.pop();
    }

    fn visit_item_impl(
        &mut self,
        item: &'ast syn::ItemImpl,
    ) {
        if let Some((_, trait_path, _)) = &item.trait_ {
            let path = self.catalog.resolve(&self.scope, &path_names(trait_path));
            let api_error = type_path(&item.self_ty).is_some_and(|ty| {
                self.catalog.resolve(&self.scope, &path_names(ty)).as_slice()
                    == ["app", "api", "error", "ApiError"]
            });
            let shared_error_wrapper = path.last().is_some_and(|name| name == "From")
                && trait_path.segments.last().is_some_and(|segment| {
                    matches!(&segment.arguments, PathArguments::AngleBracketed(arguments)
                        if matches!(arguments.args.first(), Some(GenericArgument::Type(ty))
                            if type_path(&self.catalog.declared_type(&self.scope, ty)).is_some_and(|ty|
                                self.catalog.resolve(&self.scope, &path_names(ty)).as_slice()
                                    == ["shared", "errors", "Error"])))
                });
            if api_error
                && (path.as_slice() == ["axum", "response", "IntoResponse"] || shared_error_wrapper)
            {
                return;
            }
            if conversion_trait(&path) {
                self.report(
                    item.impl_token.span(),
                    "conversion implementation must be under mappers/",
                );
                return;
            }
            if path.last().is_some_and(|name| matches!(name.as_str(), "Clone" | "Default")) {
                return;
            }
        }
        let previous = self.self_type.take();
        self.self_type = type_path(&item.self_ty).cloned();
        visit::visit_item_impl(self, item);
        self.self_type = previous;
    }

    fn visit_item_fn(
        &mut self,
        item: &'ast syn::ItemFn,
    ) {
        self.inspect_function(&item.sig, &item.block);
    }

    fn visit_impl_item_fn(
        &mut self,
        item: &'ast syn::ImplItemFn,
    ) {
        if !test_only(&item.attrs) {
            self.inspect_function(&item.sig, &item.block);
        }
    }

    fn visit_attribute(
        &mut self,
        attribute: &'ast Attribute,
    ) {
        if attribute.path().is_ident("from") {
            self.report(
                attribute.span(),
                "derived conversion must be implemented under mappers/",
            );
        }
        if attribute.path().is_ident("derive")
            && let Ok(paths) =
                attribute.parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
            && paths
                .iter()
                .any(|path| conversion_trait(&self.catalog.resolve(&self.scope, &path_names(path))))
        {
            self.report(
                attribute.span(),
                "derived conversion must be implemented under mappers/",
            );
        }
    }

    fn visit_local(
        &mut self,
        local: &'ast syn::Local,
    ) {
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            self.bind_local(local);
            if let Some((_, diverge)) = &init.diverge {
                self.visit_expr(diverge);
            }
        }
    }

    fn visit_expr_struct(
        &mut self,
        expression: &'ast syn::ExprStruct,
    ) {
        let model = self.model(&expression.path);
        if let Some((target, model)) = model {
            let external = target.first() != self.scope.first();
            let schema =
                target.iter().any(|name| matches!(name.as_str(), "schemas" | "pb" | "models"));
            let projected =
                expression.fields.iter().any(|field| self.is_projection(&field.expr, &target));
            if model.data && (external || schema || projected) {
                self.report(
                    expression.path.span(),
                    "model assembly must be implemented under mappers/",
                );
            }
        }
        else {
            let names = self.catalog.resolve(&self.scope, &path_names(&expression.path));
            if names.iter().any(|name| name == "pb") {
                self.report(
                    expression.path.span(),
                    "schema assembly must be implemented under mappers/",
                );
            }
        }
        visit::visit_expr_struct(self, expression);
    }

    fn visit_expr_closure(
        &mut self,
        closure: &'ast syn::ExprClosure,
    ) {
        let previous = self.variables.clone();
        let projections = self.projections.clone();
        let bindings = self.bindings.clone();
        for pattern in &closure.inputs {
            if let Pat::Type(pattern) = pattern {
                self.bind(&pattern.pat, (*pattern.ty).clone());
            }
        }
        if let Expr::Tuple(tuple) = &*closure.body
            && (tuple.elems.iter().any(converts_scalar)
                || (!tuple.elems.is_empty() && tuple.elems.iter().all(field_projection)))
        {
            self.report(
                closure.span(),
                "collection projection must be implemented under mappers/",
            );
        }
        visit::visit_expr_closure(self, closure);
        self.variables = previous;
        self.projections = projections;
        self.bindings = bindings;
    }

    fn visit_expr_method_call(
        &mut self,
        call: &'ast syn::ExprMethodCall,
    ) {
        let name = identifier(&call.method);
        if matches!(
            name.as_str(),
            "map" | "flat_map" | "filter_map" | "for_each" | "filter" | "find" | "any" | "all"
        ) {
            self.visit_expr(&call.receiver);
            let element = self.expression_type(&call.receiver).and_then(|ty| element_type(&ty));
            for argument in &call.args {
                let previous = self.variables.clone();
                let projections = self.projections.clone();
                let bindings = self.bindings.clone();
                if let (Some(ty), Expr::Closure(closure)) = (&element, argument)
                    && let Some(pattern) = closure.inputs.first()
                {
                    self.bind(pattern, ty.clone());
                }
                self.visit_expr(argument);
                self.variables = previous;
                self.projections = projections;
                self.bindings = bindings;
            }
        }
        else {
            if name == "map_err"
                && (self.scope.first().is_some_and(|name| {
                    matches!(name.as_str(), "application" | "infrastructure" | "shared")
                }) || matches!(self.scope.as_slice(), [package, module, ..]
                    if package == "app" && matches!(module.as_str(), "api" | "internal")))
            {
                for argument in &call.args {
                    if let Expr::Closure(closure) = argument
                        && !matches!(&*closure.body, Expr::MethodCall(call) if matches!(identifier(&call.method).as_str(), "into" | "try_into"))
                    {
                        self.report(
                            closure.span(),
                            "error translation must be implemented under mappers/",
                        );
                    }
                }
            }
            visit::visit_expr_method_call(self, call);
        }
    }

    fn visit_macro(
        &mut self,
        macro_: &'ast syn::Macro,
    ) {
        let names = self.catalog.resolve(&self.scope, &path_names(&macro_.path));
        if names.last().is_some_and(|name| name == "params")
            && names.iter().any(|name| name == "hiqlite")
        {
            self.report(
                macro_.path.span(),
                "persistence parameter projection must be under mappers/",
            );
        }
        self.inspect_macro_tokens(macro_.tokens.clone());
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
                    self.inspect_macro_tokens(macro_template(group.stream()));
                }
            }
        }
        else {
            self.visit_macro(&item.mac);
        }
    }
}

fn converts_scalar(expression: &Expr) -> bool {
    match expression {
        | Expr::MethodCall(call) => {
            matches!(
                identifier(&call.method).as_str(),
                "to_string" | "as_str" | "to_vec"
            ) && field_projection(&call.receiver)
                || converts_scalar(&call.receiver)
        }
        | Expr::Reference(value) => converts_scalar(&value.expr),
        | Expr::Try(value) => converts_scalar(&value.expr),
        | _ => false,
    }
}

fn macro_template(tokens: TokenStream) -> TokenStream {
    let mut result = TokenStream::new();
    let mut tokens = tokens.into_iter().peekable();
    while let Some(token) = tokens.next() {
        match token {
            | TokenTree::Punct(punctuation) if punctuation.as_char() == '$' => {
                match tokens.next() {
                    | Some(TokenTree::Group(group)) => {
                        result.extend(macro_template(group.stream()));
                        if tokens.peek().is_some_and(|token| matches!(token, TokenTree::Punct(value) if !matches!(value.as_char(), '*' | '+' | '?'))) {
                            tokens.next();
                        }
                        if tokens.peek().is_some_and(|token| matches!(token, TokenTree::Punct(value) if matches!(value.as_char(), '*' | '+' | '?'))) {
                            tokens.next();
                        }
                    }
                    | Some(token) => result.extend([token]),
                    | None => {}
                }
            }
            | TokenTree::Group(group) => {
                let mut replaced = Group::new(group.delimiter(), macro_template(group.stream()));
                replaced.set_span(group.span());
                result.extend([TokenTree::Group(replaced)]);
            }
            | token => result.extend([token]),
        }
    }
    result
}

fn violations(
    source: &Source,
    catalog: &Catalog,
) -> Vec<Violation> {
    if mapper_path(&source.path) {
        return Vec::new();
    }
    let mut inspector = Inspector::new(catalog, source.scope.clone());
    if source.path.file_name().is_some_and(|name| name == "mappers.rs") {
        fn namespace(
            inspector: &mut Inspector<'_>,
            items: &[Item],
        ) {
            for item in items {
                if test_only(item_attributes(item)) {
                    continue;
                }
                match item {
                    | Item::Use(_) => {}
                    | Item::Mod(module) => {
                        if !test_only(&module.attrs)
                            && let Some((_, items)) = &module.content
                        {
                            namespace(inspector, items);
                        }
                    }
                    | item => {
                        inspector.report(
                            item.span(),
                            "mappers.rs may only declare modules and reexports; implementations \
                             belong under mappers/",
                        )
                    }
                }
            }
        }
        namespace(&mut inspector, &source.syntax.items);
    }
    else {
        inspector.visit_file(&source.syntax);
    }
    inspector.violations
}

pub(crate) fn check_mappings(root: &Path) -> Result<usize, Box<dyn Error>> {
    let mut sources = Vec::new();
    for (path, source) in rust_sources(root)? {
        let syntax = syn::parse_file(&source).map_err(|error| format!("{path}: {error}"))?;
        let path = normalized(Path::new(&path));
        let source = Source {
            scope: source_scope(&path),
            path,
            syntax,
        };
        sources.push(source);
    }
    let tests = test_files(&sources);
    let mut catalog = Catalog::default();
    for source in &sources {
        if !tests.contains(&source.path) {
            catalog.collect(&source.scope, &source.syntax.items);
        }
    }
    catalog.classify_runtime_fields();
    let mut count = 0;
    for source in &mut sources {
        if tests.contains(&source.path) {
            continue;
        }
        let actual = fs::canonicalize(root.join(&source.path))?;
        let actual = actual.strip_prefix(root).unwrap_or(&actual);
        let original = std::mem::replace(&mut source.path, actual.to_owned());
        for violation in violations(source, &catalog) {
            eprintln!(
                "{}:{}:{}: error: {} [repo::mapper_location]",
                original.display(),
                violation.line,
                violation.column,
                violation.reason
            );
            count += 1;
        }
        source.path = original;
    }
    Ok(count)
}

#[cfg(test)]
mod tests;
