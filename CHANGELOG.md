# Changelog

## Version 0.2.0

- Codepoints now have an `id` attribute which is its corresponding "U+xxxx" string.

- The `block` attribute of a codepoint now contains a `name`, a `start`, and a `size`.

- Fix an issue that made some codepoints cause a panic.

- Include data from NameAlias.txt.

## Version 0.1.0

- Add the `codepoint` function.
