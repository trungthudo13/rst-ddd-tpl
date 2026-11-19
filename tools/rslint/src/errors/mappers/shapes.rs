use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    path::PathBuf,
};

use syn::{
    GenericArgument,
    PathArguments,
    Type,
};

use super::super::{
    Catalog,
    Shape,
    Source,
    names,
};

pub(in crate::errors) fn test_files(sources: &[Source]) -> BTreeSet<PathBuf> {
    let sources: Vec<_> = sources
        .iter()
        .map(|source| {
            crate::mappings::Source {
                path: PathBuf::from(&source.path),
                scope: source.scope.clone(),
                syntax: source.syntax.clone(),
            }
        })
        .collect();
    crate::mappings::test_files(&sources)
}

pub(in crate::errors) fn shape(
    catalog: &Catalog,
    scope: &[String],
    ty: &Type,
    associated: &BTreeMap<String, Shape>,
) -> Shape {
    catalog.shape(scope, ty, associated)
}

impl Catalog {
    pub(in crate::errors) fn shape(
        &self,
        scope: &[String],
        ty: &Type,
        associated: &BTreeMap<String, Shape>,
    ) -> Shape {
        self.shape_with(scope, ty, associated, &BTreeMap::new(), 0)
    }

    fn shape_with(
        &self,
        scope: &[String],
        ty: &Type,
        associated: &BTreeMap<String, Shape>,
        parameters: &BTreeMap<String, Shape>,
        depth: usize,
    ) -> Shape {
        if depth > 32 {
            return Shape::Other;
        }
        match ty {
            | Type::Path(ty) => {
                let path = names(&ty.path);
                if path.first().is_some_and(|part| part == "Self")
                    && path.len() == 2
                    && let Some(ty) = associated.get(&path[1])
                {
                    return ty.clone();
                }
                if path.len() == 1
                    && let Some(ty) = parameters.get(&path[0])
                {
                    return ty.clone();
                }
                let target = self.resolve(scope, &path, 0);
                let arguments: Vec<_> = ty
                    .path
                    .segments
                    .last()
                    .into_iter()
                    .flat_map(|part| {
                        match &part.arguments {
                            | PathArguments::AngleBracketed(arguments) => {
                                arguments
                                    .args
                                    .iter()
                                    .filter_map(|argument| {
                                        match argument {
                                            | GenericArgument::Type(ty) => {
                                                Some(self.shape_with(
                                                    scope,
                                                    ty,
                                                    associated,
                                                    parameters,
                                                    depth + 1,
                                                ))
                                            }
                                            | _ => None,
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            }
                            | _ => Vec::new(),
                        }
                    })
                    .collect();
                if let Some(alias) = self.types.get(&target) {
                    let mut replacements = BTreeMap::new();
                    for (index, parameter) in alias.item.generics.type_params().enumerate() {
                        let value = arguments
                            .get(index)
                            .cloned()
                            .or_else(|| {
                                parameter.default.as_ref().map(|ty| {
                                    self.shape_with(
                                        &alias.scope,
                                        ty,
                                        associated,
                                        &replacements,
                                        depth + 1,
                                    )
                                })
                            })
                            .unwrap_or(Shape::Other);
                        replacements.insert(parameter.ident.to_string(), value);
                    }
                    return self.shape_with(
                        &alias.scope,
                        &alias.item.ty,
                        associated,
                        &replacements,
                        depth + 1,
                    );
                }
                Shape::Path(target, arguments)
            }
            | Type::Reference(ty) => {
                self.shape_with(scope, &ty.elem, associated, parameters, depth + 1)
            }
            | Type::Paren(ty) => {
                self.shape_with(scope, &ty.elem, associated, parameters, depth + 1)
            }
            | Type::Group(ty) => {
                self.shape_with(scope, &ty.elem, associated, parameters, depth + 1)
            }
            | Type::Tuple(ty) => {
                Shape::Tuple(
                    ty.elems
                        .iter()
                        .map(|ty| self.shape_with(scope, ty, associated, parameters, depth + 1))
                        .collect(),
                )
            }
            | Type::Infer(_) => Shape::Inferred,
            | _ => Shape::Other,
        }
    }
}
