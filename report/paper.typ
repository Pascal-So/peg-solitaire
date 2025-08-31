#import "template/jacow.typ": jacow, jacow-table
#import "@preview/cetz:0.4.1"
#import "@preview/fletcher:0.5.8"
#import "board_illustration.typ": board, hole_outline, peg

#show: jacow.with(
  title: [Precomputing Peg Solitaire],
  authors: (
    (name: "Pascal Sommer"),
  ),
  paper-size: "a4",
  // funding: "Work supported by ...",
  abstract: [
    Finding a sequence of moves to solve a Peg Solitaire game is trivial if you have 1GB of RAM to spare. This might however
    not be the case on a mobile web browser. Therefore, we optimize a WASM-compatible solver not just
    for runtime, but also RAM usage and total download size.

    Furthermore, we show how a method to compute a sequence of moves from a given position to the normal end position
    can be trivially extended to also compute a sequence of moves from the normal start position to the end position
    while forcing the sequence to pass through a desired intermediate position.
  ],
  date: [#datetime.today().display("[month repr:long] [year]")],
  // show-grid: true,
)


= Introduction

Peg Solitaire is a single-player board game, where the goal is to eliminate pegs through moves such that only a single
peg remains in the centre @jefferson_modelling_2006. See @fig:progression for an illustration of the start and end positions.

In every move, the player can remove a peg from its hole, jump over an adjacent occupied hole, and place it the hole
after the jumped peg.

#figure(
  cetz.canvas(length: 0.6cm, {
    import cetz.draw: *
    import fletcher: edge

    hole_outline(0, 0)
    peg(0, 0)
    hole_outline(1, 0)
    peg(1, 0)
    hole_outline(2, 0)

    set-style(stroke: (thickness: 0.5pt, cap: "round", join: "round"), mark: (end: "straight", length: 0.22))
    arc((0.25, 0.45), start: 130deg, stop: 50deg, radius: 1.15)

    content((4, 0), text(16pt, sym.arrow.r.double))

    hole_outline(6, 0)
    hole_outline(7, 0)
    hole_outline(8, 0)
    peg(8, 0)
  })
)

The board starts out with 32 of its 33 holes occupied. Since every move removes exactly one peg, a complete game from the
start position to the end position will always consist of exactly 31 moves. Note that there are many possible solution
sequences.

When playing without any clear strategy, the player will often get stuck in a situation where no more moves are available,
for example when the remaining pegs are spread out across the board such that no two of them are directly adjacent anymore.



// todo: figure showing the size of the state space growing/shrinking, including solvable subset.


.. 50% of positions after 9 moves (23 pegs remaining) are unsolvable


#figure(
  placement: top,
  caption: "A typical progression in the game. The start position has every hole except for the central hole occupied. Every move removes one single peg, until we're left with only the central hole occupied in the end position.",
  cetz.canvas(length: 0.4cm, {
    import cetz.draw: *

    group({
      rotate(5deg)
      board("start")
      content(
        (2.4, 2.7),
        anchor: "mid-west",
        [Start Position],
      )
    })
    translate(x: 2.2, y: -5.7)
    group({
      rotate(1.5deg)
      board("mid")
      content(
        (2.4, 2.7),
        anchor: "mid-west",
        [Intermediate Position],
      )
    })
    translate(x: 1.8, y: -5.8)
    group({
      rotate(-2deg)
      board("end")
      content(
        (2.4, 2.7),
        anchor: "mid-west",
        [End Position],
      )
    })
  })
) <fig:progression>


= Solver

Simple visit map, 2^33 bits = 2^30 bytes, 1GB.

== Bloom Filters

#figure(
  image("img/k-hash-methods.png"),
  scope: "parent",
  placement: bottom,
  caption: [a plot],
) <fig:plot>

== de Bruijn, GF4

A significant optimisation is possible by making use of the game position equivalence classes defined by de Bruijn in 1972 @nicolaas_govert_de_bruijn_solitaire_1972.

= Forcing Intermediate States

#let quote(text, author) = {
  pad(left: 20%, right: 20%, top: 4mm, bottom: 4mm)[
    #emph(text)

    #box(width: 1fr, height: 5pt, stroke: (top: 0.3pt)) #h(4pt) #author
  ]
}

#quote("Then you'll see, that it is not the spoon that bends, it is only yourself.", "Spoon Boy")

#quote("Peg Solitaire is CT-symmetric", "Pascal Sommer")


#figure(
  jacow-table("lccc", header: top+left, // top, left or none
    [], [Gen A], [Gen B], [Gen C],
    [Cells], [3], [5], [9],
  ),
  caption: [
    Imaginary specifications of a device for the three generations A, B and C
  ]
) <table:specs>



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

