#![deny(clippy::disallowed_methods, clippy::disallowed_types)]

use crate::utils::with_progress;
use color_eyre::eyre::{ContextCompat, Result, eyre};
use onenote_parser::{FileSystem, Parser};
use std::path::Path;

mod notebook;
mod page;
mod section;
mod templates;
mod utils;

pub fn convert(path: &Path, output_dir: &Path, fs: impl FileSystem) -> Result<()> {
    let parser = Parser::new_with_fs(fs);

    match path.extension().map(|p| p.to_string_lossy()).as_deref() {
        Some("one") => {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            log::info!("Processing section {}...", name);

            let section = with_progress("Parsing input file...", || parser.parse_section(path))?;

            section::Renderer::new().render(&section, output_dir, fs)?;
        }
        Some("onetoc2") => {
            let name = path
                .parent()
                .unwrap()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            log::info!("Processing notebook {}...", name);

            let notebook = with_progress("[1/2] Parsing input files...", || {
                parser.parse_notebook(path)
            })?;

            let notebook_name = path
                .parent()
                .wrap_err("Input file has no parent folder")?
                .file_name()
                .wrap_err("Parent folder has no name")?
                .to_string_lossy();

            with_progress("[2/2] Rendering sections...", || {
                notebook::Renderer::new().render(&notebook, &notebook_name, output_dir, fs)
            })?;
        }
        Some(ext) => return Err(eyre!("Invalid file extension: {}", ext)),
        _ => return Err(eyre!("Couldn't determine file type")),
    };

    Ok(())
}
