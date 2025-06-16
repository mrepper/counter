A simple file-backed tally counter

```bash
cargo build -r
./target/release/counter /tmp/count1
```
```
Count: 5    [+/-/q]
```
(then press '+' or '-' to increment/decrement the counter)

Increment keys:
- `+`
- `=`
- Space

Decrement keys:
- `-`
- `_`
- Backspace

Quit keys:
- `q`
- `Q`
- ctrl-c
