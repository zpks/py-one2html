use crate::page::Renderer;
use crate::utils::{AttributeSet, StyleSet, px};
use color_eyre::Result;
use color_eyre::eyre::WrapErr;
use itertools::Itertools;
use log::warn;
use once_cell::sync::Lazy;
use onenote_parser::FileSystem;
use onenote_parser::contents::{EmbeddedObject, MathInlineObject, RichText};
use onenote_parser::property::common::ColorRef;
use onenote_parser::property::rich_text::{ParagraphAlignment, ParagraphStyling};
use regex::{Captures, Regex};
use std::iter::repeat;

impl<'a, FS: FileSystem> Renderer<'a, FS> {
    pub(crate) fn render_rich_text(&mut self, text: &RichText) -> Result<String> {
        let mut content = String::new();
        let mut attrs = AttributeSet::new();
        let mut style = self.parse_paragraph_styles(text);

        if let Some((note_tag_html, note_tag_styles)) = self.render_note_tags(text.note_tags()) {
            content.push_str(&note_tag_html);
            style.extend(note_tag_styles);
        }

        content.push_str(&self.parse_content(text)?);

        if content.starts_with("http://") || content.starts_with("https://") {
            content = format!("<a href=\"{}\">{}</a>", content, content);
        }

        if style.len() > 0 {
            attrs.set("style", style.to_string());
        }

        match text.paragraph_style().style_id() {
            Some(t) if !self.in_list && is_tag(t) => {
                Ok(format!("<{} {}>{}</{}>", t, attrs, content, t))
            }
            _ if style.len() > 0 => Ok(format!("<span style=\"{}\">{}</span>", style, content)),
            _ => Ok(content),
        }
    }

    fn parse_content(&mut self, data: &RichText) -> Result<String> {
        if !data.embedded_objects().is_empty() {
            return Ok(data
                .embedded_objects()
                .iter()
                .map(|object| match object {
                    EmbeddedObject::Ink(container) => {
                        self.render_ink(container.ink(), container.bounding_box(), true)
                    }
                    EmbeddedObject::InkSpace(space) => {
                        format!("<span class=\"ink-space\" style=\"padding-left: {}; padding-top: {};\"></span>",
                                px(space.width()), px(space.height()))
                    }
                    EmbeddedObject::InkLineBreak => {
                        "<span class=\"ink-linebreak\"><br></span>".to_string()
                    }
                })
                .collect_vec()
                .join(""));
        }

        let indices = data.text_run_indices();
        let styles = data.text_run_formatting();

        if indices.len() > styles.len() {
            warn!(
                "Some text runs have no corresponding styles: {:?} vs {:?}",
                indices, styles
            );
        }

        let mut text = data.text().to_string();

        if text.is_empty() {
            text = "&nbsp;".to_string();
        }

        let parts = if !indices.is_empty() {
            self.split_by_indices(indices, text)?
        } else {
            vec![text]
        };

        // Render text run styles
        let content = self.render_text_run_styles(styles, parts)?;

        // Render math groups
        let content = self.render_math_text_runs(data, styles, content)?;

        Ok(fix_newlines(content))
    }

    fn render_math_text_runs(
        &mut self,
        data: &RichText,
        styles: &[ParagraphStyling],
        content: Vec<String>,
    ) -> Result<String> {
        let math_groups = content
            .into_iter()
            .zip(styles.iter())
            .chunk_by(|(_text, style)| style.math_formatting());

        let mut math_object_offset = 0;

        let contents = (&math_groups)
            .into_iter()
            .map(|(is_math, group)| {
                let group_parts = group.collect_vec();
                let text = group_parts.iter().map(|(text, _)| text).join("");

                if !is_math {
                    return Ok(text);
                }

                let inline_objects = data.math_inline_objects();

                if math_object_offset >= inline_objects.len() {
                    let segment = (text, MathInlineObject::default());
                    return self.render_math(vec![segment]);
                }

                let count = group_parts.len();
                let objects = inline_objects[math_object_offset..math_object_offset + count]
                    .iter()
                    .copied();
                let segments = group_parts
                    .into_iter()
                    .map(|(text, _)| text)
                    .zip(objects)
                    .collect_vec();

                let text = self.render_math(segments)?;
                math_object_offset += count;

                Ok(text)
            })
            .collect::<Result<Vec<_>>>()?
            .join("");

        Ok(contents)
    }

