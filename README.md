# rust-chain

a simple blockchain implementation in rust, inspired by bitcoin's proof-of-work (pow) system.

## features

- pow consensus mechanism
- dynamic difficulty adjustment
- block reward halving
- merkle tree for transaction verification
- cli interface for interacting with the blockchain

## usage

to build and run the project:

```sh
cargo build
cargo run -- [command] [options]
```

available commands:

- `mine`: mine new blocks
  - options:
    - `-c, --count <COUNT>`: number of blocks to mine (default: 1)
    - `--dump`: dump all serialized objects in the chain
- `drop`: drop all blocks from the chain

examples:

```sh
cargo run -- mine -c 5
cargo run -- mine --count 3 --dump
cargo run -- drop
```

## note

this implementation is for educational purposes and does not include persistent storage. all data is kept in memory and will be lost when the program terminates.

## license

mit

