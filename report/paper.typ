#import "template/jacow.typ": jacow

#set page(numbering: "1")

#show: jacow.with(
  title: [Precomputing Peg Solitaire],
  authors: (
    (name: "Pascal Sommer"),
  ),
  paper-size: "a4",
  abstract: [
    Finding a sequence of moves to solve a Peg Solitaire game is trivial if you
    have 1GiB of RAM to spare. This might however not be the case on a mobile
    web browser.
    In this work, we thus present a solver based on bloom filters which is optimized
    for low RAM usage and download size.

    Furthermore, we show how any given method to compute a sequence of moves
    from a given position to the normal end position can easily be extended to
    also solve the "via" problem: computing a sequence of moves from the start
    to the end position, while forcing the sequence to pass through a desired
    intermediate position. This is achieved by exploiting a combination of
    symmetries in the game.
  ],
  date: [December 2025],
)


#place(footnote(numbering: it => "", [
  Source code available at
  #underline([https://github.com/Pascal-So/peg-solitaire]).
]))
#counter(footnote).update(0)

#include "section_intro.typ"
#include "section_solver.typ"
#include "section_eval.typ"
#include "section_intermediate.typ"
#pagebreak()
#include "section_pseudocode.typ"
#include "section_related.typ"
#include "section_future.typ"

#bibliography("references.bib")

// Workaround until balanced columns are available
// See https://github.com/typst/typst/issues/466
#place(
  bottom,
  scope: "parent",
  float: true,
  clearance: 559pt, // TODO: increase clearance for manual column balancing
  []
)
