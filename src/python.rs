//! Python bindings (`py_one2html` extension module).
//!
//! The structured parse results are exposed as frozen classes rather than
//! dicts. Asset bytes stay on the Rust side and are only copied into Python
//! `bytes` when a specific asset is requested via [`SectionHtml::asset`] —
//! untouched attachments cost nothing.

use crate::structured;
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::collections::BTreeMap;
use std::path::PathBuf;
use typed_path::NativePath;

fn parse_math_target(value: &str) -> PyResult<crate::MathTarget> {
    match value {
        "mathml" => Ok(crate::MathTarget::MathML),
        "latex" => Ok(crate::MathTarget::LaTeX),
        other => Err(PyValueError::new_err(format!(
            "invalid math_target {other:?}, expected \"mathml\" or \"latex\""
        ))),
    }
}

fn parse_note_tag_icons(value: &str) -> PyResult<crate::NoteTagIcons> {
    match value {
        "svg" => Ok(crate::NoteTagIcons::Svg),
        "emoji" => Ok(crate::NoteTagIcons::Emoji),
        other => Err(PyValueError::new_err(format!(
            "invalid note_tag_icons {other:?}, expected \"svg\" or \"emoji\""
        ))),
    }
}

fn parse_options(math_target: &str, note_tag_icons: &str) -> PyResult<crate::Options> {
    Ok(crate::Options {
        warnings: false,
        math_target: parse_math_target(math_target)?,
        note_tag_icons: parse_note_tag_icons(note_tag_icons)?,
    })
}

/// One rendered page with the metadata useful for page-aware chunking.
#[pyclass(frozen, module = "py_one2html")]
pub struct PageHtml {
    #[pyo3(get)]
    title: Option<String>,
    #[pyo3(get)]
    level: i32,
    #[pyo3(get)]
    link_target_id: String,
    #[pyo3(get)]
    author: Option<String>,
    /// Unix timestamp.
    #[pyo3(get)]
    created: i64,
    /// Unix timestamp.
    #[pyo3(get)]
    updated: i64,
    #[pyo3(get)]
    html: String,
}

#[pymethods]
impl PageHtml {
    fn __repr__(&self) -> String {
        let title = match &self.title {
            Some(title) => format!("{title:?}"),
            None => "None".to_string(),
        };
        format!(
            "PageHtml(title={}, level={}, html=<{} chars>)",
            title,
            self.level,
            self.html.len()
        )
    }
}

/// A fully rendered section.
#[pyclass(frozen, module = "py_one2html")]
pub struct SectionHtml {
    #[pyo3(get)]
    display_name: String,
    #[pyo3(get)]
    group_path: Vec<String>,
    pages: Vec<Py<PageHtml>>,
    assets: BTreeMap<String, Vec<u8>>,
    #[pyo3(get)]
    warnings: Vec<String>,
}

#[pymethods]
impl SectionHtml {
    #[getter]
    fn pages(&self, py: Python<'_>) -> Vec<Py<PageHtml>> {
        self.pages.iter().map(|p| p.clone_ref(py)).collect()
    }

    /// Filenames of the images/attachments referenced by the page HTML.
    #[getter]
    fn asset_names(&self) -> Vec<&str> {
        self.assets.keys().map(String::as_str).collect()
    }

    /// Size in bytes per asset, without copying any asset data.
    #[getter]
    fn asset_sizes(&self) -> BTreeMap<&str, usize> {
        self.assets
            .iter()
            .map(|(name, data)| (name.as_str(), data.len()))
            .collect()
    }

    /// One asset's bytes. The data is copied into Python on each call;
    /// assets never requested stay on the Rust side for free.
    fn asset<'py>(&self, py: Python<'py>, name: &str) -> PyResult<Bound<'py, PyBytes>> {
        self.assets
            .get(name)
            .map(|data| PyBytes::new(py, data))
            .ok_or_else(|| PyKeyError::new_err(name.to_owned()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SectionHtml(display_name={:?}, group_path={:?}, pages={}, assets={}, warnings={})",
            self.display_name,
            self.group_path,
            self.pages.len(),
            self.assets.len(),
            self.warnings.len()
        )
    }
}

/// A fully rendered `.onepkg` notebook package.
#[pyclass(frozen, module = "py_one2html")]
pub struct NotebookHtml {
    #[pyo3(get)]
    display_name: String,
    sections: Vec<Py<SectionHtml>>,
    #[pyo3(get)]
    warnings: Vec<String>,
}

