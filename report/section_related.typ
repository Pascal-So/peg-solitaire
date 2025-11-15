= Related Work

We are not aware of any previous work that makes use of bloom filters to power a tree search. The idea however is not
that far-fetched, so there must be examples of this somewhere, even if maybe under different terminology.

On peg solitaire itself there exists a large body of theoretical work, which we want to highlight here.

- Solutions to peg solitaire using integer programming are described by Kiyomi et al. @goos_integer_2001 and Jefferson et
  al. @jefferson_modelling_2006.
- The "scout into desert" problem @berlekamp_winning_1982 @aigner_moving_1997 considers an infinite peg solitaire board
  with all pegs starting on one side of a line. The goal is to move a peg as far across the line as possible.
- Pagoda functions @berlekamp_winning_1982 @jefferson_modelling_2006 are a tool to prove that some positions cannot
  reach certain other positions; they map positions to numeric values such that any legal move can never decrease the
  value.
- Peg solitaire has been generalized to a game played on arbitrary graphs rather than just regular grids
  @engbers_reversible_2015.

Unrelated to peg solitaire, the "Checkers Is Solved" paper by Schaeffer et al. @schaeffer_checkers_2007 uses a beautiful
plot to visualize the state space, which has directly inspired the state space analysis in our work.
