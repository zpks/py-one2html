use color_eyre::eyre::{Result, eyre};
use itertools::Itertools;
use onenote_parser::FileSystem;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
#[cfg(feature = "bin")]
use std::time::Duration;

// Terminal spinner only for the CLI; `indicatif` is a `bin`-feature dependency.
#[cfg(feature = "bin")]
pub(crate) fn with_progress<T, F: FnMut() -> T>(msg: &'static str, mut f: F) -> T {
    let bar = indicatif::ProgressBar::new_spinner();
    bar.set_message(msg);
    bar.enable_steady_tick(Duration::from_millis(16));

    let ret = f();

    bar.finish_and_clear();

    print!("\r");

    ret
}

#[cfg(not(feature = "bin"))]
pub(crate) fn with_progress<T, F: FnMut() -> T>(msg: &'static str, mut f: F) -> T {
    log::info!("{}", msg);

    f()
}

pub(crate) fn px(inches: f32) -> String {
    format!("{}px", (inches * 48.0).round())
}

pub(crate) fn sanitize_output_filename(filename: &str, fs: impl FileSystem) -> Result<String> {
    if filename.is_empty() {
        return Err(eyre!("Output filename is empty"));
    }
    let basename = filename
        .rsplit_once(&['/', '\\'][..])
        .map(|(_, b)| b)
        .unwrap_or(filename);
    sanitize_path(basename, fs)
}

pub(crate) fn sanitize_path(path: &str, fs: impl FileSystem) -> Result<String> {
    let sanitized = sanitize_filename::sanitize_with_options(
        path,
        sanitize_filename::Options {
            windows: fs.is_windows(),
            ..Default::default()
        },
    );

    if sanitized.is_empty() {
        return Err(eyre!("Path is empty after sanitization"));
    }

    Ok(sanitized)
}

pub(crate) fn html_entities(text: &str) -> String {
    // Match the "special chars" set: &, <, >, ", '. Anything user-controllable
    // that flows into HTML markup (filenames, alt text, etc.) must go through
    // here — `AttributeSet`'s `Display` does this automatically for values.
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}

pub(crate) fn detect_png(header: &[u8]) -> bool {
    // PNGs start with a specific set of bytes. See https://en.wikipedia.org/wiki/PNG
    //                  0x89  'P'   'N'   'G'   \r    \n
    header.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A])
}

pub(crate) struct AttributeSet(HashMap<&'static str, String>);

impl AttributeSet {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }

    pub(crate) fn set(&mut self, attribute: &'static str, value: String) {
        self.0.insert(attribute, value);
    }
}

impl Display for AttributeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .sorted_by(|(a, _), (b, _)| Ord::cmp(a, b))
                .map(|(attr, value)| attr.to_string() + "=\"" + &html_entities(value) + "\"")
                .join(" ")
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StyleSet(HashMap<&'static str, String>);

impl StyleSet {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }

    pub(crate) fn set(&mut self, prop: &'static str, value: String) {
        self.0.insert(prop, value);
    }

    pub(crate) fn extend(&mut self, other: Self) {
        self.0.extend(other.0)
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn is_bold(&self) -> bool {
        self.0
            .get("font-weight")
            .map(|weight| weight == "bold")
            .unwrap_or(false)
    }

    pub(crate) fn is_italic(&self) -> bool {
        self.0
            .get("font-style")
            .map(|style| style == "italic")
            .unwrap_or(false)
    }

    /// Render this style set as the `style="..."` attribute, with the value
    /// HTML-escaped. Use this when embedding the styles inside an element's
    /// attributes; `Display` itself produces raw CSS and is unsafe to drop
    /// straight into an attribute value.
    pub(crate) fn to_html_attr(&self) -> String {
        format!("style=\"{}\"", html_entities(&self.to_string()))
    }
}

impl Display for StyleSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .sorted_by(|(a, _), (b, _)| Ord::cmp(a, b))
                .map(|(attr, value)| attr.to_string() + ": " + value + ";")
                .join(" ")
        )
    }
}
