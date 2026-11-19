use proc_macro2::Span;

use super::super::Violation;

pub(in crate::errors) fn violation(
    path: &str,
    span: Span,
    reason: String,
) -> Violation {
    let position = span.start();
    Violation {
        path: path.to_owned(),
        line: position.line.max(1),
        column: position.column + 1,
        reason,
    }
}
