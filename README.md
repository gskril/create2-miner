```bash
cd rs

export FACTORY="0x13b0D85CcB8bf860b6b79AF3029fCA081AE9beF2"
export INIT_CODE_HASH="<HASH_OF_YOUR_CONTRACT_INIT_CODE_GOES_HERE>"
export PREFIX=00000000
cargo run -- create2 $FACTORY $INIT_CODE_HASH $PREFIX --gpu.threads 256 --gpu.thread-groups 131072
```
