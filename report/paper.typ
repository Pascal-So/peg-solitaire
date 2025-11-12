#import "template/jacow.typ": jacow, jacow-table
#import "@preview/cetz:0.4.1"
#import "@preview/fletcher:0.5.8"
#import "board_illustration.typ": board, hole_outline, peg, triple_two, triple_one

#import "@preview/ctheorems:1.1.3": *
#show: thmrules.with(qed-symbol: $square$)

#let thm-padding = 1.5em
#let theorem = thmplain(
  "theorem",
  "Theorem",
  base: none,
  titlefmt: strong,
  inset: (top: 0em, left: thm-padding, right: thm-padding),
)
#let proof = thmproof(
  "proof",
  "Proof",
  inset: (top: 0em, left: thm-padding, right: thm-padding),
)


#show: jacow.with(
  title: [Precomputing Peg Solitaire],
  authors: (
    (name: "Pascal Sommer"),
  ),
  paper-size: "a4",
  // funding: "Work supported by ...",
  abstract: [
    Finding a sequence of moves to solve a Peg Solitaire game is trivial if you have 1GiB of RAM to spare. This might however
    not be the case on a mobile web browser. Therefore, we optimize a WASM-compatible solver not just
    for runtime, but also RAM usage and total download size.

    Furthermore, we show how a method to compute a sequence of moves from a given position to the normal end position
    can be trivially extended to also compute a sequence of moves from the normal start position to the end position
    while forcing the sequence to pass through a desired intermediate position. This is achieved by exploiting a combination
    of symmetries in the game.
  ],
  date: [#datetime.today().display("[month repr:long] [year]")],
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

When playing without any clear strategy, the player will often get stuck in a dead end where no more moves are available,
for example when the remaining pegs are spread out across the board such that no two of them are directly adjacent anymore.

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

Our goal in this work is to develop a tool that players can use to automatically find a sequence of moves to reach the
end position if such a sequence exists, or to tell the player that they have entered an unsolvable position.

To make the Peg Solitaire solver tool accessible to a wide audience,
it makes sense to deploy it as a WebAssembly-based application so that users can access it from their desktop or mobile device
without having to install anything. This also allows us to deploy the tool without having to run any application-specific
software on the server, instead allowing us to rely just on a static file server.

== State Space

A board has 33 holes which all can either be occupied or empty, which means that there are $2^33$ different positions.
On modern hardware, it's easily feasible to simply enumerate the entire state space so that we can accumulate whatever
statistics we are interested in.

It turns out that only around 2% of all positions are solvable, i.e., have a path to the end position. However, the majority
of those unsolvable positions are not reachable from the start position. Similarly, there are some positions that cannot be
reached from the start, but that could still be solved if we were to manually set up the board to one of these positions
before the game. @fig:statespace shows an overview of the relations between these sets.

#figure(
  caption: [
    #let r(col) = [#box(rect(width: 8pt, height: 1.7pt, fill: col), baseline: -2pt)#sym.space.nobreak]
    Overview of the state space. We compare four different sets:
    #r(rgb(247,113,137))Solvable, the set of positions that can reach the end,
    #r(rgb(80,177,49))Reachable, the positions that can be reached from the start,
    #r(rgb(54,173,164))the intersection of those two sets,
    #r(rgb("#666"))the set of all possible positions, regardless of whether they can actually be reached during normal play.
  ],
  placement: bottom,
  scope: "parent",
  image("img/state-space.pdf")
) <fig:statespace>

One interesting observation is that during the beginning it's very hard to mess anything up, since almost all
the positions that can be reached within the first five moves are solvable. During the later stages however, the player
is almost guaranteed to fail if they just perform random moves.

= Solver

The standard method to automatically solve such a game would be some kind of tree search. At every position, we
enumerate all the valid moves that could be applied to the position, and then perform the same search method at the
descendant positions. By following this method, we will either eventually reach the end position, or run out of new
positions to visit.

