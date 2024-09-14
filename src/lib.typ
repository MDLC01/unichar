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
  let (block-name, block-start, data, aliases) = index.get-data(code)

  let it = (
    code: code,
    id: {
      let id = upper(str(code, base: 16))
      while id.len() < 4 {
        id = "0" + id
      }
      "U+" + id
    },
    name: data.at(0, default: none),
    general-category: data.at(1, default: none),
    canonical-combining-class: data.at(2, default: none),
    block: (
      name: block-name,
      start: block-start,
    ),
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
      raw(it.id)
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
