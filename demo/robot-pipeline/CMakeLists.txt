cmake_minimum_required(VERSION 3.14)
project(ritsu_robot_pipeline_demo CXX C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

if(MSVC)
    add_compile_options(/W4 /permissive-)
    add_compile_definitions(_CRT_SECURE_NO_WARNINGS _WINSOCK_DEPRECATED_NO_WARNINGS)
else()
    add_compile_options(-Wall -Wextra -Wpedantic)
endif()

# Interface library for rtclient headers and required platform sockets/multimedia libraries
add_library(rtclient_headers INTERFACE)
target_include_directories(rtclient_headers INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/../../rt-client-c)
if(WIN32)
    target_link_libraries(rtclient_headers INTERFACE ws2_32 winmm)
endif()

# C mock nodes
add_executable(sensor_node sensor_node/sensor_node.c)
target_link_libraries(sensor_node PRIVATE rtclient_headers)

add_executable(odometry_node odometry_node/odometry_node.c)
target_link_libraries(odometry_node PRIVATE rtclient_headers)

add_executable(motor_node motor_node/motor_node.c)
target_link_libraries(motor_node PRIVATE rtclient_headers)
