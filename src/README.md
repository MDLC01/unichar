> [!NOTE]
> This file is used to generate [the Typst Universe page](https://typst.app/universe/package/unichar). It is processed by [`/build.py`](/build.py).


# Unichar

This package ports part of the [Unicode Character Database](https://www.unicode.org/reports/tr44/) to Typst. Notably, it includes information from [UnicodeData.txt](https://unicode.org/reports/tr44/#UnicodeData.txt) and [Blocks.txt](https://unicode.org/reports/tr44/#Blocks.txt).


## Usage

This package defines a single function: `codepoint`. It lets you get the information related to a specific codepoint. The codepoint can be specified as a string containing a single character, or with its value.

```example: SQUARE ROOT // Latin-1 Supplement // Lu // relation // ("vertical bowtie", "white framus")
#codepoint("√").name \
#codepoint(sym.times).block.name \
#codepoint(0x00C9).general-category \
#codepoint(sym.eq).math-class \
#codepoint("⧖").info.aliases
```

You can display a codepoint in the style of [Template:Unichar](https://en.wikipedia.org/wiki/Template:Unichar) using the `show` entry:

```example: Each codepoint is displayed using the "U+" syntax followed by a representative glyph and the symbol name in small capitals, or relevant information inside chevrons if the codepoint does not have a name.
#codepoint(sym.aleph).show \
#codepoint(sym.angzarr).show \
#codepoint(0x1249).show \
#codepoint(0x100000).show
```
