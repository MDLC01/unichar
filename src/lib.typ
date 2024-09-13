/// Create a `codepoint` object.
///
/// You can convert a to content `codepoint` using its `show` field:
/// ```example
/// #codepoint("¤").show
/// ```
#let codepoint(code) = {
  if type(code) != int {
    code = str.to-unicode(code)
  }

  import "ucd/index.typ"
  let (block, data, aliases) = index.get-data(code)

  let it = (
    code: code,
    name: data.at(0, default: none),
    general-category: data.at(1, default: none),
    canonical-combining-class: data.at(2, default: none),
    block: block,
    aliases: (
      corrections: aliases.at(0),
      controls: aliases.at(1),
      alternates: aliases.at(2),
      figments: aliases.at(3),
      abbreviations: aliases.at(4),
    )
  )

  (
    ..it,
    "show": {
      let str-code = upper(str(it.code, base: 16))
      while str-code.len() < 4 {
        str-code = "0" + str-code
      }
      raw("U+" + str-code)
      sym.space.nobreak
      if it.name == none {
        "<unused>"
      } else if it.name.starts-with("<") {
        it.name
      } else {
        // The character appears bigger without increasing line height.
        text(size: 1.2em, top-edge: "x-height", str.from-unicode(it.code))
        sym.space
        smallcaps(lower(it.name))
      }
    },
  )
}
