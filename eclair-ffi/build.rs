fn main() {
    cxx_build::bridge("src/eclair_ffi.rs")
        .flag_if_supported("-std=c++17")
        .compile("eclair-ffi");

    println!("cargo:rerun-if-changed=src/lib.rs");
}
