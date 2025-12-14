#import "@preview/cetz:0.4.1"

#import "utils.typ": board, hole_outline, peg, shortline

= Introduction

Peg Solitaire is a single-player board game, where the goal is to eliminate pegs through moves such that only a single
peg remains in the centre @jefferson_modelling_2006. See @fig:progression for an illustration of the start and end positions.

In every move, the player can remove a peg from its hole, jump over an adjacent occupied hole, and place it the empty hole
after the jumped peg.

#figure(
  caption: "Three adjacent holes, before and after a single move was performed.",
  cetz.canvas(length: 0.6cm, {
    import cetz.draw: *

    hole_outline(0, 0)
    peg(0, 0)
    hole_outline(1, 0)
    peg(1, 0)
    hole_outline(2, 0)

    set-style(stroke: (thickness: 0.5pt, cap: "round", join: "round", dash: (1.93pt, 1.93pt)), mark: (end: "straight", length: 0.24))
    arc((0.25, 0.45), start: 130deg, stop: 50deg, radius: 1.15)

    content((3.5, 0.08), text(weight: 100, 18pt, sym.arrow.r))

    hole_outline(5, 0)
    hole_outline(6, 0)
    hole_outline(7, 0)
    peg(7, 0)
  })
)

The board starts out with 32 of its 33 holes occupied. Since every move removes exactly one peg, a complete game from the
start position to the end position will always consist of exactly 31 moves. Note that there are many possible solution
sequences.

When playing without any clear strategy, the player will often get stuck in a dead end where no more moves are available,
for example when the remaining pegs are spread out across the board such that no two of them are directly adjacent any
more.

#figure(
  placement: top,
  caption: "A typical progression in the game. The start position has every hole except for the central hole occupied. Every move removes one single peg, until we're left with only the central hole occupied in the end position.",
  cetz.canvas(length: 0.4cm, {
    import cetz.draw: *

    group({
      rotate(-2deg)
      board("start")
      content(
        (2.4, 2.7),
        anchor: "mid-west",
        [Start Position],
      )
    })
    translate(x: 2.2, y: -5.7)
    group({
      rotate(-2deg)
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

Our goal in this work is to develop a tool that players can use to automatically find a sequence of moves to reach the
end position if such a sequence exists, or to tell the player that they have entered an unsolvable position.

To make the peg solitaire solver tool accessible to a wide audience,
it makes sense to deploy it as a WebAssembly-based application so that users can access it from their desktop or mobile device
without having to install anything. This also allows us to deploy the tool without having to run any application-specific
software on the server, instead allowing us to rely just on a static file server.

== State Space

A board has 33 holes which all can either be occupied or empty, which means that there are $2^33$ different positions.
On modern hardware, it's easily feasible to simply enumerate the entire state space so that we can accumulate whatever
statistics we are interested in.

#figure(
  caption: [
    Overview of the state space. We compare the following sets:
    #shortline(rgb(247,113,137))Solvable, the set of positions that can reach the end,
    #shortline(rgb(80,177,49))Reachable, the positions that can be reached from the start,
    #shortline(rgb(54,173,164))the intersection of those two sets,
    #shortline(rgb("#666"))the set of all possible positions, regardless of whether they can actually be reached during normal play,
    #shortline(rgb("#bbb"))the set of positions that are solvable according to de Bruijn's formula (explained in the next section).
  ],
  placement: bottom,
  scope: "parent",
  image("img/state-space.pdf")
) <fig:statespace>

It turns out that only around 2% of all positions are solvable, i.e., have a path to the end position. However, the majority
of those unsolvable positions are not reachable from the start position. Similarly, there are some positions that cannot be
reached from the start, but that could still be solved if we were to manually set up the board to one of these positions
before the game. @fig:statespace shows an overview of the relations between these sets.

One interesting observation is that during the beginning it's very hard to mess anything up, since almost all
the positions that can be reached within the first five moves are solvable. During the later stages however, the player
is almost guaranteed to fail if they just perform random moves.

