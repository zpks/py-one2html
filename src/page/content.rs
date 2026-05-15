use crate::page::Renderer;
use crate::page::ink::InkBuilder;
use color_eyre::Result;
use log::warn;
use onenote_parser::FileSystem;
use onenote_parser::contents::Content;

impl<'a, FS: FileSystem> Renderer<'a, FS> {
    pub(crate) fn render_contents(&mut self, contents: &[Content]) -> Result<String> {
        let mut result = vec![];
        let mut ink_builder = InkBuilder::new(false);

        for content in contents {
            if !matches!(content, Content::Ink(_)) {
                result.push(ink_builder.finish());
            }

            match content {
                Content::RichText(text) => {
                    result.push(self.render_rich_text(text)?);
                }
                Content::Image(image) => {
                    result.push(self.render_image(image)?);
                }
                Content::EmbeddedFile(file) => {
                    result.push(self.render_embedded_file(file)?);
                }
                Content::Table(table) => {
                    result.push(self.render_table(table)?);
                }
                Content::Ink(ink) => {
                    ink_builder.push(ink, None);
                }
                Content::Unknown => {
                    warn!("Page with unknown content");
                }
            }
        }

        result.push(ink_builder.finish());
        Ok(result.join(""))
    }
}
