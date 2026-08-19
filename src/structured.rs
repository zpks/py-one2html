//! In-memory, structured conversion.
//!
//! Unlike [`crate::convert`], which renders to a directory on disk (pages,
//! assets, and a table-of-contents page), this renders `.one` sections and
//! `.onepkg` packages from byte buffers into per-page HTML strings plus
//! in-memory asset maps. The page ↔ HTML mapping comes straight from the
//! render loop — there is no table of contents to correlate against — and
//! parser warnings are returned as data instead of being rendered into a
//! warnings page.

use crate::{Options, page, section};
use color_eyre::eyre::Result;
use onenote_parser::fs::FileSource;
use onenote_parser::section::Section;
#[cfg(feature = "onepkg")]
use onenote_parser::section::SectionEntry;
use onenote_parser::{FileSystem, Parser};
use std::collections::BTreeMap;
use std::io::{Error, ErrorKind, Read};
use std::sync::{Arc, Mutex};
use typed_path::{NativePath, TypedPath, TypedPathBuf};

/// One rendered page with the metadata useful for page-aware chunking.
pub struct PageHtml {
    pub title: Option<String>,
    pub level: i32,
    pub link_target_id: String,
    pub author: Option<String>,
    /// Unix timestamps.
    pub created: i64,
    pub updated: i64,
    pub html: String,
}

/// A fully rendered section.
pub struct SectionHtml {
    pub display_name: String,
    /// Display names of the section groups this section is nested under,
    /// outermost first. Empty for top-level sections and for sections parsed
    /// standalone via [`convert_section_buffer`].
    pub group_path: Vec<String>,
    pub pages: Vec<PageHtml>,
    /// Images and embedded attachments, keyed by the exact (percent-decoded)
    /// filename the page HTML references in `src=` attributes.
    pub assets: BTreeMap<String, Vec<u8>>,
    /// Non-fatal parser warnings, prefixed with the page title when known.
    pub warnings: Vec<String>,
}

/// A fully rendered `.onepkg` notebook package.
pub struct NotebookHtml {
    /// Derived from the package file name; a package carries no notebook name.
    pub display_name: String,
    /// All sections in the notebook, including those inside section groups,
    /// in table-of-contents order.
    pub sections: Vec<SectionHtml>,
    /// Notebook-level (table-of-contents) parser warnings; per-section
    /// warnings live on each [`SectionHtml`].
    pub warnings: Vec<String>,
}

fn unsupported() -> Error {
    Error::new(ErrorKind::Unsupported, "not supported by in-memory fs")
}

/// Read-only `FileSystem` exposing a single in-memory file: the input buffer
/// handed to the parser. Satisfies the parser's FS parameter without touching
/// disk (and without requiring the `native-fs` feature).
#[derive(Copy, Clone)]
struct InputFs<'a> {
    name: &'a str,
    data: &'a [u8],
}

impl FileSystem for InputFs<'_> {
    fn is_directory(&self, _: TypedPath) -> Result<bool, Error> {
        Ok(false)
    }

    fn read_dir(&self, _: TypedPath) -> Result<Vec<TypedPathBuf>, Error> {
        Err(unsupported())
    }

    fn read_file(&self, path: TypedPath) -> Result<Vec<u8>, Error> {
        if path.to_string_lossy() == self.name {
            Ok(self.data.to_vec())
        } else {
            Err(Error::new(
                ErrorKind::NotFound,
                format!("in-memory fs only holds {:?}", self.name),
            ))
        }
    }

    fn write_file(&self, _: TypedPath, _: &[u8]) -> Result<(), Error> {
        Err(unsupported())
    }

    fn stream_to_file(&self, _: TypedPath, _: &mut dyn Read) -> Result<(), Error> {
        Err(unsupported())
    }

    fn make_dir(&self, _: TypedPath) -> Result<(), Error> {
        Err(unsupported())
    }

    fn canonicalize(&self, path: TypedPath) -> Result<TypedPathBuf, Error> {
        Ok(path.to_path_buf())
    }

    fn exists(&self, path: TypedPath) -> Result<bool, Error> {
        Ok(path.to_string_lossy() == self.name)
    }

    fn open_file(&self, _: TypedPath) -> Result<Arc<dyn FileSource>, Error> {
        Err(unsupported())
    }

    fn is_windows(&self) -> bool {
        false
    }
}

/// Write-only in-memory `FileSystem`: captures the asset files the page
/// renderer streams out. The render path never reads back what it wrote, so
/// all read operations are unsupported.
#[derive(Copy, Clone)]
struct MemFs<'a> {
    files: &'a Mutex<BTreeMap<String, Vec<u8>>>,
}

impl MemFs<'_> {
    fn insert(&self, path: TypedPath, data: Vec<u8>) {
        let key = path.to_string_lossy().trim_start_matches('/').to_owned();
        self.files
            .lock()
            .expect("no panics while holding the lock")
            .insert(key, data);
    }
}

