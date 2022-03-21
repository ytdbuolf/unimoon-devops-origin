<p align="center">
  <a href="https://unimoon.io">
    <img alt="Unimoon" src="https://images.squarespace-cdn.com/content/v1/6198d1285cae080ff834da1a/fad06e2c-b271-46b7-ad21-dcb50a0b8b16/UNIMOON_LOGO_WHITE.png?format=1500w" width="250" />
  </a>
  <Unimoon>
</p>

[![Solana crate](https://img.shields.io/crates/v/solana-core.svg)](https://crates.io/crates/solana-core)
[![Solana documentation](https://docs.rs/solana-core/badge.svg)](https://docs.rs/solana-core)
[![Build status](https://badge.buildkite.com/8cc350de251d61483db98bdfc895b9ea0ac8ffa4a32ee850ed.svg?branch=master)](https://buildkite.com/solana-labs/solana/builds?branch=master)
[![codecov](https://codecov.io/gh/solana-labs/solana/branch/master/graph/badge.svg)](https://codecov.io/gh/solana-labs/solana)

# Building

## **1. Install rustc, cargo and rustfmt.**

```bash
$ curl https://sh.rustup.rs -sSf | sh
$ source $HOME/.cargo/env
$ rustup component add rustfmt
```

When building the master branch, please make sure you are using the latest stable rust version by running:

```bash
$ rustup update
```

When building a specific release branch, you should check the rust version in `ci/rust-version.sh` and if necessary, install that version by running:
```bash
$ rustup install VERSION
```
Note that if this is not the latest rust version on your machine, cargo commands may require an [override](https://rust-lang.github.io/rustup/overrides.html) in order to use the correct version.

On Linux systems you may need to install libssl-dev, pkg-config, zlib1g-dev, etc.  On Ubuntu:

```bash
$ sudo apt-get update
$ sudo apt-get install libssl-dev libudev-dev pkg-config zlib1g-dev llvm clang make
```

## **2. Download the source code.**

```bash
$ git clone https://github.com/UNIMOONnetwork/unimoon-chain-devops.git
$ cd unimoon-chain-devops
```

## **3. Build.**

```bash
$ cargo build
```

# Testing

**Run the test suite:**

```bash
$ cargo test
```

### Starting a local testnet
Start your own testnet locally, instructions are in the [online docs](https://docs.solana.com/cluster/bench-tps).

### Accessing the remote development cluster
* `devnet` - stable public cluster for development accessible via
devnet.solana.com. Runs 24/7. Learn more about the [public clusters](https://docs.solana.com/clusters)

# Benchmarking

First, install the nightly build of rustc. `cargo bench` requires the use of the
unstable features only available in the nightly build.

```bash
$ rustup install nightly
```

Run the benchmarks:

```bash
$ cargo +nightly bench
```

# Release Process

The release process for this project is described [here](RELEASE.md).