    fn render_text_run_styles(
        &mut self,
        styles: &[ParagraphStyling],
        parts: Vec<String>,
    ) -> Result<Vec<String>> {
        // Inline hyperlinks are encoded as a hidden marker run followed by the
        // visible display run. We track the URL extracted from the marker and
        // hand it to the next HyperlinkProtected run.
        //
        //   marker run:  Hidden=T, HyperlinkProtected=T (§2.3.76 + §2.3.77)
        //                text = "\u{fddf}HYPERLINK \"URL\""  ← OneNote-internal
        //                                                      encoding; spec
        //                                                      defines no
        //                                                      out-of-band URL
        //                                                      field for inline
        //                                                      hyperlinks
        //   display run: HyperlinkProtected=T, Hidden=F (§2.3.77)
        //                text = visible link text, styled normally
        let mut pending_url: Option<String> = None;

        parts
            .into_iter()
            .rev()
            .zip(styles.iter().map(Some).chain(repeat(None)))
            .map(|(text, style)| -> Result<String> {
                let style = match style {
                    Some(style) => style,
                    None => return Ok(html_escape(&text)),
                };

                if style.hidden() {
                    pending_url = extract_hyperlink_url(&text);
                    return Ok(String::new());
                }

                if style.hyperlink_protected() {
                    let parsed_style = self.parse_style(style);
                    let escaped = html_escape(&text);
                    return Ok(match pending_url.take() {
                        Some(url) => format!(
                            "<a href=\"{}\" style=\"{}\">{}</a>",
                            url, parsed_style, escaped
                        ),
                        None => {
                            warn!(
                                "Hyperlink display run with no preceding URL marker: {:?}",
                                text
                            );
                            render_styled_span(parsed_style, escaped)
                        }
                    });
                }

                // A plain run resets any unconsumed marker URL — keeps state
                // from leaking across an unexpected gap.
                pending_url = None;

                // Bare-URL run: wrap in `<a>` directly so OneNote's `<URL>`
                // citation pattern (split across runs as `From <`, URL, `>`)
                // produces `From &lt;<a>URL</a>&gt;`. Mirrors the
                // hyperlink_protected formatting (early return, no extra span).
                if !style.math_formatting()
                    && (text.starts_with("http://") || text.starts_with("https://"))
                {
                    let parsed_style = self.parse_style(style);
                    let url = text.trim_end();
                    let trailing = &text[url.len()..];
                    return Ok(format!(
                        "<a href=\"{}\" style=\"{}\">{}</a>{}",
                        url,
                        parsed_style,
                        html_escape(url),
                        html_escape(trailing)
                    ));
                }

                // Math runs feed the math parser, which tokenises raw text
                // (and treats `&` as an alignment marker), so leave them
                // unescaped. Everything else is HTML-escaped, with any
                // single-run `<URL>` citation pattern auto-linked.
                let text = if style.math_formatting() {
                    text
                } else {
                    autolink_angle_url(&html_escape(&text))
                };

                Ok(render_styled_span(self.parse_style(style), text))
            })
            .collect::<Result<Vec<String>>>()
    }

    fn split_by_indices(&self, indices: &[u32], text: String) -> Result<Vec<String>> {
        // Split text into parts specified by indices
        let mut parts = vec![];

        let mut text = text.encode_utf16().collect::<Vec<u16>>();

        for i in indices.iter().copied().rev() {
            let part = text[i as usize..].to_vec();
            text = text[0..i as usize].to_vec();

            parts.push(part);
        }

        if !indices.is_empty() {
            parts.push(text);
        }

        parts
            .into_iter()
            .map(|text| String::from_utf16(&text).wrap_err("Failed to parse rich text contents"))
            .collect::<Result<Vec<_>>>()
    }


    fn parse_paragraph_styles(&self, text: &RichText) -> StyleSet {
        if !text.embedded_objects().is_empty() {
            assert_eq!(
                text.text(),
                "",
                "paragraph with text and embedded objects is not supported"
            );

            return StyleSet::new();
        }

        let mut styles = self.parse_style(text.paragraph_style());

        if let [style] = text.text_run_formatting() {
            styles.extend(self.parse_style(style))
        }

        if text.paragraph_space_before() > 0.0 {
            styles.set("padding-top", px(text.paragraph_space_before()))
        }

        if text.paragraph_space_after() > 0.0 {
            styles.set("padding-bottom", px(text.paragraph_space_after()))
        }

        if let Some(line_spacing) = text.paragraph_line_spacing_exact()
            && line_spacing > 0.0
        {
            warn!(
                "Paragraph exact line spacing not implemented; ignoring value {}",
                line_spacing
            );
        }

        match text.paragraph_alignment() {
            ParagraphAlignment::Center => styles.set("text-align", "center".to_string()),
            ParagraphAlignment::Right => styles.set("text-align", "right".to_string()),
            _ => {}
        }

        styles
    }

