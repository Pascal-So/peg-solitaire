#import "template/jacow.typ": jacow, jacow-table
#import "@preview/cetz:0.4.1"
#import "@preview/fletcher:0.5.8"
#import "board_illustration.typ": board, hole_outline, peg, triple_two, triple_one

#import "@preview/ctheorems:1.1.3": *
#show: thmrules.with(qed-symbol: $square$)

#let theorem = thmplain(
  "theorem",
  "Theorem",
  base: none,
  titlefmt: strong
)
#let proof = thmproof("proof", "Proof")

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
  caption: "Three adjacent holes, before and after a single move was performed.",
  cetz.canvas(length: 0.6cm, {
    import cetz.draw: *
    import fletcher: edge

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

#let GF4 = $"GF"(4)$

== de Bruijn\'s $GF4$ Trick

The algorithm described so far will, assuming that the bloom filter is not too small, terminate rather quickly in the average case. There are however starting locations where the search gets stuck in a cluster of adjacent false positives. In those cases, the algorithm has to explore all of them before being able to conclude that they are in fact false positives.

This is especially relevant if we want to provide a user interface where the player can interactively edit a starting location by adding and removing individual pegs, just like they might do on a real board. We want to show the user whether the current position is solvable, and update this information in real time.

A significant optimisation would be possible by partitioning game positions into equivalence classes that are closed under moves. If we can then figure out that a given position is not in the same class as the end position, then we immediately know that the given position is unsolvable, since we can only ever reach positions that are in the same class as where we started.

// todo: link to some introduction to GF4?

One possible mapping from game positions to equivalence classes was published by de Bruijn by making use of Galois fields @nicolaas_govert_de_bruijn_solitaire_1972, specifically $GF4$ (also denoted $bb(F)_4$). He defines two functions $A$ and $B$ that both map a position $P$ to $GF4$. Since $GF4$ has four elements, this means that the pair $(A(P), B(P))$ can take on $4 times 4 = 16$ values.

$
A(P) = sum_(p in P) "TODO"
$

Note that we follow de Bruijn's notation for the elements of $GF4$: $0, 1, p, q$, i.e., we use $p$ to denote one of the elements that are neither the additive nor multiplicative identity.

We summarize the essence of de Bruijn's paper here for the reader's convenience.

#theorem([$A$ is preserved under moves])[\
  $P arrow.r Q arrow.r.double A(P) = A(Q)$
]<thm>

#proof[\
  Assume that the move happens in a direction where coordinates increase, such that the locations where pegs are removed contribute values $p^(i)$ and $p^(i+1)$, and the location where a peg is added contributes $p^(i+2)$.

  TODO do the proof

  // TODO state this as conclusion the other way around: $P$ and $Q$ differ by only this one move. To show that $A(P) = A(Q)$, we thus require that $p^i + p^(i+1) = p^(i+2)$
]<proof>

The proof for $B$ 

Note that this trick should only be applied once per search, namely at the starting position of the search. If we find that the given starting position is in the same equivalence class as the end position, then we perform the normal search algorithm as described previously. It does not make sense to re-apply this check at every visited position in the search, since all the visited positions are reachable from the given starting position, and therefore fall into the same equivalence class.

= Forcing Intermediate Positions

The usual goal of Peg Solitaire is to find a path from the start position to the end position. Players might also be interested in the question whether some intermediate position can reach the end position, or if they're stuck and should start again. Both of these questions can be answered by the method described so far.

Players might also wonder whether there exists a soluion path that goes via a specific intermediate position $P$ chosen by the player. We'll refer to this as the intermediate-position-problem. If $P$ is known beforehand, then we could simply precompute another bloom filter of all positions that can reach $P$, as opposed to all positions that reach the end position as we have done so far. On the client we then separate the solver into two steps, first computing a path from the start to $P$, and then combining that with a path from $P$ to the end.

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
we end up with the original move, as if we had not applyied no transformation at all.

Given positions $P$ and $Q$ that are one single move apart, this means that the inverse position of $Q$
can reach the inverse position of $P$ with that same move. We say that two moves are the same if
the coordinates of the moved peg, of the jumped peg, and of the target hole are matching.

$
P arrow Q arrow.r.double inv(Q) arrow inv(P),
$

where $inv(P)$ denotes the inverse of position $P$ as defined in the #link(<sec:charge-conjugation>)["Charge Conjugation" section].

We thus say that CT symmetry holds in Peg Solitaire.

== Algorithm

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

If $"solve"(inv(P), E)$ does not find a sequence, then we know that $P$ is not reachable from the start. If it does find a sequence, then we only have to reverse that sequence to get the path from the start to $P$. For our implementation this means that we can reuse the same bloom filter for both segments, thus achieving our goal of keeping the download size low and allowing $P$ to be chosen at runtime, after all precomputations have finished.

// TODO: introduce notation for function to compute path from A to B. Notation for inverse (bar)

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
