use crate::Options;
use crate::utils::{StyleSet, html_entities};
use askama::Template;
use color_eyre::Result;
use color_eyre::eyre::WrapErr;
use itertools::Itertools;
use std::collections::HashMap;

pub(crate) struct PageTimestamps {
    pub(crate) created_time: i64,
    pub(crate) updated_time: i64,
}

#[derive(Template)]
#[template(path = "page.html", escape = "none")]
struct PageTemplate<'a> {
    page_id_attr: &'a str,
    name: &'a str,
    content: &'a str,
    global_styles: Vec<(&'a String, &'a StyleSet)>,
    options: Options,
    created_date_attr: &'a str,
    updated_date_attr: &'a str,
}

pub(crate) fn render(
    page_id: &str,
    name: &str,
    timestamps: &PageTimestamps,
    content: &str,
    global_styles: &HashMap<String, StyleSet>,
    options: Options,
) -> Result<String> {
    PageTemplate {
        page_id_attr: &html_entities(page_id),
        name: &html_entities(name),
        content,
        global_styles: global_styles
            .iter()
            .sorted_by(|(a, _), (b, _)| Ord::cmp(a, b))
            .collect(),
        options,
        created_date_attr: &timestamps.created_time.to_string(),
        updated_date_attr: &timestamps.updated_time.to_string(),
    }
    .render()
    .wrap_err("Failed to render page template")
}
