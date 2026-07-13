# typst4janim

## Introduction

This package compiles a Typst document and exports the elements of it directly to [JAnim](https://github.com/jkjkil4/JAnim), instead of exporting to SVG and then reading that back into JAnim.

## Structure

A call to `compile` flows through two stages:

1. **Compile** (`src/typst/`) — a Typst `World` is assembled, and the document is compiled into a `PagedDocument`.
2. **Collect** (`src/collect/`) — the (single) page's frame tree is walked, and every supported item is converted into an `Element`, which is returned to Python inside a `Collected` object.

The implementation of `src/typst/` closely follows `SystemWorld` from `typst-cli`, while `src/collect/` follows the approach of `SVGRenderer` from `typst-svg`.

A few conventions run across the collect stage:

- Curves are emitted as **quadratic** Bézier points to match JAnim's item model; cubic segments from font outlines and Typst geometries are approximated on the fly.
- Repeated glyph outlines are de-duplicated into a `shared` table and referenced by id.
- Elements carrying a Typst label are indexed into named `groups`, so JAnim can address subsets of the output.
- Anything not yet representable (images, gradients, tilings, clip paths, bitmap/color glyphs) is reported through non-fatal `warnings`.
