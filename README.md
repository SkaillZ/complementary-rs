# complementary-rs

Rust rewrite of [Complementary](https://github.com/complementaries/complementary). The original game and the assets used were made by Annabelle Nissl, Kajetan Hammerle and Rene Buchmayer.

## Running the game

To build and start the game, run `cargo run --bin complementary`.

## Data conversion tool

This repository also contains a tool `complementary_data_converter` for converting binary assets from the C++ version to JSON files. The path to the original `assets` folder must be passed to the binary:

```
cargo run --bin complementary_data_converter /path/to/complementary/assets
```

This is optional since the converted assets are committed to the repository.
