```bash
cd rs

export FACTORY=0x13b0D85CcB8bf860b6b79AF3029fCA081AE9beF2
export INIT_CODE_HASH=0x73e13aa0a6521b60c6ff2b14ce799a224b103b04b6a037bb9f00fa3b10279d7e
export PREFIX=dddddddddd
cargo run -- create2 $FACTORY $INIT_CODE_HASH $PREFIX --gpu.threads 256 --gpu.thread-groups 131072
```
