use askama::Template;
use color_eyre::Result;
use color_eyre::eyre::WrapErr;

#[derive(Template)]
#[template(path = "warnings.html")]
struct WarningsPageTemplate<'a> {
    section: &'a str,
    entries: &'a [Entry<'a>],
}

pub(crate) struct Entry<'a> {
    pub page: &'a str,
    pub message: &'a str,
}

pub(crate) fn render(section: &str, entries: &[Entry<'_>]) -> Result<String> {
    WarningsPageTemplate { section, entries }
        .render()
        .wrap_err("Failed to render warnings list template")
}
