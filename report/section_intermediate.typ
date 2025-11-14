#import "@preview/cetz:0.4.1"

#import "utils.typ": triple_two, triple_one

= Forcing Intermediate Positions

The usual goal of Peg Solitaire is to find a path from the start position to the end position. Players might also be
interested in whether some intermediate position can reach the end position, or if they're stuck and should start again.
Both of these questions can be answered by the method described so far.

Players might also wonder whether there exists a solution path that goes via a specific intermediate position $P$ chosen by the player. We'll refer to this as the intermediate-position-problem. If $P$ is known beforehand, then we could simply precompute another bloom filter of all positions that can reach $P$, as opposed to all positions that reach the end position as we have done so far. On the client we then separate the solver into two steps, first computing a path from the start to $P$, and then combining that with a path from $P$ to the end.

This method has two downsides however:

- We have to know $P$ at precompute time, which means that the player can't just choose any arbitrary intermediate position at runtime, unless we've precomputed the data for that position.
- Since the computation on the client device now requires access to two bloom filters, this effectively doubles the download size. Additionally, if the player wants to ask this question multiple times for different values of $P$, another bloom filter has to be downloaded every time.

To come up with a solution to both of these problems, we first have to exploit a few symmetries of the game.

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

By gratuitous abuse of terminology, we now apply this same concept to Peg Solitaire.

=== Parity Transformation
If we record a game and mirror one coordinate, the resulting recording still behaves exactly like a normal game of Peg
Solitaire. This is exactly the property that we used in the #link(<sec:mirroring>)[section on the mirroring optimization].
We therefore say that P symmetry holds in Peg Solitaire.

=== Time Reversal
To play a game of Peg Solitaire backwards in time is clearly not the same as playing it forwards, thus T symmetry does
not hold by itself.

=== Charge Conjugation
<sec:charge-conjugation>
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

    let arrow_r = text(weight: 100, 14pt, sym.arrow.r)
    let arrow_l = text(weight: 100, 14pt, sym.arrow.l)
    let arrow_offset = 0.08
    set-style(stroke: 0.7pt, mark: (end: "barbed"))

    let gap = 0.3
    let sx = 3.2
    let sy = 0
    let ex = 16.8
    let ey = -6

    let rh = 0.7
    let group_rect(x, y) = rect((x - gap - 3 - rh, y - rh), (x + gap + 3 + rh, y + rh), radius: 0.5, stroke: 0.5pt + gray)

    let text_padding = 0.3

    triple_two(sx - gap - 3, sy)
    content((sx, sy + arrow_offset), arrow_r)
    triple_one(sx + gap + 1, sy)
    group_rect(sx, sy)

    line((7.7, sy), (12.3, sy))
    content((10, sy + text_padding), anchor: "south", $C$)

    triple_one(ex - gap - 3, sy)
    content((ex, sy + arrow_offset), arrow_r)
    triple_two(ex + gap + 1, sy)
    group_rect(ex, sy)

    line((sx, -1), (sx, -5))
    content((sx - text_padding, -3), anchor: "east", $T$)
    line((ex, -1), (ex, -5))
    content((ex - text_padding, -3), anchor: "east", $T$)

    triple_two(sx - gap - 3, ey)
    content((sx, ey + arrow_offset), arrow_l)
    triple_one(sx + gap + 1, ey)
    group_rect(sx, ey)

    line((7.7, ey), (12.3, ey))
    content((10, ey + text_padding), anchor: "south", $C$)

    triple_one(ex - gap - 3, ey)
    content((ex, ey + arrow_offset), arrow_l)
    triple_two(ex + gap + 1, ey)
    group_rect(ex, ey)
  })
) <fig:commutative>


#let inv(p) = {
  math.overline(p)
}

@fig:commutative[Figure] shows the effect of applying both transformations, namely that
we end up with the original move, as if we had not applied any transformation at all.

Given positions $P$ and $Q$ that are one single move apart, this means that the inverse position of $Q$
can reach the inverse position of $P$ with that same move. We say that two moves are the same if
the coordinates of the moved peg, of the jumped peg, and of the target hole are matching.

$
P arrow Q arrow.r.double inv(Q) arrow inv(P),
$

where $inv(P)$ denotes the inverse of position $P$ as defined in the #link(<sec:charge-conjugation>)["Charge Conjugation" section].

We thus say that CT symmetry holds in Peg Solitaire.

== Intermediate Position Solver

Using this fact, we can now come up with a method to solve the intermediate-position-problem. Finding a path that goes via position $P$ can be broken down into the subtasks of finding a path from the start $S$ to $P$ and from $P$ to the end $E$. If there exists a path for both segments, then we can combine them to form the overall path.

$
S arrow dots.h.c arrow P arrow dots.h.c arrow E
$

Our method is based on the fact that $S$ and $E$ are inverses of each other:

$
S = inv(E)
$

Given that we have a function to find a sequence of moves $bold(m)_i, i in [1,k]$ to the end position

$
"solve"(P, E) = bold(m) arrow.l.r.double.long P arrow.r.long^(bold(m)_1) dots.h.c arrow.r.long^(bold(m)_k) E
$

we can use that same function together with CT symmetry to find a sequence of moves from the start to any position $P$:

$
& S arrow.r.long^(bold(m)_k) dots.h.c arrow.r.long^(bold(m)_1) P \
arrow.l.r.double.long & inv(P) arrow.r.long^(bold(m)_1) dots.h.c arrow.r.long^(bold(m)_k) inv(S) \
arrow.l.r.double.long & inv(P) arrow.r.long^(bold(m)_1) dots.h.c arrow.r.long^(bold(m)_k) E \
arrow.l.r.double.long & bold(m) = "solve"(inv(P), E)
$

If $"solve"(inv(P), E)$ does not find a sequence, then we know that $P$ is not reachable from the start. If it does find
a sequence, then we only have to reverse that sequence to get the path from the start to $P$. For our implementation
this means that we can reuse the same bloom filter for both segments, thus achieving our goal of keeping the download
size low and allowing $P$ to be chosen at runtime, after all precomputations have finished.