impl FileSystem for MemFs<'_> {
    fn is_directory(&self, _: TypedPath) -> Result<bool, Error> {
        Err(unsupported())
    }

    fn read_dir(&self, _: TypedPath) -> Result<Vec<TypedPathBuf>, Error> {
        Err(unsupported())
    }

    fn read_file(&self, _: TypedPath) -> Result<Vec<u8>, Error> {
        Err(unsupported())
    }

    fn write_file(&self, path: TypedPath, data: &[u8]) -> Result<(), Error> {
        self.insert(path, data.to_vec());
        Ok(())
    }

    fn stream_to_file(&self, path: TypedPath, reader: &mut dyn Read) -> Result<(), Error> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        self.insert(path, data);
        Ok(())
    }

    fn make_dir(&self, _: TypedPath) -> Result<(), Error> {
        Ok(())
    }

    fn canonicalize(&self, path: TypedPath) -> Result<TypedPathBuf, Error> {
        Ok(path.to_path_buf())
    }

    fn exists(&self, _: TypedPath) -> Result<bool, Error> {
        Ok(false)
    }

    fn open_file(&self, _: TypedPath) -> Result<Arc<dyn FileSource>, Error> {
        Err(unsupported())
    }

    fn is_windows(&self) -> bool {
        false
    }
}

fn collect_warnings(report: &onenote_parser::warn::Report) -> Vec<String> {
    report
        .warnings()
        .iter()
        .map(|w| match w.page() {
            Some((_, title)) => format!("{}: {}", title, w.message()),
            None => w.message().to_string(),
        })
        .collect()
}

/// Render one parsed section's pages to HTML with assets captured in memory.
fn render_section(section: &Section, options: Options) -> Result<SectionHtml> {
    let files = Mutex::new(BTreeMap::new());
    let fs = MemFs { files: &files };
    // Assets land directly under the output root, so their stored keys equal
    // the relative filenames the HTML references.
    let output_dir = TypedPathBuf::from_unix("/");

    let mut section_renderer = section::Renderer::new();
    let mut pages = Vec::new();

    for page_series in section.page_series() {
        for page in page_series.pages() {
            let mut renderer =
                page::Renderer::new(output_dir.clone(), &mut section_renderer, options, fs);
            let html = renderer.render_page(page)?;

            pages.push(PageHtml {
                title: page.title_text().map(str::to_string),
                level: page.level(),
                link_target_id: page.link_target_id().to_string(),
                author: page.author().map(str::to_string),
                created: page.created_time().unix_timestamp(),
                updated: page.updated_time().unix_timestamp(),
                html,
            });
        }
    }

    Ok(SectionHtml {
        display_name: section.display_name().to_string(),
        group_path: Vec::new(),
        pages,
        assets: files.into_inner().expect("no panics while holding the lock"),
        warnings: collect_warnings(section.report()),
    })
}

/// Parse a `.one` section from a byte buffer and render every page to HTML
/// in memory. `file_name` populates section metadata — its stem becomes the
/// section's `display_name`, since a `.one` buffer does not carry its own
/// name. `options.warnings` is ignored: warnings always come back on
/// [`SectionHtml::warnings`] instead of as a rendered page.
pub fn convert_section_buffer(data: &[u8], file_name: &str, options: Options) -> Result<SectionHtml> {
    let fs = InputFs {
        name: file_name,
        data,
    };
    let section = Parser::new_with_fs(fs)
        .parse_section_buffer(data, NativePath::new(file_name.as_bytes()).to_typed_path())?;

    render_section(&section, options)
}

#[cfg(feature = "onepkg")]
fn collect_sections(
    entries: &[SectionEntry],
    group_path: &[String],
    options: Options,
    out: &mut Vec<SectionHtml>,
) -> Result<()> {
    for entry in entries {
        match entry {
            SectionEntry::Section(section) => {
                let mut rendered = render_section(section, options)?;
                rendered.group_path = group_path.to_vec();
                out.push(rendered);
            }
            SectionEntry::SectionGroup(group) => {
                let mut path = group_path.to_vec();
                path.push(group.display_name().to_string());
                collect_sections(group.entries(), &path, options, out)?;
            }
        }
    }
    Ok(())
}

/// Parse a `.onepkg` notebook package from a byte buffer and render every
/// section to HTML in memory. The cabinet archive is decompressed in memory;
/// nothing touches disk. Section display names come from the notebook's table
/// of contents (unlike [`convert_section_buffer`], which can only derive them
/// from the file name). The notebook `display_name` is the stem of
/// `file_name`, as packages carry no notebook name.
#[cfg(feature = "onepkg")]
pub fn convert_package_buffer(
    data: &[u8],
    file_name: &str,
    options: Options,
) -> Result<NotebookHtml> {
    let fs = InputFs {
        name: file_name,
        data,
    };
    let notebook = Parser::new_with_fs(fs)
        .parse_package(NativePath::new(file_name.as_bytes()).to_typed_path())?;

    let mut sections = Vec::new();
    collect_sections(notebook.entries(), &[], options, &mut sections)?;

    let display_name = file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name)
        .to_string();

    Ok(NotebookHtml {
        display_name,
        sections,
        warnings: collect_warnings(notebook.report()),
    })
}
