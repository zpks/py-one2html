# py-one2html

py-one2html is a python wrapper around one2html, which lets you convert OneNote® files (sections or whole notebooks)
into HTML. Extra added over base repo are buffer processing and returning a Python-readable object.

Needed this for work, may become something more later.

Current install: add to repo as submodule, run
```
uv run maturin develop
```
and add py-one2html to your pyproject.toml with a separate

```
[tool.uv.sources]
py-one2html = { path = "one2html"}
```
to add the source locally.

Unless you know what you are doing, I would refer all readers to https://github.com/msiemens/one2html

## Limitations

- Due to limitations of the [OneNote parser](https://github.com/msiemens/onenote.rs)
  only files downloaded from OneDrive are supported. This means you can't
  convert files created by the OneNote 2016 desktop application using
  this tool.

## Disclaimer

This project is neither related to nor endorsed by Microsoft in any way. The
author does not have any affiliation with Microsoft.
