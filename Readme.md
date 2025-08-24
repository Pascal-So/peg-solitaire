# Peg Solitaire Solver

Trying to optimize a solver in WASM for low RAM and network usage.


## Precompress

```bash
FILE="filter_173378771_norm.bin"
gzip -k -9 "$FILE"
brotli -k -Z "$FILE"

scp "$FILE" server:www/codelis/root/pegsolitaire/
```
