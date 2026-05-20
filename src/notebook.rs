use crate::templates::notebook::Toc;
use crate::utils::sanitize_output_filename;
use crate::{Options, section, templates};
use color_eyre::eyre::Result;
use onenote_parser::FileSystem;
use onenote_parser::notebook::Notebook;
use onenote_parser::property::common::Color;
use onenote_parser::section::{Section, SectionEntry};
use palette::rgb::Rgb;
use palette::{Alpha, Darken, FromColor, Hsl, Saturate, Srgb};
use std::path::Path;

pub(crate) type RgbColor = Alpha<Rgb<palette::encoding::Srgb, u8>, f32>;

pub(crate) struct Renderer;

impl Renderer {
    pub fn new() -> Self {
        Renderer
    }

    pub fn render(
        &mut self,
        notebook: &Notebook,
        name: &str,
        options: Options,
        output_dir: &Path,
        fs: impl FileSystem,
    ) -> Result<()> {
        fs.make_dir(output_dir)?;

        let notebook_dir = output_dir.join(sanitize_filename::sanitize(name));

        fs.make_dir(notebook_dir.as_path())?;

        let mut toc = Vec::new();

        for entry in notebook.entries() {
            self.walk_entry(entry, &notebook_dir, output_dir, 0, options, fs, &mut toc)?;
        }

        let toc_html = templates::notebook::render(name, &toc)?;
        let toc_name = sanitize_output_filename(name, fs)? + ".html";
        let toc_file = output_dir.join(toc_name);

        fs.write_file(toc_file.as_path(), toc_html.as_bytes())?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn walk_entry(
        &mut self,
        entry: &SectionEntry,
        parent_dir: &Path,
        base_dir: &Path,
        depth: u32,
        options: Options,
        fs: impl FileSystem,
        toc: &mut Vec<Toc>,
    ) -> Result<()> {
        match entry {
            SectionEntry::Section(section) => {
                let rendered = self.render_section(section, parent_dir, base_dir, options, fs)?;
                toc.push(Toc::Section {
                    section: rendered,
                    depth,
                });
            }
            SectionEntry::SectionGroup(group) => {
                let group_dir = parent_dir.join(sanitize_filename::sanitize(group.display_name()));
                fs.make_dir(group_dir.as_path())?;

                toc.push(Toc::GroupHeader {
                    name: group.display_name().to_string(),
                    depth,
                });

                for child in group.entries() {
                    self.walk_entry(child, &group_dir, base_dir, depth + 1, options, fs, toc)?;
                }
            }
        }
        Ok(())
    }

    fn render_section(
        &mut self,
        section: &Section,
        notebook_dir: &Path,
        base_dir: &Path,
        options: Options,
        fs: impl FileSystem,
    ) -> Result<templates::notebook::Section> {
        let mut renderer = section::Renderer::new();
        let path = renderer.render(section, notebook_dir, options, fs)?;

        Ok(templates::notebook::Section {
            name: section.display_name().to_string(),
            path: path.strip_prefix(base_dir)?.to_string_lossy().to_string(),
            color: section.color().map(prepare_color),
        })
    }
}

fn prepare_color(color: Color) -> RgbColor {
    Alpha {
        alpha: color.alpha() as f32 / 255.0,
        color: Srgb::from_color(
            Hsl::from_color(Srgb::new(
                color.r() as f32 / 255.0,
                color.g() as f32 / 255.0,
                color.b() as f32 / 255.0,
            ))
            .darken(0.2)
            .saturate(1.0),
        )
        .into_format(),
    }
}
