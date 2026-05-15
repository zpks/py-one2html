use crate::page::Renderer;
use crate::utils::sanitize_output_filename;
use crate::utils::{AttributeSet, StyleSet, detect_png, px};
use color_eyre::Result;
use color_eyre::eyre::WrapErr;
use onenote_parser::FileSystem;
use onenote_parser::contents::Image;

impl<'a, FS: FileSystem> Renderer<'a, FS> {
    pub(crate) fn render_image(&mut self, image: &Image) -> Result<String> {
        let mut content = String::new();

        if let Some(data) = image.data() {
            let filename = self.determine_image_filename(image, data)?;

            let target_file = self.output.join(filename.clone());

            self.fs
                .write_file(target_file.as_path(), data)
                .wrap_err("Failed to write image")?;

            let mut attrs = AttributeSet::new();
            let mut styles = StyleSet::new();

            attrs.set("src", filename);

            if let Some(text) = image.alt_text() {
                attrs.set("alt", text.to_string().replace('"', "&quot;"));
            }

            if let Some(width) = image.layout_max_width() {
                styles.set("max-width", px(width));
            }

            if let Some(height) = image.layout_max_height() {
                styles.set("max-height", px(height));
            }

            if image.offset_horizontal().is_some() || image.offset_vertical().is_some() {
                styles.set("position", "absolute".to_string());
            }

            if let Some(offset) = image.offset_horizontal() {
                styles.set("left", px(offset));
            }

            if let Some(offset) = image.offset_vertical() {
                styles.set("top", px(offset));
            }

            if styles.len() > 0 {
                attrs.set("style", styles.to_string());
            }

            content.push_str(&format!("<img {} />", attrs));
        }

        Ok(self.render_with_note_tags(image.note_tags(), content))
    }

    fn determine_image_filename(&mut self, image: &Image, initial_bytes: &[u8]) -> Result<String> {
        if let Some(name) = image.image_filename() {
            // Workaround: PDF printout pages are PNG images, but have an image_filename
            // with extension .PDF. Add a PNG extension to these files so that they are
            // imported properly.
            let is_pdf = std::path::Path::new(name)
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("pdf"))
                .unwrap_or(false);
            let owned;
            let name: &str = if is_pdf && detect_png(initial_bytes) {
                owned = format!("{name}.png");
                &owned
            } else {
                name
            };

            let sanitized = sanitize_output_filename(name)?;
            return self.determine_filename(&sanitized);
        }

        if let Some(ext) = image.extension() {
            let mut i = 0;

            loop {
                let filename = format!("image{}{}", i, ext);

                if !self.section.files.contains(&filename) {
                    self.section.files.insert(filename.clone());

                    return Ok(filename);
                }

                i += 1;
            }
        }

        let mut i = 0;

        loop {
            let filename = format!("image{}", i);

            if !self.section.files.contains(&filename) {
                self.section.files.insert(filename.clone());

                return Ok(filename);
            }

            i += 1;
        }
    }
}
