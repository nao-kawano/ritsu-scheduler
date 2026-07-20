#!/bin/bash
cd "$(dirname "$0")"
cmake -B build -S .
cmake --build build --config Release --clean-first

