use std::{
    error::Error,
    fs,
    io,
    path::Path,
    process::{
        Command,
        ExitCode,
    },
};

use proc_macro2::{
    Group,
    TokenStream,
    TokenTree,
};
use ra_ap_rustc_lexer::{
    FrontmatterAllowed,
    TokenKind,
    strip_shebang,
    tokenize,
};

mod arguments;
mod errors;
mod mappings;
mod shared_values;

fn main() -> Result<ExitCode, Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("repository root is missing")?;
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.as_slice() == ["--errors-only"] {
        return Ok(if errors::check_errors(root)? == 0 {
            ExitCode::SUCCESS
        }
        else {
            ExitCode::FAILURE
        });
    }
    if !arguments.is_empty() {
        return Err("usage: cargo lint [--errors-only]".into());
    }
    let comments = check_comments(root)?;
    if comments > 0 {
        eprintln!("error: {comments} Rust comments violate repo::no_comments");
    }
    let mappings = mappings::check_mappings(root)?;
    if mappings > 0 {
        eprintln!("error: {mappings} representation mappings violate repo::mapper_location");
    }
    let arguments = arguments::check_arguments(root)?;
    if arguments > 0 {
        eprintln!("error: {arguments} inline struct arguments violate repo::named_arguments");
    }
    let errors = errors::check_errors(root)?;
    if errors > 0 {
        eprintln!("error: {errors} error contracts violate repo::shared_errors");
    }
    let values = shared_values::check_shared_values(root)?;
    if values > 0 {
        eprintln!("error: {values} enum or constant declarations violate repo::shared_values");
    }
    let clippy = Command::new("cargo")
        .current_dir(root)
        .args(["clippy", "--workspace", "--locked", "--all-targets", "--", "-D", "warnings"])
        .status()?;
    Ok(
        if comments == 0
            && mappings == 0
            && arguments == 0
            && errors == 0
            && values == 0
            && clippy.success()
        {
            ExitCode::SUCCESS
        }
        else {
            ExitCode::FAILURE
        },
    )
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
                        if tokens.peek().is_some_and(|token| matches!(token, TokenTree::Punct(value) if !matches!(value.as_char(), '*' | '+' | '?'))) { tokens.next(); }
                        if tokens.peek().is_some_and(|token| matches!(token, TokenTree::Punct(value) if matches!(value.as_char(), '*' | '+' | '?'))) { tokens.next(); }
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

fn rust_sources(root: &Path) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let files = Command::new("git")
        .current_dir(root)
        .args(["ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", "*.rs"])
        .output()?;
    if !files.status.success() {
        return Err(io::Error::other(String::from_utf8_lossy(&files.stderr)).into());
    }
    let files = String::from_utf8(files.stdout)?;
    let mut files: Vec<_> = files.split('\0').filter(|path| !path.is_empty()).collect();
    files.sort_unstable();
    files.dedup();
    let mut sources = Vec::new();
    for path in files {
        let source = match fs::read_to_string(root.join(path)) {
            | Ok(source) => source,
            | Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            | Err(error) => return Err(format!("{path}: {error}").into()),
        };
        sources.push((path.to_owned(), source));
    }
    Ok(sources)
}

fn check_comments(root: &Path) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;
    for (path, source) in rust_sources(root)? {
        for (line, column) in comment_positions(&source) {
            eprintln!(
                "{path}:{line}:{column}: error: Rust comments are forbidden [repo::no_comments]"
            );
            count += 1;
        }
    }
    Ok(count)
}

fn comment_positions(source: &str) -> Vec<(usize, usize)> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let mut offset = strip_shebang(source).unwrap_or(0);
    let mut positions = Vec::new();
    for token in tokenize(&source[offset..], FrontmatterAllowed::Yes) {
        if matches!(
            token.kind,
            TokenKind::LineComment { .. } | TokenKind::BlockComment { .. }
        ) {
            let prefix = &source[..offset];
            let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
            let column = prefix.chars().rev().take_while(|ch| *ch != '\n').count() + 1;
            positions.push((line, column));
        }
        offset += token.len as usize;
    }
    positions
}

#[cfg(test)]
mod tests;
