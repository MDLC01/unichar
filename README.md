# Unichar

This is a [Typst](https://github.com/typst/typst) package that ports part of the [Unicode Character Database](https://www.unicode.org/reports/tr44/) to Typst. Notably, it includes information from [UnicodeData.txt](https://unicode.org/reports/tr44/#UnicodeData.txt) and [Blocks.txt](https://unicode.org/reports/tr44/#Blocks.txt). It is available on [Typst Universe](https://typst.app/universe/package/unichar).


## Project structure

This project mainly consists of a Python script that downloads the necessary files from the Unicode Character Database and generates corresponding Typst files.

A small interface written in Typst lets the user access the data conveniently. It is available in [`src/`](src/).

This project can be built into a proper Typst package by [`build.py`](build.py). This build script was made for Python 3.12, and does not require anything outside of the standard library.


## Usage

For more information on how to use this package, take a look at the rendered README on [Typst Universe](https://typst.app/universe/package/unichar).


## License

The contents of this repository are licensed under the [MIT License](LICENSE). As per the [Unicode Terms of Use](https://www.unicode.org/copyright.html), the [Unicode License v3](https://www.unicode.org/license.txt) applies to the built Typst package.
