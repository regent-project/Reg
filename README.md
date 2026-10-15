# Reg : a lightweight CLI automation tool

> A lightning-fast, single-binary infrastructure automation and configuration tool built on top of [regent-sdk](https://github.com/regent-project/regent-sdk).



## Overview

**RegRunner** is an all-in-one lightweight 🤖 automation and configuration tool written in 🦀 Rust and powered by [`regent-sdk`](https://github.com/regent-project/regent-sdk). Usage is as simple as possible. Write an inventory file and an expected state file using YAML, and run it using **reg**.


## Installation

For now installation is only available as local build ([Rust required](https://rust-lang.org/tools/install/))

```bash
git clone https://github.com/regent-project/reg
cargo install --path reg
```

## Usage

```bash
# Help
reg -h
# Assess compliance
reg -i /path/to/inventory.yaml -s /path/to/expected_state_description.yaml -m assess 
# Enforce compliance
reg -i /path/to/inventory.yaml -s /path/to/expected_state_description.yaml -m enforce 
# Validate the inventory and/or the expected state description (YAML syntax and deserialization)
reg -i /path/to/inventory.yaml -m validate
reg -i /path/to/inventory.yaml -s /path/to/expected_state_description.yaml -m validate
```