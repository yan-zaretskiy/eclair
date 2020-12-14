cmake_minimum_required(VERSION 3.17)
project(eclair_gui)

set(CMAKE_CXX_STANDARD 17)

set(CXXBRIDGE_BUILD_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../target/cxxbridge")

# Add mahi-gui
add_subdirectory("${CMAKE_CURRENT_SOURCE_DIR}/deps/mahi-gui")

set(ECLAIR_SRC
    main.cpp
    EclairApp.cpp
    utils.cpp
    deps/ImGuiFileBrowser.cpp
)

add_executable(eclair_gui ${ECLAIR_SRC} "${CXXBRIDGE_BUILD_DIR}/eclair-ffi/src/eclair_ffi.rs.cc")

# Point to the folder where cargo places the cxx-generated C++ code.
target_include_directories(
    eclair_gui PRIVATE deps "${CXXBRIDGE_BUILD_DIR}/eclair-ffi/src" "${CXXBRIDGE_BUILD_DIR}/rust"
)

# Link eclair-ffi and ZeroMQ. When generating the static library from eclair, it does not include any C definitions
# from external libraries, so we need to link to ZeroMQ manually.

# Look for the ZeroMQ lib that was built by Cargo.
if (WIN32)
    file(GLOB_RECURSE CARGO_ZMQ_BUILD "${CXXBRIDGE_BUILD_DIR}/../release/build/*/out/lib/zmq.lib")
else ()
    file(GLOB_RECURSE CARGO_ZMQ_BUILD "${CXXBRIDGE_BUILD_DIR}/../release/build/*/out/lib/libzmq.a")
endif ()

find_library(ECLAIR_FFI eclair_ffi PATHS "${CMAKE_CURRENT_SOURCE_DIR}/../target/release/")

message("ZeroMQ library: ${CARGO_ZMQ_BUILD}")
message("Eclair FFI library: ${ECLAIR_FFI}")

target_link_libraries(eclair_gui PRIVATE "${ECLAIR_FFI}" "${CARGO_ZMQ_BUILD}" mahi::gui)

# For ZeroMQ on Windows
if (WIN32)
    target_link_libraries(eclair_gui PRIVATE Ws2_32.lib iphlpapi.lib userenv.lib)
endif ()