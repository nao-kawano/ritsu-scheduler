cmake_minimum_required(VERSION 3.14)
project(rt_client_c_examples CXX C)

set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Configure compiler flags with strict warnings and disable MSVC CRT deprecation warnings.
if(MSVC)
    add_compile_options(/W4 /permissive-)
    add_compile_definitions(_CRT_SECURE_NO_WARNINGS _WINSOCK_DEPRECATED_NO_WARNINGS)
else()
    add_compile_options(-Wall -Wextra -Wpedantic)
endif()

# Define reusable header-only interface target for rtclient library dependencies.
add_library(rtclient_headers INTERFACE)
target_include_directories(rtclient_headers INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/..)
if(WIN32)
    target_link_libraries(rtclient_headers INTERFACE ws2_32 winmm)
endif()

# Target: example_cpp
add_executable(example_cpp example_cpp.cpp)
target_link_libraries(example_cpp PRIVATE rtclient_headers)
