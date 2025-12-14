= Future Work

During the work on this report, several ideas for further exploration had to be
moved to the "later" pile in the interest of time.

One such idea would be an analysis of the unsolvable, yet de-Bruijn-solvable
part of the state space. Looking at @fig:statespace, it feels like the region
wedged between the "de Bruijn", the "Solvable" and the "Reachable" lines is
rather small (keep in mind, however, the logarithmic y-axis). This raises the
question whether all de-Bruijn-solvable positions are reachable in reversible
peg solitaire @engbers_reversible_2015, i.e. if we allow both forwards and
backwards moves.

As a further optimization to the current solver, potentially allowing us to
use an even smaller bloom filter, we could try to get a better grasp on clusters
of false positives. It might be the case that there is a small set of very large
such clusters. If it turns out that a large percentage of fruitless tree
searches currently go through a very small set of crucial nodes, then a potential
strategy to improve the solver would be to additionally store a list of these
worst offenders, separate from the bloom filter.
