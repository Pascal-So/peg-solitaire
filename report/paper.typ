/*
 * Paper template for JACoW conference proceedings
 *
 * Based on the JACoW guide for preparation of papers.
 * See https://jacow.org/ for more information.
 *
 * This file is part of the accelerated-jacow template.
 * Typst universe: https://typst.app/universe/package/accelerated-jacow
 * GitHub repository: https://github.com/eltos/accelerated-jacow
 */

#import "template/jacow.typ": jacow, jacow-table

#show: jacow.with(
  // Paper title
  title: [Precomputing Peg Solitaire],
  // Author list
  authors: (
    (name: "Pascal Sommer"),
  ),
  // Funding note (optional, comment out if not applicable)
  // funding: "Work supported by ...",
  // Paper abstract
  abstract: [
    Finding a sequence of moves to solve a Peg Solitaire game is trivial if you have 1GB of RAM to spare. However,
    this might not be the case on a mobile web browser. Therefore, we optimize a WASM-compatible solver not just
    for runtime, but also the dimensions of RAM usage and total download size.
  ],
  date: [#datetime.today().display("[month repr:long] [year]")]
  // Writing utilities
  //draft-note: [*Draft 1* \u{2503} #datetime.today().display()],
  //page-limit: 3,
  //show-line-numbers: true,
  //show-grid: true,
)


// Other useful packages, see below for usage examples

#import "@preview/cetz:0.4.1"


= Introduction

Peg Solitaire is a single-player board game, where the goal is to eliminate pegs through moves such that only a single
peg remains in the centre @jefferson_modelling_2006.

#let board() = {
    import cetz.draw: *

    group({
      set-style(stroke: oklab(30%, 0, 0) + 1.0pt)
      // circle((0, 0), radius: 4.2)

      let cross_padding = 0.7
      let outer_radius = cross_padding
      let inner_radius = 1 - cross_padding

      let far = 3 + cross_padding
      let near = 1 + cross_padding

      merge-path(fill: white,  {
        for angle in (0, 1, 2, 3) {
          rotate(z: 90deg)
          line((far, -near + outer_radius), (far, near - outer_radius))
          arc((far - 0 * outer_radius, near - outer_radius), start: 0deg, stop: 90deg, radius: outer_radius)
          line((far - outer_radius, near), (near + inner_radius, near))
          arc((near + inner_radius, near), start: 270deg, stop: 180deg, radius: inner_radius) 
          line((near, near + inner_radius), (near, far - outer_radius))
          arc((near, far - outer_radius), start: 0deg, stop: 90deg, radius: outer_radius) 
        }
      })
    })

    for x in (-3, -2, -1, 0, 1, 2, 3) {
      for y in (-3, -2, -1, 0, 1, 2, 3) {
        if calc.abs(x) <= 1 or calc.abs(y) <= 1 {
          circle((x,y), radius: 0.35, stroke: 0.5pt)

          if x != 0 or y != 0 {
            circle((x, y), radius: 0.28, fill: oklab(20%, 0, 0), stroke: none)
          }
        }

      }

    }
}

#figure(
  placement: top,
  caption: "asdf",
  cetz.canvas(length: 0.4cm, {
    import cetz.draw: *

    group({
      rotate(4deg)
      board()
    })
    translate(x: 5)
    group({
      rotate(-4deg)
      board()
    })
  })
)

By adding a label

$
  e^("i" pi) + 1 = 0 
$ <eq:mycustomlabel>

they can be referenced as in @eq:mycustomlabel. // a reference to a labelled equation
The same works for @fig:writer too.
Remember to use the long form at the beginning of a sentence:
@fig:writer[Figure].
@eq:mycustomlabel[Equation].
Done.




== Figures and tables

Floating figures can be added and their placement can be controlled easily like shown here with @fig:writer.

#figure(
  image("plot.png"),
  scope: "parent",
  placement: bottom, // `top`, `bottom` or `auto` for floating placement or `none` for inline placement
  caption: [a plot],
) <fig:writer>

For JACoW style tables, the `jacow-table` function is provided.
It takes the column alignment as first argument (here `lcrl` means left, center, right, left), followed by the table contents.
The optional `header` argument allows to adjust the appearance of the JACoW table style as shown in @table:specs.


#figure(
  jacow-table("lccc", header: top+left, // top, left or none
    [], [Gen A], [Gen B], [Gen C],
    [Cells], [3], [5], [9],
  ),
  //placement: none, // `top`, `bottom` or `auto` for floating placement or `none` for inline placement
  caption: [
    Imaginary specifications of a device for the three generations A, B and C
  ]
) <table:specs>




= Packages

The Typst ecosystem features a broad range of community driven packages to make writing papers with Typst even more convenient.
These can be found by exploring the Typst Universe at https://typst.app/universe.

// See the import section near the top of this document





= Citations
Reference formatting uses standard bib files.
The bib snippets can conveniently be copied by selecting the format type "BibTex" when using the JACoW reference search tool at https://refs.jacow.org/.
Examples are given below @typst @jacowguide @jacow.org @example-journal-article @example-report @example-book @example-book-chapter @example-thesis @example-jacow-unpublished.




#bibliography("references.bib")



// Workaround until balanced columns are available
// See https://github.com/typst/typst/issues/466
#place(
  bottom,
  scope: "parent",
  float: true,
  clearance: 70pt, // TODO: increase clearance for manual column balancing
  []
)

