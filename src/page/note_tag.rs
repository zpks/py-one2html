use crate::NoteTagIcons;
use crate::page::Renderer;
use crate::utils::{AttributeSet, StyleSet};
use log::warn;
use onenote_parser::FileSystem;
use onenote_parser::contents::{NoteTag, OutlineElement};
use onenote_parser::property::common::ColorRef;
use onenote_parser::property::note_tag::{ActionItemStatus, NoteTagShape};
use std::borrow::Cow;

const COLOR_BLUE: &str = "#4673b7";
const COLOR_GREEN: &str = "#369950";
const COLOR_ORANGE: &str = "#dba24d";
const COLOR_PINK: &str = "#f78b9d";
const COLOR_RED: &str = "#db5b4d";
const COLOR_YELLOW: &str = "#ffd678";

const SVG_ARROW_RIGHT: &str = include_str!("../../assets/icons/arrow-right-line.svg");
const SVG_AWARD: &str = include_str!("../../assets/icons/award-line.svg");
const SVG_BOOK: &str = include_str!("../../assets/icons/book-open-line.svg");
const SVG_BUBBLE: &str = include_str!("../../assets/icons/chat-4-line.svg");
const SVG_CHECKBOX_COMPLETE: &str = include_str!("../../assets/icons/checkbox-fill.svg");
const SVG_CHECKBOX_EMPTY: &str = include_str!("../../assets/icons/checkbox-blank-line.svg");
const SVG_CHECK_MARK: &str = include_str!("../../assets/icons/check-line.svg");
const SVG_CIRCLE: &str = include_str!("../../assets/icons/checkbox-blank-circle-fill.svg");
const SVG_CONTACT: &str = include_str!("../../assets/icons/contacts-line.svg");
const SVG_EMAIL: &str = include_str!("../../assets/icons/send-plane-2-line.svg");
const SVG_ERROR: &str = include_str!("../../assets/icons/error-warning-line.svg");
const SVG_FILM: &str = include_str!("../../assets/icons/film-line.svg");
const SVG_FLAG: &str = include_str!("../../assets/icons/flag-fill.svg");
const SVG_HOME: &str = include_str!("../../assets/icons/home-4-line.svg");
const SVG_LIGHT_BULB: &str = include_str!("../../assets/icons/lightbulb-line.svg");
const SVG_LINK: &str = include_str!("../../assets/icons/link.svg");
const SVG_LOCK: &str = include_str!("../../assets/icons/lock-line.svg");
const SVG_MUSIC: &str = include_str!("../../assets/icons/music-fill.svg");
const SVG_PAPER: &str = include_str!("../../assets/icons/file-list-2-line.svg");
const SVG_PEN: &str = include_str!("../../assets/icons/mark-pen-line.svg");
const SVG_PERSON: &str = include_str!("../../assets/icons/user-line.svg");
const SVG_PHONE: &str = include_str!("../../assets/icons/phone-line.svg");
const SVG_QUESTION_MARK: &str = include_str!("../../assets/icons/question-mark.svg");
const SVG_SQUARE: &str = include_str!("../../assets/icons/checkbox-blank-fill.svg");
const SVG_STAR: &str = include_str!("../../assets/icons/star-fill.svg");

const EMOJI_ARROW_RIGHT: &str = "→";
const EMOJI_AWARD: &str = "🎖️";
const EMOJI_BOOK: &str = "📖";
const EMOJI_BUBBLE: &str = "🗨️";
const EMOJI_CHECKBOX_COMPLETE: &str = "☑";
const EMOJI_CHECKBOX_EMPTY: &str = "☐";
const EMOJI_CHECK_MARK: &str = "✓";
const EMOJI_CIRCLE_BLUE: &str = "🔵";
const EMOJI_CIRCLE_GREEN: &str = "🟢";
const EMOJI_CIRCLE_ORANGE: &str = "🟠";
const EMOJI_CONTACT: &str = "👥";
const EMOJI_EMAIL: &str = "📨";
const EMOJI_ERROR: &str = "❗";
const EMOJI_FILM: &str = "🎞️";
const EMOJI_FLAG: &str = "🚩";
const EMOJI_HOME: &str = "🏠";
const EMOJI_LIGHT_BULB: &str = "💡";
const EMOJI_LINK: &str = "🔗";
const EMOJI_LOCK: &str = "🔒";
const EMOJI_MUSIC: &str = "🎵";
const EMOJI_PAPER: &str = "📄";
const EMOJI_PEN: &str = "🖊️";
const EMOJI_PERSON: &str = "👤";
const EMOJI_PHONE: &str = "📞";
const EMOJI_QUESTION_MARK: &str = "❓";
const EMOJI_SQUARE_RED: &str = "🟥";
const EMOJI_SQUARE_YELLOW: &str = "🟨";
const EMOJI_SQUARE_ORANGE: &str = "🟧";
const EMOJI_SQUARE_GREEN: &str = "🟩";
const EMOJI_SQUARE_BLUE: &str = "🟦";
const EMOJI_SQUARE_PURPLE: &str = "🟪";
const EMOJI_STAR: &str = "🟊";
const EMOJI_YELLOW_STAR: &str = "⭐";

