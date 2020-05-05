`eclair` aspires to be a set of tools to manipulate the outputs of the Eclipse reservoir simulator.

At this point all it can do is convert `UNSMRY` files into the MessagePack format.

## Building

- Install Rust and Cargo. Refer to https://rustup.rs/ for rust installation;
- Inside the repo run `cargo build --all-features` or `cargo build --all-features --release`;

## Running
- Inside the repo, run `cargo install --path .`, which will make `eclair` discoverable on your system. Then run

    ```
    eclair <INPUT>
    ```

    where `<INPUT>` is either an `SMSPEC` or an `UNSMRY` file. File extension can be omitted, e.g.

    ```
    eclair SPE10
    ```

## Inspecting results
- Inside the `plotting` folder there is Python code that uses `ipywidgets` in a Jupyter notebook to load and display data from the MessagePack format. The `docs` folder contains a brief description of the MessagePack layout. The data is loaded in Python as a tree of nested dictionaries, so it's rather easy to inspect its contents manually. For this you'd only need to call the `load_summary(file_path)` method from the `data_manager.py` module.
