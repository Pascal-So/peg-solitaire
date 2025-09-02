#import "template/jacow.typ": jacow, jacow-table
#import "@preview/cetz:0.4.1"
#import "@preview/fletcher:0.5.8"
#import "board_illustration.typ": board, hole_outline, peg, triple_two, triple_one

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
    while forcing the sequence to pass through a desired intermediate position. This is achieved by exploiting a combination
    of symmetries in the game.
  ],
  date: [#datetime.today().display("[month repr:long] [year]")],
  // show-grid: true,
)


= Introduction

Peg Solitaire is a single-player board game, where the goal is to eliminate pegs through moves such that only a single
peg remains in the centre @jefferson_modelling_2006. See @fig:progression for an illustration of the start and end positions.

In every move, the player can remove a peg from its hole, jump over an adjacent occupied hole, and place it the empty hole
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

    content((3.5, 0.08), text(weight: 100, 18pt, sym.arrow.r.double))

    hole_outline(5, 0)
    hole_outline(6, 0)
    hole_outline(7, 0)
    peg(7, 0)
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
      rotate(4.8deg)
      board("start")
      content(
        (2.4, 2.7),
        anchor: "mid-west",
        [Start Position],
      )
    })
    translate(x: 2.2, y: -5.7)
    group({
      rotate(1.1deg)
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

== Rotations and Mirroring
<sec:mirroring>

== de Bruijn, GF4

A significant optimisation is possible by making use of the game position equivalence classes defined by de Bruijn in 1972 @nicolaas_govert_de_bruijn_solitaire_1972.

= Forcing Intermediate States

== Symmetries

#let quote(text, author) = {
  pad(left: 20%, right: 20%, top: 4mm, bottom: 1mm)[
    #emph(text)

    #box(width: 1fr, height: 5pt, stroke: (top: 0.3pt)) #h(4pt) #author
  ]
}

#block(breakable: false)[
  #quote(
    [Then you'll see, that it is not the spoon that bends, it is only yourself.],
    [Spoon Boy @wachowski_matrix_1999]
  )

  #quote(
    [Peg Solitaire is CT-symmetric.],
    [Pascal Sommer]
  )

  #v(5mm)
]


The concept of CPT symmetry in physics refers to the observation that the behaviour of the universe would remain
unchanged if all particles in the universe were to be replaced with their antiparticles (charge conjugation), the
universe was mirrored (parity transformation) and time was reversed @sozzi_parity_2007. Note that all three
transformations must be applied together, the symmetry does not hold for a subset of them.

By abuse of terminology, we now apply this same concept to Peg Solitaire.

=== Parity Transformation
If we record a game and mirror one coordinate, the resulting recording still behaves exactly like a normal game of Peg
Solitaire. This is exactly the property that we used in the #link(<sec:mirroring>)[section on the mirroring optimization].
We therefore say that P symmetry holds in Peg Solitaire.

=== Time Reversal
To play a game of Peg Solitaire backwards in time is clearly not the same as playing it forwards, thus T symmetry does
not hold by itself.

=== Charge Conjugation
As long as we don't look to closely at the actual materials making up our board and the pegs, the game doesn't really
have particles with charges that could be inverted. A natural equivalent, however, is to replace all empty holes with
occupied holes and vice versa. We also apply this transformation to the moves: instead of moving a peg over another one
into an empty spot, we can think of this inverted move as moving a hole over another hole onto a peg, destroying the
middle hole in the process.

It turns out that the time reversal and charge conjugation transformations are remarkably similar. While a normal move
changes a triplet with two pegs and one empty hole into a triplet with two empty holes and one peg, we now observe that
both transformations turn this into a change from one to two pegs.

#figure(
  placement: auto,
  caption: [
    Commutative diagram demonstrating CT symmetry for one single Peg Solitaire move. Note that the top left and bottom
    right moves are equivalent. $C$ denotes a charge conjugation, flipping all hole states. $T$ denotes a time reversal.
  ],
  cetz.canvas(length: 0.35cm, {
    import cetz.draw: *

    let arrow_r = text(weight: 100, 14pt, sym.arrow.r.double)
    let arrow_l = text(weight: 100, 14pt, sym.arrow.l.double)
    let arrow_offset = 0.08
    set-style(stroke: 0.7pt, mark: (end: "barbed"))

    let gap = 0.3
    let sx = 3.2
    let sy = 0
    let ex = 16.8
    let ey = -6

    let text_padding = 0.3
    
    triple_two(sx - gap - 3, sy)
    content((sx, sy + arrow_offset), arrow_r)
    triple_one(sx + gap + 1, sy)
    
    line((7.7, sy), (12.3, sy))
    content((10, sy + text_padding), anchor: "south", $C$)

    triple_one(ex - gap - 3, sy)
    content((ex, sy + arrow_offset), arrow_r)
    triple_two(ex + gap + 1, sy)

    line((sx, -1), (sx, -5))
    content((sx - text_padding, -3), anchor: "east", $T$)
    line((ex, -1), (ex, -5))
    content((ex - text_padding, -3), anchor: "east", $T$)
    
    triple_two(sx - gap - 3, ey)
    content((sx, ey + arrow_offset), arrow_l)
    triple_one(sx + gap + 1, ey)

    line((7.7, ey), (12.3, ey))
    content((10, ey + text_padding), anchor: "south", $C$)

    triple_one(ex - gap - 3, ey)
    content((ex, ey + arrow_offset), arrow_l)
    triple_two(ex + gap + 1, ey)
  })
) <fig:commutative>


@fig:commutative[Figure] shows the effect of applying both transformations, namely that
we end up with a normal move, as if we had not applyied no transformation at all.
We thus say that CT symmetry holds in Peg Solitaire.

== Algorithm

// notation for function to compute path from A to B. Notation for inverse (bar)


#figure(
  jacow-table("lccc", header: top+left, // top, left or none
    [], [Gen A], [Gen B], [Gen C],
    [Cells], [3], [5], [9],
  ),
  caption: [
    Imaginary specifications of a device for the three generations A, B and C
  ]
) <table:specs>


= Related Work

integer programming @jefferson_modelling_2006 @goos_integer_2001.

time-reversible game @engbers_reversible_2015.


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
