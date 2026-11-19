use shared::Error;

pub(crate) fn startup(error: hiqlite::Error) -> Error {
    Error::Unexpected(format!("HiQLite failed to start: {error}"))
}

pub(crate) fn shutdown(error: hiqlite::Error) -> Error {
    Error::Unexpected(format!("HiQLite could not shut down: {error}"))
}

pub(crate) fn migration(
    error: hiqlite::Error,
    shutdown: Option<hiqlite::Error>,
) -> Error {
    let diagnostic = match shutdown {
        | Some(shutdown) => {
            format!("HiQLite could not apply migrations: {error}; shutdown failed: {shutdown}")
        }
        | None => format!("HiQLite could not apply migrations: {error}"),
    };
    Error::Unexpected(diagnostic)
}
