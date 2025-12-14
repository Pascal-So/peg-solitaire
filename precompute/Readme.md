# Code for precomputing & evaluating

Here we compute for every possible game position whether or not the game is solvable from there. This set of positions is then stored in a bloom filter.

Note that the code in here is the least structured part of the project. For running different tasks, you'll have to uncomment some sections in `fn main()`.

```bash
# bash snippet for compressing the bloom filters
for f in *.bin; do
    if [[ ! -f $f.br ]]; then
        brotli -k -Z $f
    fi
done
```
