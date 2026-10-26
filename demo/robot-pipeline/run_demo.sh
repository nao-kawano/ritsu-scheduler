#!/usr/bin/env bash
# Copyright 2026 Naoyuki Kawano
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
# =============================================================================

set -e

# Resolve absolute path of demo directory regardless of invocation location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "================================================================================"
echo "  Ritsu Scheduler - Robot Pipeline Demo (Linux / macOS)"
echo "================================================================================"

# 1. Build rt-server (always ensure latest release binary)
echo "[1/3] Building rt-server (release)..."
cargo build --manifest-path "$REPO_ROOT/Cargo.toml" -p rt-server --release
SERVER_EXE="$REPO_ROOT/target/release/rt-server"

# 2. Build C mock nodes via CMake (with --clean-first for clean build safety)
echo "[2/3] Building C mock nodes (sensor, odometry, motor)..."
cmake -B "$SCRIPT_DIR/build" -S "$SCRIPT_DIR"
cmake --build "$SCRIPT_DIR/build" --config Release --clean-first

SENSOR_EXE="$SCRIPT_DIR/build/sensor_node"
[ -f "$SCRIPT_DIR/build/Release/sensor_node" ] && SENSOR_EXE="$SCRIPT_DIR/build/Release/sensor_node"

ODOMETRY_EXE="$SCRIPT_DIR/build/odometry_node"
[ -f "$SCRIPT_DIR/build/Release/odometry_node" ] && ODOMETRY_EXE="$SCRIPT_DIR/build/Release/odometry_node"

MOTOR_EXE="$SCRIPT_DIR/build/motor_node"
[ -f "$SCRIPT_DIR/build/Release/motor_node" ] && MOTOR_EXE="$SCRIPT_DIR/build/Release/motor_node"

# 3. Build Rust mock node (planner_node)
echo "[3/3] Building Rust mock node (planner_node)..."
cargo build --manifest-path "$SCRIPT_DIR/planner_node/Cargo.toml" --release
PLANNER_EXE="$SCRIPT_DIR/planner_node/target/release/planner-node"

echo ""
echo "All components built successfully!"
echo "Launching server and 5 client nodes..."

mkdir -p "$SCRIPT_DIR/logs"

# Function to spawn process in a new terminal window or fallback to background
launch_node() {
    local title="$1"
    local cmd="$2"

    if [ "$(uname)" = "Darwin" ]; then
        # macOS Terminal
        osascript -e "tell application \"Terminal\" to do script \"cd \\\"$SCRIPT_DIR\\\" && echo -n -e \\\"\\\\033]0;$title\\\\007\\\" && $cmd\"" >/dev/null 2>&1 || eval "cd '$SCRIPT_DIR' && $cmd &"
    elif command -v gnome-terminal >/dev/null 2>&1; then
        gnome-terminal --title="$title" -- bash -c "cd '$SCRIPT_DIR' && $cmd; exec bash"
    elif command -v xterm >/dev/null 2>&1; then
        xterm -T "$title" -e "cd '$SCRIPT_DIR' && $cmd" &
    else
        echo "Starting [$title] in background..."
        (cd "$SCRIPT_DIR" && eval "$cmd") &
    fi
}

# Launch Server
launch_node "[Ritsu Server]" "$SERVER_EXE --config '$SCRIPT_DIR/config.toml'"
sleep 1

# Launch 5 Client Nodes
launch_node "[sensor_node (C++)]" "$SENSOR_EXE --port 10000 --client-id 10"
sleep 0.2

PYTHON_CMD="python3"
if ! command -v python3 >/dev/null 2>&1; then
    PYTHON_CMD="python"
fi
launch_node "[ai_vision_node (Python)]" "$PYTHON_CMD '$SCRIPT_DIR/ai_vision_node/ai_vision_node.py' --port 10000 --client-id 20"
sleep 0.2

launch_node "[odometry_node (C++)]" "$ODOMETRY_EXE --port 10000 --client-id 30"
sleep 0.2

launch_node "[planner_node (Rust)]" "$PLANNER_EXE --port 10000 --client-id 40"
sleep 0.2

launch_node "[motor_node (C++)]" "$MOTOR_EXE --port 10000 --client-id 50"

echo "--------------------------------------------------------------------------------"
echo "  Pipeline running! Check each node window to observe the 25-cycle sequence."
echo "  When completed, logs will be saved to: demo/robot-pipeline/logs/rt-server.log"
echo "--------------------------------------------------------------------------------"
