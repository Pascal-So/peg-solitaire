#import "@preview/cetz:0.4.1"

#import "utils.typ": board, theorem, proof

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
store them all on the client. To achieve a speed-up by not re-visiting lots of unsolvable nodes, it is sufficient if
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

