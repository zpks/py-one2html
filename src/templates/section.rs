use askama::Template;
use color_eyre::Result;
use color_eyre::eyre::WrapErr;

#[derive(Template)]
#[template(path = "section.html")]
struct NotebookTemplate<'a> {
    name: &'a str,
    entries: &'a [Entry],
}

pub(crate) struct Entry {
    pub name: String,
    pub path: String,
    pub level: i32,
    pub is_warnings: bool,
}

pub(crate) fn render(name: &str, entries: &[Entry]) -> Result<String> {
    NotebookTemplate { name, entries }
        .render()
        .wrap_err("Failed to render section template")
}

mod filters {
    pub(crate) use crate::templates::url_encode as encode;
}
