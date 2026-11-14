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
# ==============================================================================

set -e

# Resolve absolute path of demo directory regardless of invocation location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "================================================================================"
echo "  Ritsu Scheduler - iceoryx2 Zero-Copy Pipeline Demo (Linux / macOS)"
echo "================================================================================"

# 1. Build rt-server (always ensure latest release binary)
echo "[1/3] Building rt-server (release)..."
cargo build --manifest-path "$REPO_ROOT/Cargo.toml" -p rt-server --release
SERVER_EXE="$REPO_ROOT/target/release/rt-server"

# 2. Build demo Rust workspace (shared_types, sensor_node, actuator_node)
echo "[2/3] Building iceoryx2-pipeline Rust nodes (release)..."
cargo build --manifest-path "$SCRIPT_DIR/Cargo.toml" --release

SENSOR_EXE="$SCRIPT_DIR/target/release/sensor_node"
if [ ! -f "$SENSOR_EXE" ] && [ -f "$SCRIPT_DIR/target/release/sensor-node" ]; then
    SENSOR_EXE="$SCRIPT_DIR/target/release/sensor-node"
fi

ACTUATOR_EXE="$SCRIPT_DIR/target/release/actuator_node"
if [ ! -f "$ACTUATOR_EXE" ] && [ -f "$SCRIPT_DIR/target/release/actuator-node" ]; then
    ACTUATOR_EXE="$SCRIPT_DIR/target/release/actuator-node"
fi

# 3. Prepare isolated Python virtual environment (.venv) before launching nodes
# Pre-building .venv here avoids consuming RtClient startup_wait_sec during launch.
echo "[3/3] Preparing isolated Python venv (.venv)..."
VENV_DIR="$SCRIPT_DIR/.venv"
VENV_PYTHON="$VENV_DIR/bin/python"

if [ ! -x "$VENV_PYTHON" ]; then
    PYTHON_CMD="python3"
    if ! command -v python3 >/dev/null 2>&1; then
        PYTHON_CMD="python"
    fi
    echo "  Creating virtual environment at $VENV_DIR..."
    "$PYTHON_CMD" -m venv "$VENV_DIR"
fi

if "$VENV_PYTHON" -c "import importlib.metadata as m, sys; dists = {d.metadata['Name'].lower(): d.version for d in m.distributions() if d.metadata.get('Name')}; sys.exit(0 if dists.get('iceoryx2') == '0.10.0' else 1)"; then
    echo "  iceoryx2==0.10.0 already installed in .venv (skipping pip install)."
else
    echo "  Installing requirements into isolated .venv..."
    "$VENV_PYTHON" -m pip install -r "$SCRIPT_DIR/ai_vision_node/requirements.txt"
fi

echo ""
echo "All components built and prepared successfully!"
echo "Launching server and 3 client nodes..."
echo ""

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
        xterm -T "$title" -e bash -c "cd '$SCRIPT_DIR' && $cmd; exec bash" &
    else
        echo "Starting [$title] in background..."
        (cd "$SCRIPT_DIR" && eval "$cmd") &
    fi
}

# Launch Server
launch_node "[Ritsu Server]" "'$SERVER_EXE' --config '$SCRIPT_DIR/config.toml'"
sleep 1

# Launch 3 Client Nodes (10 -> 20 -> 30)
launch_node "[sensor_node (Rust)]" "'$SENSOR_EXE'"
sleep 0.2

launch_node "[ai_vision_node (Python)]" "'$VENV_PYTHON' '$SCRIPT_DIR/ai_vision_node/ai_vision_node.py'"
sleep 0.2

launch_node "[actuator_node (Rust)]" "'$ACTUATOR_EXE'"

echo "--------------------------------------------------------------------------------"
echo "  Pipeline running! Check each node window to observe the 33-cycle sequence."
echo "  When completed, logs will be saved to: demo/iceoryx2-pipeline/logs/rt-server.log"
echo "--------------------------------------------------------------------------------"
