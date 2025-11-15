#import "@preview/lovelace:0.3.0": *

#let code(title: none, body) = [
  #v(3mm)
  #pseudocode-list(
    hooks: .5em,
    booktabs: true,
    booktabs-stroke: stroke(1.1pt + rgb("333")),
    title: smallcaps(title),
    body
  )
  #v(3mm)
]

#let eq = box(inset: (x: 3pt), scale(x: 230%)[=])

#let solved = "SOLVED"
#let timeout = "TIMEOUT"
#let unsolvable = "UNSOLVABLE"

= Pseudocode
<sec:pseudocode>


The algorithm is mainly based on a depth-first-search, but instead of tracking which nodes have already been visited,
we just skip nodes that are not present in the bloom filter.

#code(title: [Depth-First-Search], [
  + *function* DFS(node, steplimit)
    + *if* node #eq end
      + *return* #solved
    + *if* steplimit #eq 0
      + *return* #timeout
    + *foreach* next *in* reachable nodes *do*
      + *if* normalized(next) $in$ bloomfilter
        + *match* DFS(next, steplimit - 1)
          + #solved $=>$ *return* #solved
          + #timeout $=>$ *return* #timeout
          + #unsolvable $=>$ *continue*
      + *else*
        + continue
    + *return* #unsolvable
])

This method can sometimes get stuck in clusters of false positives where the same nodes are visited repeatedly. To
counteract this, we cancel the attempt after a certain number of steps, shuffle the order in which descendant nodes
are visited, and try again. This retry method vastly improves the worst case number of steps.

#code(title: [Solve-With-Bloom-Filter], [
  + *function* solve(startnode)
    + steplimit $<-$ 50
    + *for* 100 attempts *do*
      + *if* last attempt
        + steplimit $<-$ 10,000
      + *match* DFS(startnode, steplimit)
        + #solved $=>$ *return* #solved
        + #timeout $=>$ *continue*
        + #unsolvable $=>$ *return* #unsolvable
    + *return* #timeout
])

The increased limit in the last attempt serves to deal with unsolvable start positions that are located in deep false
positive clusters.
