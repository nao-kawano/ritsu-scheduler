#!/bin/bash
cd "$(dirname "$0")"
mkdir -p build
g++ -std=c++11 -I.. example_cpp.cpp -o build/example_cpp