    fn parse_style(&self, style: &ParagraphStyling) -> StyleSet {
        let mut styles = StyleSet::new();

        if style.math_formatting() {
            return styles;
        }

        if style.bold() {
            styles.set("font-weight", "bold".to_string());
        }

        if style.italic() {
            styles.set("font-style", "italic".to_string());
        }

        if style.underline() {
            styles.set("text-decoration", "underline".to_string());
        }

        if style.superscript() {
            styles.set("vertical-align", "super".to_string());
        }

        if style.subscript() {
            styles.set("vertical-align", "sub".to_string());
        }

        if style.strikethrough() {
            styles.set("text-decoration", "line-through".to_string());
        }

        if let Some(font) = style.font() {
            styles.set("font-family", font.to_string());
        }

        if let Some(size) = style.font_size() {
            styles.set("font-size", ((size as f32) / 2.0).to_string() + "pt");
        }

        if let Some(ColorRef::Manual { r, g, b }) = style.font_color() {
            styles.set("color", format!("rgb({},{},{})", r, g, b));
        }

        if let Some(ColorRef::Manual { r, g, b }) = style.highlight() {
            styles.set("background-color", format!("rgb({},{},{})", r, g, b));
        }

        if style.paragraph_alignment().is_some() {
            warn!("Paragraph alignment in text run style not implemented; ignoring");
        }

        if let Some(space) = style.paragraph_space_before()
            && space != 0.0
        {
            warn!(
                "Paragraph space-before in text run style not implemented; ignoring value {}",
                space
            );
        }

        if let Some(space) = style.paragraph_space_after()
            && space != 0.0
        {
            warn!(
                "Paragraph space-after in text run style not implemented; ignoring value {}",
                space
            );
        }

        if let Some(space) = style.paragraph_line_spacing_exact() {
            if space != 0.0 {
                warn!(
                    "Paragraph exact line spacing in text run style not implemented; ignoring value {}",
                    space
                );
            }

            if let Some(size) = style.font_size() {
                styles.set(
                    "line-height",
                    format!("{}px", (size as f32 * 1.2 / 72.0 * 48.0).ceil()),
                )
            }
        }

        // if style.math_formatting() {
        //     // FIXME: Handle math formatting
        //     // See https://docs.microsoft.com/en-us/windows/win32/api/richedit/ns-richedit-gettextex
        //     // for unicode chars used
        //     unimplemented!()
        // }

        styles
    }
}

fn is_tag(tag: &str) -> bool {
    !matches!(tag, "PageDateTime" | "PageTitle")
}

fn extract_hyperlink_url(text: &str) -> Option<String> {
    const HYPERLINK_MARKER: &str = "\u{fddf}HYPERLINK \"";

    text.strip_prefix(HYPERLINK_MARKER)
        .and_then(|s| s.strip_suffix('"'))
        .map(str::to_owned)
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Turn the OneNote `<URL>` citation pattern into a clickable link, operating
/// on text that has already been HTML-escaped (so the angle brackets are
/// `&lt;`/`&gt;` and any `&` inside the URL is `&amp;`).
fn autolink_angle_url(escaped: &str) -> String {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"&lt;(https?://\S+?)&gt;").expect("invalid auto-link regex")
    });

    RE.replace_all(escaped, |caps: &Captures| {
        format!("&lt;<a href=\"{0}\">{0}</a>&gt;", &caps[1])
    })
    .into_owned()
}

fn render_styled_span(style: StyleSet, text: String) -> String {
    if style.len() > 0 {
        format!("<span style=\"{}\">{}</span>", style, text)
    } else {
        text
    }
}

fn fix_newlines(text: String) -> String {
    static REGEX_LEADING_SPACES: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"<br>(\s+)").expect("failed to compile regex"));

    let text = text
        .replace("\u{000b}", "<br>")
        .replace("\n", "<br>")
        .replace("\r", "<br>");

    REGEX_LEADING_SPACES
        .replace_all(&text, |captures: &Captures| {
            "<br>".to_string() + &"&nbsp;".repeat(captures[1].len())
        })
        .to_string()
}
