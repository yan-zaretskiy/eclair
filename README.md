## Introduction

Eclair is a GUI application to visualize summary outputs of the oil&gas reservoir simulators that write out results in the Eclipse binary format:

![Screenshot](https://raw.githubusercontent.com/mindv0rtex/eclair/master/assets/screenshot.png)

## Building Instructions

Eclair consists of the Rust back-end and the C++ front-end. To build the application, one first need to compile the
backend, using

```
cargo build --release --all
```

Afterwards one needs to build the CMake project inside the `eclair-gui` folder. The `CMakeLists.txt` is setup to
automatically discover the necessary Rust files, so the build instructions are simply:

```
cd eclair-gui
mkdir build && cd build
cmake -DCMAKE_BUILD_TYPE=Release ..
make
```

On Windows, you might want to add `-GNinja` to the `cmake` invocation and then build with `ninja` instead of `make`.
