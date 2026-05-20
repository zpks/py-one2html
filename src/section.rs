use crate::utils::sanitize_output_filename;
use crate::{Options, page, templates};
use color_eyre::eyre::Result;
use onenote_parser::FileSystem;
use onenote_parser::section::Section;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(crate) struct Renderer {
    pub(crate) files: HashSet<String>,
    pub(crate) pages: HashSet<String>,
}

impl Renderer {
    pub fn new() -> Self {
        Renderer {
            files: Default::default(),
            pages: Default::default(),
        }
    }

    pub fn render(
        &mut self,
        section: &Section,
        output_dir: &Path,
        options: Options,
        fs: impl FileSystem,
    ) -> Result<PathBuf> {
        let section_dir = output_dir.join(sanitize_filename::sanitize(section.display_name()));

        fs.make_dir(section_dir.as_path())?;

        let mut entries: Vec<templates::section::Entry> = Vec::new();
        let mut fallback_title_index = 0;

        for page_series in section.page_series() {
            for page in page_series.pages() {
                let title = page.title_text().map(|s| s.to_string()).unwrap_or_else(|| {
                    fallback_title_index += 1;

                    format!("Untitled Page {}", fallback_title_index)
                });

                let file_name = title.trim().replace("/", "_");
                let file_name = self.determine_page_filename(&file_name)?;
                let file_name = sanitize_filename::sanitize(&(file_name + ".html"));

                let output_file = section_dir.join(file_name);

                let mut renderer = page::Renderer::new(section_dir.clone(), self, options, fs);
                let output = renderer.render_page(page)?;

                fs.write_file(output_file.as_path(), output.as_bytes())?;

                entries.push(templates::section::Entry {
                    name: title,
                    path: output_file
                        .strip_prefix(output_dir)?
                        .to_string_lossy()
                        .to_string(),
                    level: page.level(),
                    is_warnings: false,
                });
            }
        }

        let warnings = section.report().warnings();
        if options.warnings && !warnings.is_empty() {
            let stem = self.determine_page_filename("Warnings")?;
            let filename = sanitize_filename::sanitize(&(stem + ".html"));
            let warnings_path = section_dir.join(&filename);

            let warning_entries: Vec<templates::warnings::Entry> = warnings
                .iter()
                .map(|w| templates::warnings::Entry {
                    page: w
                        .page()
                        .map(|(_, title)| title)
                        .unwrap_or("(section-level)"),
                    message: w.message(),
                })
                .collect();

            let html = templates::warnings::render(section.display_name(), &warning_entries)?;
            fs.write_file(warnings_path.as_path(), html.as_bytes())?;

            entries.push(templates::section::Entry {
                name: "\u{26A0} Conversion Warnings".to_string(),
                path: warnings_path
                    .strip_prefix(output_dir)?
                    .to_string_lossy()
                    .to_string(),
                level: 0,
                is_warnings: true,
            });
        }

        let toc_html = templates::section::render(section.display_name(), &entries)?;
        let toc_name = sanitize_output_filename(section.display_name())? + ".html";
        let toc_file = output_dir.join(toc_name);

        fs.write_file(toc_file.as_path(), toc_html.as_bytes())?;

        Ok(section_dir)
    }

    pub(crate) fn determine_page_filename(&mut self, filename: &str) -> Result<String> {
        let mut i = 0;
        let mut current_filename = sanitize_filename::sanitize(filename);

        loop {
            if !self.pages.contains(&current_filename) {
                self.pages.insert(current_filename.clone());

                return Ok(current_filename);
            }

            i += 1;

            current_filename = format!("{}_{}", filename, i);
        }
    }
}
