use crate::page::Renderer;
use crate::utils::{html_entities, sanitize_output_filename};
use color_eyre::Result;
use color_eyre::eyre::WrapErr;
use onenote_parser::FileSystem;
use onenote_parser::contents::EmbeddedFile;
use onenote_parser::property::embedded_file::FileType;
use std::path::PathBuf;

impl<'a, FS: FileSystem> Renderer<'a, FS> {
    pub(crate) fn render_embedded_file(&mut self, file: &EmbeddedFile) -> Result<String> {
        let filename = self.determine_filename(file.filename())?;

        let target_file = self.output.join(filename.clone());

        let mut reader = file.read();
        self.fs
            .stream_to_file(target_file.as_path(), &mut *reader)
            .wrap_err("Failed to write embedded file")?;

        let file_type = Self::guess_type(file);

        let src = html_entities(&filename);
        let content = match file_type {
            FileType::Audio => format!("<audio controls src=\"{}\"></audio>", src),
            FileType::Video => format!("<video controls src=\"{}\"></video>", src),
            FileType::Unknown => format!("<embed src=\"{}\" />", src),
        };

        Ok(self.render_with_note_tags(file.note_tags(), content))
    }

    fn guess_type(file: &EmbeddedFile) -> FileType {
        match file.file_type() {
            FileType::Audio => return FileType::Audio,
            FileType::Video => return FileType::Video,
            _ => {}
        };

        let filename = file.filename();

        if let Some(mime) = mime_guess::from_path(filename).first() {
            if mime.type_() == "audio" {
                return FileType::Audio;
            }

            if mime.type_() == "video" {
                return FileType::Video;
            }
        }
        FileType::Unknown
    }

    pub(crate) fn determine_filename(&mut self, filename: &str) -> Result<String> {
        let mut i = 0;
        let sanitized = sanitize_output_filename(filename, self.fs)?;
        let mut current_filename = sanitized.clone();

        loop {
            if !self.section.files.contains(&current_filename) {
                self.section.files.insert(current_filename.clone());

                return Ok(current_filename);
            }

            let path = PathBuf::from(&sanitized);

            let ext = path
                .extension()
                .unwrap_or("bin".as_ref())
                .to_string_lossy()
                .to_string();

            let path_str = path.as_os_str().to_string_lossy();

            let base = path_str
                .strip_suffix(&ext)
                .map(|s| s.trim_matches('.'))
                .unwrap_or(path_str.as_ref())
                .to_string();

            current_filename = format!("{}-{}.{}", base, i, ext);

            i += 1;
        }
    }
}