#[derive(Debug, Copy, Clone, PartialEq)]
enum IconSize {
    Normal,
    Large,
}

struct NoteTagIcon {
    html: Cow<'static, str>,
    size: IconSize,
    styles: StyleSet,
    is_checkbox: bool,
}

impl From<(Cow<'static, str>, IconSize)> for NoteTagIcon {
    fn from((html, size): (Cow<'static, str>, IconSize)) -> Self {
        Self {
            html,
            size,
            styles: StyleSet::new(),
            is_checkbox: false,
        }
    }
}

impl From<(Cow<'static, str>, IconSize, StyleSet)> for NoteTagIcon {
    fn from((html, size, styles): (Cow<'static, str>, IconSize, StyleSet)) -> Self {
        Self {
            html,
            size,
            styles,
            is_checkbox: false,
        }
    }
}

impl<'a, FS: FileSystem> Renderer<'a, FS> {
    pub(crate) fn render_with_note_tags(
        &mut self,
        note_tags: &[NoteTag],
        content: String,
    ) -> String {
        if let Some((markup, styles)) = self.render_note_tags(note_tags) {
            let mut contents = String::new();
            contents.push_str(&format!("<div {}>{}", styles.to_html_attr(), markup));
            contents.push_str(&content);
            contents.push_str("</div>");

            contents
        } else {
            content
        }
    }

    pub(crate) fn render_note_tags(&mut self, note_tags: &[NoteTag]) -> Option<(String, StyleSet)> {
        let mut markup = String::new();
        let mut styles = StyleSet::new();

        if note_tags.is_empty() {
            return None;
        }

        for note_tag in note_tags {
            if let Some(def) = note_tag.definition() {
                if let Some(ColorRef::Manual { r, g, b }) = def.highlight_color() {
                    styles.set("background-color", format!("rgb({},{},{})", r, g, b));
                }

                if let Some(ColorRef::Manual { r, g, b }) = def.text_color() {
                    styles.set("color", format!("rgb({},{},{})", r, g, b));
                }

                if def.shape() != NoteTagShape::NoIcon {
                    let icon = self.note_tag_icon(def.shape(), note_tag.item_status());
                    let icon_classes = self.build_note_tag_class_names(&icon);
                    let attrs =
                        self.get_note_tag_attrs(&icon, note_tag.item_status(), &icon_classes);

                    if self.use_emoji() {
                        // Wrapping in `.text` keeps the emoji's color/size CSS hooks
                        // independent of the outer note-tag span.
                        markup.push_str(&format!(
                            "<span {}><span class=\"text\">{}</span></span>",
                            attrs, icon.html
                        ));
                    } else {
                        markup.push_str(&format!("<span {}>{}</span>", attrs, icon.html));
                    }
                }
            }
        }

        Some((markup, styles))
    }

    fn build_note_tag_class_names(&mut self, icon: &NoteTagIcon) -> Vec<String> {
        let mut icon_classes = vec!["note-tag-icon".to_string()];

        if icon.styles.len() > 0 {
            let class = self.gen_class("icon");
            icon_classes.push(class.to_string());

            let selector = if self.use_emoji() {
                format!(".{} > .text", class)
            } else {
                // SVGs may be replaced with `img` by downstream consumers; cover both.
                format!(".{} > svg, .{} > img", class, class)
            };
            self.global_styles.insert(selector, icon.styles.clone());
        }

        if icon.is_checkbox {
            icon_classes.push("-checkbox".into());
        }

        if icon.size == IconSize::Large {
            icon_classes.push("-large".into());
        } else if icon.size == IconSize::Normal {
            icon_classes.push("-normal".into());
        }

        icon_classes
    }

    fn get_note_tag_attrs(
        &mut self,
        icon: &NoteTagIcon,
        status: ActionItemStatus,
        class_names: &[String],
    ) -> AttributeSet {
        let mut attrs = AttributeSet::new();
        attrs.set("class", class_names.join(" "));

        if icon.is_checkbox {
            attrs.set("role", "checkbox".into());
            attrs.set(
                "aria-checked",
                if status.completed() { "true" } else { "false" }.into(),
            );
            attrs.set("aria-disabled", "true".into());
        }

        attrs
    }

    pub(crate) fn has_note_tag(&self, element: &OutlineElement) -> bool {
        element
            .contents()
            .iter()
            .flat_map(|element| element.rich_text())
            .any(|text| !text.note_tags().is_empty())
    }

    fn use_emoji(&self) -> bool {
        self.options.note_tag_icons == NoteTagIcons::Emoji
    }

    fn pick(&self, svg: &'static str, emoji: &'static str) -> &'static str {
        if self.use_emoji() { emoji } else { svg }
    }

    fn note_tag_icon(&self, shape: NoteTagShape, status: ActionItemStatus) -> NoteTagIcon {
        match shape {
            NoteTagShape::GreenCheckBox => self.icon_checkbox(status, COLOR_GREEN),
            NoteTagShape::YellowCheckBox => self.icon_checkbox(status, COLOR_YELLOW),
            NoteTagShape::BlueCheckBox => self.icon_checkbox(status, COLOR_BLUE),
            NoteTagShape::GreenStarCheckBox => self.icon_checkbox_with_star(status, COLOR_GREEN),
            NoteTagShape::YellowStarCheckBox => {
                // OneNote shows this as a filled gold star, which the regular ICON_STAR
                // (a hollow outline) doesn't convey in emoji form.
                if self.use_emoji() {
                    self.icon_checkbox_with(status, COLOR_YELLOW, EMOJI_YELLOW_STAR)
                } else {
                    self.icon_checkbox_with_star(status, COLOR_YELLOW)
                }
            }
            NoteTagShape::BlueStarCheckBox => self.icon_checkbox_with_star(status, COLOR_BLUE),
            NoteTagShape::GreenExclamationCheckBox => {
                self.icon_checkbox_with_exclamation(status, COLOR_GREEN)
            }
            NoteTagShape::YellowExclamationCheckBox => {
                self.icon_checkbox_with_exclamation(status, COLOR_YELLOW)
            }
            NoteTagShape::BlueExclamationCheckBox => {
                self.icon_checkbox_with_exclamation(status, COLOR_BLUE)
            }
            NoteTagShape::GreenRightArrowCheckBox => {
                self.icon_checkbox_with_right_arrow(status, COLOR_GREEN)
            }
            NoteTagShape::YellowRightArrowCheckBox => {
                self.icon_checkbox_with_right_arrow(status, COLOR_YELLOW)
            }
            NoteTagShape::BlueRightArrowCheckBox => {
                self.icon_checkbox_with_right_arrow(status, COLOR_BLUE)
            }
            NoteTagShape::YellowStar => {
                let mut style = StyleSet::new();
                style.set(self.color_property(), COLOR_YELLOW.to_string());

                let icon = self.pick(SVG_STAR, EMOJI_YELLOW_STAR);
                (Cow::from(icon), IconSize::Normal, style).into()
            }

            NoteTagShape::QuestionMark => self.simple_icon(SVG_QUESTION_MARK, EMOJI_QUESTION_MARK),

            NoteTagShape::HighPriority => self.simple_icon(SVG_ERROR, EMOJI_ERROR),
            NoteTagShape::ContactInformation => self.simple_icon(SVG_PHONE, EMOJI_PHONE),

            NoteTagShape::LightBulb => self.simple_icon(SVG_LIGHT_BULB, EMOJI_LIGHT_BULB),

            NoteTagShape::Home => self.simple_icon(SVG_HOME, EMOJI_HOME),
            NoteTagShape::CommentBubble => self.simple_icon(SVG_BUBBLE, EMOJI_BUBBLE),

            NoteTagShape::AwardRibbon => self.simple_icon(SVG_AWARD, EMOJI_AWARD),

            NoteTagShape::BlueCheckBox1 => self.icon_checkbox_with_1(status, COLOR_BLUE),

            NoteTagShape::BlueCheckBox2 => self.icon_checkbox_with_2(status, COLOR_BLUE),

            NoteTagShape::BlueCheckBox3 => self.icon_checkbox_with_3(status, COLOR_BLUE),

            NoteTagShape::BlueCheckMark => self.icon_checkmark(COLOR_BLUE),
            NoteTagShape::BlueCircle => self.icon_circle(COLOR_BLUE, EMOJI_CIRCLE_BLUE),

            NoteTagShape::GreenCheckBox1 => self.icon_checkbox_with_1(status, COLOR_GREEN),

            NoteTagShape::GreenCheckBox2 => self.icon_checkbox_with_2(status, COLOR_GREEN),

            NoteTagShape::GreenCheckBox3 => self.icon_checkbox_with_3(status, COLOR_GREEN),

            NoteTagShape::GreenCheckMark => self.icon_checkmark(COLOR_GREEN),
            NoteTagShape::GreenCircle => self.icon_circle(COLOR_GREEN, EMOJI_CIRCLE_GREEN),

            NoteTagShape::YellowCheckBox1 => self.icon_checkbox_with_1(status, COLOR_YELLOW),

            NoteTagShape::YellowCheckBox2 => self.icon_checkbox_with_2(status, COLOR_YELLOW),

            NoteTagShape::YellowCheckBox3 => self.icon_checkbox_with_3(status, COLOR_YELLOW),

            NoteTagShape::YellowCheckMark => self.icon_checkmark(COLOR_YELLOW),
            // OneNote renders this as a more orange circle; emoji palette matches.
            NoteTagShape::YellowCircle => self.icon_circle(COLOR_YELLOW, EMOJI_CIRCLE_ORANGE),

            NoteTagShape::BluePersonCheckBox => self.icon_checkbox_with_person(status, COLOR_BLUE),
            NoteTagShape::YellowPersonCheckBox => {
                self.icon_checkbox_with_person(status, COLOR_YELLOW)
            }
            NoteTagShape::GreenPersonCheckBox => {
                self.icon_checkbox_with_person(status, COLOR_GREEN)
            }
            NoteTagShape::BlueFlagCheckBox => self.icon_checkbox_with_flag(status, COLOR_BLUE),
            NoteTagShape::RedFlagCheckBox => self.icon_checkbox_with_flag(status, COLOR_RED),
            NoteTagShape::GreenFlagCheckBox => self.icon_checkbox_with_flag(status, COLOR_GREEN),
            NoteTagShape::RedSquare => self.icon_square(COLOR_RED, EMOJI_SQUARE_RED),
            NoteTagShape::YellowSquare => self.icon_square(COLOR_YELLOW, EMOJI_SQUARE_YELLOW),
            NoteTagShape::BlueSquare => self.icon_square(COLOR_BLUE, EMOJI_SQUARE_BLUE),
            NoteTagShape::GreenSquare => self.icon_square(COLOR_GREEN, EMOJI_SQUARE_GREEN),
            NoteTagShape::OrangeSquare => self.icon_square(COLOR_ORANGE, EMOJI_SQUARE_ORANGE),
            // OneNote labels this "pink" but renders purple.
            NoteTagShape::PinkSquare => self.icon_square(COLOR_PINK, EMOJI_SQUARE_PURPLE),
            NoteTagShape::EMailMessage => self.simple_icon(SVG_EMAIL, EMOJI_EMAIL),

            NoteTagShape::Contact => self.simple_icon(SVG_CONTACT, EMOJI_CONTACT),

            NoteTagShape::MusicalNote => self.simple_icon(SVG_MUSIC, EMOJI_MUSIC),
            NoteTagShape::MovieClip => self.simple_icon(SVG_FILM, EMOJI_FILM),

            NoteTagShape::HyperlinkGlobe => self.simple_icon(SVG_LINK, EMOJI_LINK),

            NoteTagShape::Padlock => self.simple_icon(SVG_LOCK, EMOJI_LOCK),
            NoteTagShape::OpenBook => self.simple_icon(SVG_BOOK, EMOJI_BOOK),

            NoteTagShape::BlankPaperWithLines => self.simple_icon(SVG_PAPER, EMOJI_PAPER),

            NoteTagShape::Pen => self.simple_icon(SVG_PEN, EMOJI_PEN),

            shape => self.icon_fallback(shape),
        }
    }

    fn simple_icon(&self, svg: &'static str, emoji: &'static str) -> NoteTagIcon {
        (Cow::from(self.pick(svg, emoji)), IconSize::Normal).into()
    }

    fn color_property(&self) -> &'static str {
        // Emoji are text glyphs, so they're styled via `color`; SVGs use `fill`.
        if self.use_emoji() { "color" } else { "fill" }
    }

    fn icon_fallback(&self, shape: NoteTagShape) -> NoteTagIcon {
        warn!("Unsupported icon type: {:?}", shape);

        self.simple_icon(SVG_QUESTION_MARK, EMOJI_QUESTION_MARK)
    }

    fn checkbox_glyph(&self, status: ActionItemStatus) -> &'static str {
        match (self.use_emoji(), status.completed()) {
            (false, true) => SVG_CHECKBOX_COMPLETE,
            (false, false) => SVG_CHECKBOX_EMPTY,
            (true, true) => EMOJI_CHECKBOX_COMPLETE,
            (true, false) => EMOJI_CHECKBOX_EMPTY,
        }
    }

    fn icon_checkbox(&self, status: ActionItemStatus, color: &'static str) -> NoteTagIcon {
        let mut styles = StyleSet::new();
        styles.set(self.color_property(), color.to_string());

        NoteTagIcon {
            html: Cow::from(self.checkbox_glyph(status)),
            size: IconSize::Large,
            styles,
            is_checkbox: true,
        }
    }

    fn icon_checkbox_with_person(
        &self,
        status: ActionItemStatus,
        color: &'static str,
    ) -> NoteTagIcon {
        self.icon_checkbox_with(status, color, self.pick(SVG_PERSON, EMOJI_PERSON))
    }

    fn icon_checkbox_with_right_arrow(
        &self,
        status: ActionItemStatus,
        color: &'static str,
    ) -> NoteTagIcon {
        self.icon_checkbox_with(status, color, self.pick(SVG_ARROW_RIGHT, EMOJI_ARROW_RIGHT))
    }

    fn icon_checkbox_with_star(
        &self,
        status: ActionItemStatus,
        color: &'static str,
    ) -> NoteTagIcon {
        self.icon_checkbox_with(status, color, self.pick(SVG_STAR, EMOJI_STAR))
    }

    fn icon_checkbox_with_flag(
        &self,
        status: ActionItemStatus,
        color: &'static str,
    ) -> NoteTagIcon {
        self.icon_checkbox_with(status, color, self.pick(SVG_FLAG, EMOJI_FLAG))
    }

    fn icon_checkbox_with_1(&self, status: ActionItemStatus, color: &'static str) -> NoteTagIcon {
        self.icon_checkbox_with(status, color, "1")
    }

    fn icon_checkbox_with_2(&self, status: ActionItemStatus, color: &'static str) -> NoteTagIcon {
        self.icon_checkbox_with(status, color, "2")
    }

    fn icon_checkbox_with_3(&self, status: ActionItemStatus, color: &'static str) -> NoteTagIcon {
        self.icon_checkbox_with(status, color, "3")
    }

    fn icon_checkbox_with_exclamation(
        &self,
        status: ActionItemStatus,
        color: &'static str,
    ) -> NoteTagIcon {
        self.icon_checkbox_with(status, color, "!")
    }

    fn icon_checkbox_with(
        &self,
        status: ActionItemStatus,
        color: &'static str,
        secondary_icon: &str,
    ) -> NoteTagIcon {
        let mut style = StyleSet::new();
        style.set(self.color_property(), color.to_string());

        // The secondary icon's CSS rules expect a `.content` element so the
        // glyph can be positioned over the underlying checkbox.
        let secondary_html = format!("<span class=\"content\">{secondary_icon}</span>");

        let mut content = String::new();
        content.push_str(self.checkbox_glyph(status));
        content.push_str(&format!(
            "<span class=\"icon-secondary\">{}</span>",
            secondary_html
        ));

        NoteTagIcon {
            html: Cow::from(content),
            size: IconSize::Large,
            styles: style,
            is_checkbox: true,
        }
    }

    fn icon_checkmark(&self, color: &'static str) -> NoteTagIcon {
        let mut style = StyleSet::new();
        style.set(self.color_property(), color.to_string());

        NoteTagIcon {
            is_checkbox: true,
            html: Cow::from(self.pick(SVG_CHECK_MARK, EMOJI_CHECK_MARK)),
            size: IconSize::Large,
            styles: style,
        }
    }

    fn icon_circle(&self, color: &'static str, emoji: &'static str) -> NoteTagIcon {
        if self.use_emoji() {
            // Emoji colored circles already carry their color glyph, so no style override.
            return (Cow::from(emoji), IconSize::Normal).into();
        }

        let mut style = StyleSet::new();
        style.set("fill", color.to_string());

        (Cow::from(SVG_CIRCLE), IconSize::Normal, style).into()
    }

    fn icon_square(&self, color: &'static str, emoji: &'static str) -> NoteTagIcon {
        if self.use_emoji() {
            return (Cow::from(emoji), IconSize::Normal).into();
        }

        let mut style = StyleSet::new();
        style.set("fill", color.to_string());

        (Cow::from(SVG_SQUARE), IconSize::Large, style).into()
    }
}
