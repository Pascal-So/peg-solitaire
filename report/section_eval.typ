#import "utils.typ": shortline

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

#place(
  bottom,
  scope: "parent",
  float: true,
  grid(
    columns: 2,
    align: top,
    gutter: 5mm,
    [
      #figure(
        image("img/k.pdf"),
        caption: [
          Comparing false positive rates for different values of $k$. For smaller filters, $k = 1$ is optimal, but on larger
          filters, increasingly larger $k$ yield lower false positive rates. The measured results for our data align closely
          with the expected theoretical values.
        ],
      ) <fig:k>
    ],
    [
      #figure(
        image("img/k-vs-compression.pdf"),
        caption: [
          Comparing false positive rates for compressed and uncompressed bloom filters with different values of $k$. While
          $k = 3$ is optimal among all uncompressed filters of size 12 MB, we see that $k = 1$ yields a lower false positive
          rate at 12 MB if we allow for Brotli compression.
        ],
      ) <fig:k_compression>
    ]
  )
)


$
k = "round"(m/n log(2))
$

Note however, that a larger $k$ increases the number of bits that are set to 1 in the bloom filter. This negatively
affects the compression ratios achieved by Brotli and other methods. It turns out that across all size ranges, the better
compression on $k = 1$ outperforms the better false positive rates on $k > 1$, as can be seen in @fig:k_compression.

== Hashing Method & Filter Size

Sticking with $k = 1$ turns out make a much simpler hashing method possible. Assuming our bloom filter contains $m$ bits,
we need to design a hash function that takes a board position and maps it to a bit index in the range $[0, m)$. A simple
way to do this is to interpret the position as a 33-bit number by interpreting every hole as a 1 if its occupied
and 0 otherwise, and then assembling all the holes into a binary number. From this number we take the remainder after
division by $m$, which gives us a bit index in the desired range. And as long as $m$ is much smaller than $2^(33)$ we
end up with a distribution that is close enough to uniform for our purposes.

Now we are left with the task of selecting a value for $m$. We consider two candidate groups:

#let rn = ["Round Numbers"]

- Primes, which intuitively should minimize any hash collisions between similar positions.
- #rn, numbers whose prime factorizations mostly or entirely consist of factors 2. We
  expect them to behave as the conceptual opposite of the prime numbers, thus demonstrating the relevance of considering
  primes in the first place.

We observe a large difference between these two candidate groups in the measured false positive rate
(@fig:round_vs_primes_fpr), the achieved compression ratios (@fig:round_vs_primes_compression), 

#place(
  top,
  scope: "parent",
  float: true,
  grid(
    columns: 3,
    align: top,
    gutter: 2.5mm,
    [
      #figure(
        image("img/round-vs-primes-fpr.pdf"),
        caption: [
          Comparing the false positive rates between bloom filters that hash by taking the remainder after division by prime
          numbers vs. numbers that are divisible by large powers of two.
        ],
      ) <fig:round_vs_primes_fpr>
    ],
    [
      #figure(
        image("img/round-vs-primes-compression.pdf"),
        caption: [
          Comparing compression efficiencies with Brotli between different bloom filters. The point where the primes curve
          reaches 1:1 is where 50% of the bits in the bloom filter are set.
        ],
      ) <fig:round_vs_primes_compression>
    ],
    [
      #figure(
        image("img/avg-steps-vs-rate.pdf"),
        caption: [
          Average number of tree search steps until the solver either solves a position or it detects that it is unsolvable.
        ],
      ) <fig:round_vs_primes_avg_steps>
    ]
  )
)

While the source of these discrepancies hasn't been analysed in depth, it seems reasonable to assume that the reason has
to do with the correlated hash outputs between similar positions in the #rn group.

Note that even when we compare bloom filters with equivalent false positive rates, we observe a difference in the number
of tree search steps required to solve a given start position. For solvable positions, the #rn bloom filters
tend to find the end position more quickly, even with large false positive rates. For unsolvable positions however, they
on average take much longer to determine that the position is indeed unsolvable (@fig:round_vs_primes_avg_steps).
One possible explanation for this could be that the prime number bloom filters have their false positives more evenly
distributed, and therefore don't get stuck in clusters of false positive positions as much.

In order to select the best choice of $m$ for our application, it makes sense to evaluate the performance of the
candidates in end-to-end evaluations. For interactive use of the solver, it is desirable to never make the user wait for
a long time for the result, i.e. we're trying to minimize the worst-case of the solver step count. Our second objective
is to minimize the download size, for which we consider only the Brotli-compressed version.

We evaluate the bloom filters on a dataset of 65,536 solvable and 65,536 unsolvable (but still de-Bruijn-solvable)
positions. Solvers are timed out after about 15,000 steps (see #link(<sec:pseudocode>)[pseudocode section] for details).
The results for the two optimal candidates are listed in @tbl:stats[Table].

Using prime numbers is optimal for a low worst-case step count, whereas with round numbers we can get a much lower
download size, at the cost of some unsolvable positions where the solver times out before detecting them as unsolvable.

#let tablestroke = 0.6pt + rgb("888")
#set table(
  stroke: (x, y) => if x > 0 {
    (left: tablestroke)
  },
  align: (x, y) => (
    if x == 0 and y >= 2 { left + horizon }
    else { center + horizon }
  )
)

#set table.cell(inset: (x: 3pt, y: 4pt))
#show table.cell: set text(size: 8pt)
#show table.cell: set par(justify: false)

#place(
  top,
  scope: "parent",
  float: true,
  [
    #figure(
      caption: [Statistics for the best bloom filters from both groups.],
      table(
        columns: 10,
        table.header(
          table.cell(rowspan: 2, [Group]),
          table.cell(rowspan: 2, [Filter Size $m$ [Bits]]),
          table.cell(rowspan: 2, [Uncompressed Size [Bytes]]),
          table.cell(rowspan: 2, [Compressed Size [Bytes]]),
          table.cell(colspan: 3, [#h(8pt)Solvable Positions]),
          table.cell(colspan: 3, [#h(8pt)Unsolvable Positions]),
          [Average Steps],
          [Max Steps],
          [Completed],
          [Average Steps],
          [Max Steps],
          [Completed],
        ),
        table.hline(stroke: tablestroke),
        [#shortline(rgb(247,113,137)) Round], [268,435,456], [32.00 MiB], strong[2.74 MiB], [31.06], [1,658], [100%], [410.72], [timeout], [99.93%],
        [#shortline(rgb(54,173,164)) Prime], [502,115,651], [59.86 MiB], [8.15 MiB], [31.50], [194], [100%], [6.68], [9,149], strong[100%],
      )
    ) <tbl:stats>
  ]
)


