cmake_minimum_required(VERSION 3.17)
project(eclair_gui)

set(CMAKE_CXX_STANDARD 17)

set(CXXBRIDGE_BUILD_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../target/cxxbridge")

# Add GUI
include(FetchContent)
FetchContent_Declare(mahi-gui GIT_REPOSITORY https://github.com/mahilab/mahi-gui.git)
FetchContent_MakeAvailable(mahi-gui)

add_executable(eclair_gui main.cpp utils.cpp "${CXXBRIDGE_BUILD_DIR}/eclair-ffi/src/eclair_ffi.rs.cc")

# Point to the folder where cargo places the cxx-generated C++ code.
target_include_directories(
        eclair_gui PRIVATE "${CXXBRIDGE_BUILD_DIR}/eclair-ffi/src" "${CXXBRIDGE_BUILD_DIR}"
)

# Link eclair-ffi and ZeroMQ. When generating the static library from eclair, it does not include any C definitions
# from external libraries, so we need to link to ZeroMQ manually.
target_link_directories(eclair_gui PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/../target/release/" "/usr/local/lib")
target_link_libraries(eclair_gui PRIVATE "libeclair_ffi.a" "libzmq.a" mahi::gui)
