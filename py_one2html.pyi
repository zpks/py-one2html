from os import PathLike
from typing import Literal, final

MathTarget = Literal["mathml", "latex"]
NoteTagIcons = Literal["svg", "emoji"]

@final
class PageHtml:
    """One rendered page with the metadata useful for page-aware chunking."""

    title: str | None
    level: int
    link_target_id: str
    author: str | None
    created: int
    """Unix timestamp."""
    updated: int
    """Unix timestamp."""
    html: str

@final
class SectionHtml:
    """A fully rendered section.

    Asset bytes stay on the Rust side; they are only copied into Python when
    requested via ``asset()``.
    """

    display_name: str
    group_path: list[str]
    """Display names of enclosing section groups, outermost first. Empty for
    top-level sections and for sections parsed via ``parse_section``."""
    pages: list[PageHtml]
    asset_names: list[str]
    """Filenames of the images/attachments referenced by the page HTML."""
    asset_sizes: dict[str, int]
    """Size in bytes per asset, without copying any asset data."""
    warnings: list[str]
    """Non-fatal parser warnings, prefixed with the page title when known."""

    def asset(self, name: str) -> bytes:
        """One asset's bytes, copied into Python on each call.

        Raises KeyError if ``name`` is not in ``asset_names``.
        """

@final
class NotebookHtml:
    """A fully rendered .onepkg notebook package."""

    display_name: str
    """Derived from the package file name; a package carries no notebook name."""
    sections: list[SectionHtml]
    """All sections, including those inside section groups, in TOC order."""
    warnings: list[str]
    """Notebook-level (table-of-contents) parser warnings."""

def convert(
    path: str | PathLike[str],
    output_dir: str | PathLike[str],
    *,
    warnings: bool = False,
    math_target: MathTarget = "mathml",
    note_tag_icons: NoteTagIcons = "svg",
) -> None:
    """Convert a .one, .onetoc2, or .onepkg file to HTML files in output_dir."""

def parse_section(
    data: bytes,
    *,
    file_name: str = "section.one",
    math_target: MathTarget = "latex",
    note_tag_icons: NoteTagIcons = "emoji",
) -> SectionHtml:
    """Parse a .one section's bytes into per-page HTML plus lazily-copied assets.

    Pass ``file_name`` (the original .one name) to get a meaningful
    ``display_name`` — the buffer itself does not carry one. Defaults suit an
    HTML→Markdown ingestion pass: LaTeX math and emoji note tags survive as
    text; MathML and inline SVGs do not.
    """

def parse_package(
    data: bytes,
    *,
    file_name: str = "notebook.onepkg",
    math_target: MathTarget = "latex",
    note_tag_icons: NoteTagIcons = "emoji",
) -> NotebookHtml:
    """Parse a .onepkg notebook package's bytes into rendered sections.

    The cabinet archive is decompressed entirely in memory.
    """
