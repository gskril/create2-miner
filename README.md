This branch is optimized to run on Apple computers. See the `create2-cuda` branch for a version that runs on other platforms.

---

Clone the repo

```bash
git clone https://github.com/gskril/create2-miner.git
```

Set your CREATE2 factory address, init code hash and desired prefix as environment variables

```bash
export FACTORY="0x13b0D85CcB8bf860b6b79AF3029fCA081AE9beF2"
export INIT_CODE_HASH="<HASH_OF_YOUR_CONTRACT_INIT_CODE_GOES_HERE>"
export PREFIX=00000000
```

Run the program

```bash
cd create2-miner/rs
cargo run -- create2 $FACTORY $INIT_CODE_HASH $PREFIX --gpu.threads 256 --gpu.thread-groups 131072
```