#[pymethods]
impl NotebookHtml {
    #[getter]
    fn sections(&self, py: Python<'_>) -> Vec<Py<SectionHtml>> {
        self.sections.iter().map(|s| s.clone_ref(py)).collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "NotebookHtml(display_name={:?}, sections={}, warnings={})",
            self.display_name,
            self.sections.len(),
            self.warnings.len()
        )
    }
}

fn page_obj(py: Python<'_>, page: structured::PageHtml) -> PyResult<Py<PageHtml>> {
    Py::new(
        py,
        PageHtml {
            title: page.title,
            level: page.level,
            link_target_id: page.link_target_id,
            author: page.author,
            created: page.created,
            updated: page.updated,
            html: page.html,
        },
    )
}

fn section_obj(py: Python<'_>, section: structured::SectionHtml) -> PyResult<Py<SectionHtml>> {
    let pages = section
        .pages
        .into_iter()
        .map(|p| page_obj(py, p))
        .collect::<PyResult<Vec<_>>>()?;

    Py::new(
        py,
        SectionHtml {
            display_name: section.display_name,
            group_path: section.group_path,
            pages,
            assets: section.assets,
            warnings: section.warnings,
        },
    )
}

/// Convert a `.one`, `.onetoc2`, or `.onepkg` file to HTML in `output_dir`.
#[pyfunction]
#[pyo3(signature = (path, output_dir, *, warnings = false, math_target = "mathml", note_tag_icons = "svg"))]
fn convert(
    py: Python<'_>,
    path: PathBuf,
    output_dir: PathBuf,
    warnings: bool,
    math_target: &str,
    note_tag_icons: &str,
) -> PyResult<()> {
    let options = crate::Options {
        warnings,
        math_target: parse_math_target(math_target)?,
        note_tag_icons: parse_note_tag_icons(note_tag_icons)?,
    };

    py.detach(|| {
        crate::convert(
            NativePath::new(path.as_os_str().as_encoded_bytes()).to_typed_path(),
            NativePath::new(output_dir.as_os_str().as_encoded_bytes()).to_typed_path(),
            options,
            onenote_parser::fs::native_fs::NativeFs {},
        )
    })
    .map_err(|e| PyRuntimeError::new_err(format!("{e:?}")))
}

/// Parse a `.one` section's bytes into per-page HTML plus lazily-copied
/// assets.
///
/// Pass `file_name` (the original `.one` name) to get a meaningful
/// `display_name` — the buffer itself does not carry one. Defaults suit an
/// HTML→Markdown ingestion pass: LaTeX math and emoji note tags survive as
/// text; MathML and inline SVGs do not.
#[pyfunction]
#[pyo3(signature = (data, *, file_name = "section.one", math_target = "latex", note_tag_icons = "emoji"))]
fn parse_section(
    py: Python<'_>,
    data: &[u8],
    file_name: &str,
    math_target: &str,
    note_tag_icons: &str,
) -> PyResult<Py<SectionHtml>> {
    let options = parse_options(math_target, note_tag_icons)?;
    let section = py
        .detach(|| structured::convert_section_buffer(data, file_name, options))
        .map_err(|e| PyRuntimeError::new_err(format!("{e:?}")))?;
    section_obj(py, section)
}

/// Parse a `.onepkg` notebook package's bytes into rendered sections.
///
/// Sections nested in section groups carry the group names on `group_path`
/// (outermost first). The cabinet archive is decompressed entirely in memory.
#[pyfunction]
#[pyo3(signature = (data, *, file_name = "notebook.onepkg", math_target = "latex", note_tag_icons = "emoji"))]
fn parse_package(
    py: Python<'_>,
    data: &[u8],
    file_name: &str,
    math_target: &str,
    note_tag_icons: &str,
) -> PyResult<Py<NotebookHtml>> {
    let options = parse_options(math_target, note_tag_icons)?;
    let notebook = py
        .detach(|| structured::convert_package_buffer(data, file_name, options))
        .map_err(|e| PyRuntimeError::new_err(format!("{e:?}")))?;

    let sections = notebook
        .sections
        .into_iter()
        .map(|s| section_obj(py, s))
        .collect::<PyResult<Vec<_>>>()?;

    Py::new(
        py,
        NotebookHtml {
            display_name: notebook.display_name,
            sections,
            warnings: notebook.warnings,
        },
    )
}

#[pymodule(name = "py_one2html")]
fn py_one2html(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PageHtml>()?;
    m.add_class::<SectionHtml>()?;
    m.add_class::<NotebookHtml>()?;
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    m.add_function(wrap_pyfunction!(parse_section, m)?)?;
    m.add_function(wrap_pyfunction!(parse_package, m)?)?;
    Ok(())
}
