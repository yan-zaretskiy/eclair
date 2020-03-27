`eclair` aspires to be a set of tools to manipulate the outputs of the Eclipse reservoir simulator.

At this point all it can do is convert `UNSMRY` files into a MessagePack format.

## Building

- Install Rust and Cargo (version 1.40.0 or above). Refer to https://rustup.rs/ for rust installation;
- Inside the repo run `cargo build` or `cargo build --release`;

## Running
- Inside the folder run `cargo run <INPUT>` or `cargo run --release <INPUT>`, where `<INPUT>` is either an `SMSPEC` or an `UNSMRY` file. File extension can be omitted, e.g.

    ```
    cargo run assets/SPE10
    ```