To prevent unnecessary work, we should make sure that every position will only be visited at most once. For this
purpose, we can store one bit per position, remembering if that position has already been visited. With $2^33$ positions,
this sums up to exactly $2^30$ bytes, or 1 GiB.

While this is a perfectly reasonable amount of memory to allocate in desktop software nowadays, it is still a large amount
in the context of software running in the browser as WASM. We instead want to use a method that gets by with much less memory.

The essence of our strategy is to still perform a tree search, specifically depth-first-search, but avoid storing a map
of visited positions. Of course just running the DFS without a visit map leads to exponential runtime, since we're
re-visiting positions that have already been explored. Note however, that the re-visited positions must be positions that
are unsolvable anyway. If the position had been solvable, then we would have reached the end, and thus never re-visited
it.

Our main idea is therefore to simply prioritize a correct path during the DFS (duh). Recall that only around 2% of the
positions are solvable. If the algorithm has a fast method to look up whether the position that we're about to visit is
solvable, then we can avoid visiting unsolvable positions in the first place.

Of course 2% in this context is still a large number, in total there are 187,636,299 solvable positions, so we can't
store them all on the client. To achieve the speedup by not re-visiting lots of unsolvable nodes, it is sufficient if
we have an approximation of this set of solvable positions, rather than knowing it exactly.

== Bloom Filters

Bloom Filters are probabilistic data structures that provide two operations: we can add an element to a set, and we can
ask whether a given element is present in the set @bloom_spacetime_1970. A bloom filter requires much less storage than
simply storing all the elements, but the downside is that there is a small chance of getting an incorrect answer when
checking for presence.

If an element is in the set, then a bloom filter will always correctly respond "Yes" when asking for that element. But
if an element is not present, there is still a small chance for the filter to think it is there, i.e. there might be
false positives but never false negatives.

This is exactly what we can use for our tree search. We build a bloom filter of all solvable positions, and then during
the search we only visit positions where the presence check returns "Yes". We will therefore sometimes explore unnecessary
positions, but we will never abandon a solvable path.

Bloom filters provide us with a trade-off parameter that we can tune. If we make the bloom filter more accurate, we
waste less time during tree search, but this increases the storage size of the bloom filter, meaning that we have to
download and store more data to the user's device.

To construct a bloom filter, we run a tree search backwards from the end position, which lets us enumerate all solvable
positions. These are then added to the filter. This computation can be done once offline, so the runtime of this
preprocessing step is not that relevant.

== Rotations and Mirroring
<sec:mirroring>

Another way to improve the accuracy of a bloom filter, beyond increasing the size of the filter, is to store fewer
elements in the filter.

The peg solitaire is four-fold rotationally symmetric, as well as reflection symmetric. This also applies to the end
position. Therefore, if we know that a given position is solvable, then we also know that any rotation or mirroring of
that position is solvable.

This can be used to reduce the number of positions that we need to store in the bloom filter, from 187,636,299 down to
23,475,688. The reduction factor is slightly less than 8 because we're not eliminating positions that are themselves
already symmetric.

We design a normalization function to select one specific orientation of a given position that we then insert into
the bloom filter. The exact method by which that orientation is selected is not relevant, as long as our choice is
applied consistently. In our implementation we first list all eight candidates as 33-bit binary numbers where every
hole is converted to one bit, assembled row-wise. From this list we select the candidate with the lowest numeric value.

