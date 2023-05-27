# MultiDuino

## Disclaimer
This repository is **only** for educational purposes. You may get your account banned! I am not responsible for any damages done to your account by using this tool. 

## Purpose
This tool has been built to learn how to use Rust's features to make an efficient PC miner for the Duino-Coin network. This includes multi account with multiple miners mining, which will get you **banned**, and a nice dashboard with stats.

## Usage
To build this tool, run `cargo build --release`.

You will then find the executable in `./target/release/` called `multi-duino`.

For convenience, move the file to the root of the project using `mv target/release/multi-duino .`. Then go back to the project root.

Before you run this, rename `conf.toml.example` to `conf.toml` and enter your details. 

Last but not least, run the tool using `./multi-duino`.