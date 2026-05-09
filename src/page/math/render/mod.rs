use super::ast::Equation;
use crate::{MathTarget, Options};
use color_eyre::Result;
use finl_unicode::categories::CharacterCategories;

mod latex;
mod mathml;

pub(crate) fn render_equation(equation: Equation, options: Options) -> Result<String> {
    match options.math_target {
        MathTarget::MathML => mathml::render_equation(equation),
        MathTarget::LaTeX => latex::render_equation(equation),
    }
}

pub(super) fn is_skippable_format(c: char) -> bool {
    c.is_format() && c != '\u{2061}'
}