#figure(
  caption: [
    These two positions are equivalent if we ignore mirroring and rotation. If
    we know that the left one is solvable, then the right one must also be
    solvable.
  ],
  cetz.canvas(length: 0.28cm, {
    import cetz.draw: *

    board("mid")

    content((8.6, 0), text(1.5em)[#sym.arrow.l.r])
    content((6.4, 0), text(1.5em)[#sym.arrow.cw])

    translate(x: 15, y: 0)
    rotate(270deg)
    scale(y: -1)

    board("mid")
  })
) <fig:mirroring>


#let GF4 = $"GF"(4)$

== de Bruijn\'s $GF4$ Trick

The algorithm described so far will, assuming that the bloom filter is not too small, terminate rather quickly in the
average case. There are however starting positions where the search gets stuck in a cluster of adjacent false positives.
In those cases, the algorithm has to explore all of them before being able to conclude that they are in fact false positives.

// todo: add some reference to the experiments section here for clusters and for "quickly in the average case"

This is especially relevant if we want to provide a user interface where the player can interactively edit a starting
position by adding and removing individual pegs, just like they might do on a real board. We want to show the user
whether the current position is solvable, and update this information in real time.

A significant optimization would be possible by partitioning game positions into equivalence classes that are closed
under moves. If we can then figure out that a given position is not in the same class as the end position, then we
immediately know that the given position is unsolvable, since we can only ever reach positions that are in the same
class as where we started.

One possible mapping from game positions to equivalence classes was published by de Bruijn by making use of Galois
fields @nicolaas_govert_de_bruijn_solitaire_1972, specifically $GF4$ (also denoted $bb(F)_4$). He defines two functions
$A$ and $B$ that both map a position $P$ to $GF4$. Since $GF4$ has four elements, this means that the pair $(A(P), B(P))$
can take on $4 times 4 = 16$ values.

$
A(P) = sum_((k,l) in P) p^(k + l),
$

where we say that $(k,l) in P$ if $k$ and $l$ are natural numbers that represent the coordinates of a peg present in
position $P$ such that the centre hole has coordinates (0, 0).

Note that we follow de Bruijn's notation for the elements of $GF4$: $0, 1, p, q$, i.e., we use $p$ to denote one of the
elements that are neither the additive nor multiplicative identity.

The function $B$ takes almost the same form:

$
B(P) = sum_((k,l) in P) p^(k - l)
$

We summarize the proof from de Bruijn's paper here in a slightly compressed form for the reader's convenience.

#theorem([$A$ is preserved under moves])[\
  $P arrow.r Q arrow.r.double A(P) = A(Q)$
]<thm>

#proof[\
  Assume that the move happens in a direction where coordinates increase, such that the coordinates where pegs are removed
  contribute values $p^(i)$ and $p^(i+1)$, and the coordinate where a peg is added contributes $p^(i+2)$.

  From the definition of #GF4 we can verify that

  $
  1 + p = p^2.
  $

  Using this, we get

  $
  p^i + p^(i+1) &= p^i(1+p) \
  &= p^i p^2 \
  &= p^(i+2),
  $

  which is the term that we add at the same time as removing the other two terms. Therefore, the sum does not change after
  the move, and we get that $A(P) = A(Q)$.

  It remains to show that the same also holds when the move is taken in the direction of decreasing coordinates. Let the
  coordinates where pegs are removed still be $p^i$ and $p^(i+1)$, but now a peg is added at $p^(i-1)$ instead.

  Because the multiplicative group of #GF4 has three elements, we know that $p^3 = 1$ and therefore $p^(i-1) = p^(i+2)$.
]<proof>

The proof for $B$ follows the same procedure.

Note that this trick should only be applied once per search, namely at the starting position of the search. If we find
that the given starting position is in the same equivalence class as the end position, then we perform the normal search
algorithm as described previously. It does not make sense to re-apply this check at every visited position in the
search, since all the visited positions are reachable from the given starting position, and therefore fall into the same
equivalence class. In particular, this trick will never improve the performance of a search from the normal start
position.

== Compression

Our switch away from a normal tree search to the bloom filter assisted search was motivated by RAM usage. While the
bloom filter requires much less RAM, the downside now is that the filter actually has to be downloaded over the network
to the client. Since the filter sizes are in the megabytes (see #link(<sec:evaluation>)[evaluation section]), the
download size is now the bigger concern.

We can somewhat counteract the slow download speeds by making use of HTTP compression. This means that we can upload a
compressed version of the bloom filter to our static file server along with the original, so that if a client supports
the compressed format, it does not have to download the full file. Decompression is then done automatically by the
client browser, and our WASM code does not have to be aware of the compression.

= Parameter Selection and Evaluation
<sec:evaluation>

In this section, we perform some evaluations in order to tune the Bloom Filter parameters available to us and to see the
overall performance that we end up with.

We aim to optimize two statistics:
- The solver performance, i.e. the number of operations that have to be performed in order to find a sequence of moves
  to solve a given position, or to determine that the position is unsolvable.
- The download size of the compressed bloom filter that we have to transmit to the client. For these evaluations, we
  specifically select the Brotli compression format, since it is widely supported by browsers and achieves competitive
  compression ratios @alakuijala_brotli_2016.

== Parameter $k$

Bloom Filters compute $k$ different hashes of the given input @bloom_spacetime_1970. Assuming that the hashes are
uncorrelated, the optimal $k$ for the lowest false positive rate can be computed given the number of elements in the
filter and the total size of the input space @thomas_hurst_bloom_nodate.

#figure(
  image("img/k.pdf"),
  placement: bottom,
  caption: [
    Comparing false positive rates for different values of $k$. For smaller filters, $k = 1$ is optimal, but on larger
    filters, increasingly larger $k$ yield lower false positive rates.
  ],
) <fig:k>


$
k = "round"(m/n log(2))
$

Note however, that a larger $k$ increases the number of bits that are set to 1 in the bloom filter. This negatively
affects the compression ratios achieved by Brotli and other methods. It turns out that the better compression on $k = 1$
outperforms the better false positive rates on $k > 1$, as can be seen in @fig:k_compression.

#figure(
  image("img/k-vs-compression.pdf"),
  placement: top,
  caption: [
    Comparing false positive rates for compressed and uncompressed bloom filters with different values of $k$. While
    $k = 3$ is optimal among all uncompressed filters of size 12 MB, we see that $k = 1$ yields a lower false positive
    rate at 12 MB if we allow for Brotli compression.
  ],
) <fig:k_compression>

