`eclair` aspires to be a set of tools to manipulate the outputs of the Eclipse reservoir simulator.

At this point all it can do is convert `UNSMRY` files into a MessagePack format.

## Building

- Install Rust and Cargo. Refer to https://rustup.rs/ for rust installation;
- Inside the repo run `cargo build` or `cargo build --release`;

## Running
- Inside the folder run `cargo run <INPUT>` or `cargo run --release <INPUT>`, where `<INPUT>` is either an `SMSPEC` or an `UNSMRY` file. File extension can be omitted, e.g.

    ```
    cargo run assets/SPE10
    ```
## Inspecting results
- Look inside the `plotting` folder for a sample Jupyter notebook that shows how to load data in Python and plot what you want. The `docs` folder contains a brief description of the MessagePack layout. The data is loaded in Python as a tree of nested dictionaries, so it's rather easy to inspect its contents.