== Hashing Method & Filter Size

Sticking with $k = 1$ turns out make a much simpler hashing method possible. Assuming our bloom filter contains $m$ bits,
we need to design a hash function that takes a board position and maps it to a bit index in the range $[0, m)$. A simple
way to do this is to interpret the position as a 33-bit number by interpreting every hole as a 1 if its occupied
and 0 otherwise, and then assembling all the holes into a binary number. From this number we take the remainder after
division by $m$, which gives us a bit index in the desired range. And as long as $m$ is much smaller than $2^(33)$ we
end up with a distribution that is close enough to uniform for our purposes.

Now we are left with the task of selecting a value for $m$. We consider two candidate groups:

- Primes, which intuitively should minimize any hash collisions between similar positions.
- "Round numbers", numbers whose prime factorizations mostly or entirely consist of factors 2. We
  expect them to behave as the conceptual opposite of the prime numbers, this should thus show us if using primes even
  makes a difference.

#figure(
  image("img/round-vs-primes-fpr.pdf"),
  placement: bottom,
  caption: [
    Comparing the false positive rates between bloom filters that hash by taking the remainder after division by prime
    numbers vs. numbers that are divisible by large powers of two. The latter hash method yields bloom filters that do
    not follow the expected theoretical false positive rates for our dataset.
  ],
) <fig:round_vs_primes_fpr>

#figure(
  image("img/round-vs-primes-compression.pdf"),
  placement: bottom,
  caption: [
    Comparing compression efficiencies with Brotli between different bloom filters. The point where the primes curve
    reaches 1:1 is where 50% of the bits in the bloom filter are set.
  ],
) <fig:round_vs_primes_compression>


TODO

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
we end up with the original move, as if we had not applied any transformation at all.

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

= Overall Pseudocode



= Related Work

TODO

integer programming @jefferson_modelling_2006 @goos_integer_2001.

time-reversible game @engbers_reversible_2015.

= Future Work

TODO

Reachable state space in time-reversible game on normal board?


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
