// Copyright 2026 Naoyuki Kawano
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// =============================================================================
#include <chrono>
#include <cstdio>
#include <ctime>
#include <iostream>
#include <string>
#include <thread>

// Define RTCLIENT_IMPLEMENTATION to include the implementation code from rtclient.h.
#define RTCLIENT_IMPLEMENTATION
#include "rtclient.h"

// Helper function to output timestamped log messages matching the format used by Python/Rust clients (HH:MM:SS.mmm).
void log_msg(const std::string& message) {
    auto now        = std::chrono::system_clock::now();
    auto time_t_now = std::chrono::system_clock::to_time_t(now);
    auto duration   = now.time_since_epoch();
    auto millis     = std::chrono::duration_cast<std::chrono::milliseconds>(duration).count() % 1000;

    char buf[64];
    std::strftime(buf, sizeof(buf), "%H:%M:%S", std::localtime(&time_t_now));
    std::printf("%s.%03d %s\n", buf, (int)millis, message.c_str());
}

// Maps RtMessageType values to their string representations.
const char* get_msg_type_name(RtMessageType mtype) {
    switch (mtype) {
        case RT_MSG_JOIN:
            return "JOIN";
        case RT_MSG_READY:
            return "READY";
        case RT_MSG_DONE:
            return "DONE";
        case RT_MSG_EXIT:
            return "EXIT";
        case RT_MSG_JOINED:
            return "JOINED";
        case RT_MSG_START:
            return "START";
        case RT_MSG_OK:
            return "OK";
        case RT_MSG_SKIP:
            return "SKIP";
        case RT_MSG_LATE:
            return "LATE";
        case RT_MSG_ERROR:
            return "ERROR";
        default:
            return "UNKNOWN";
    }
}

int main(int argc, char* argv[]) {
    // Set default configuration parameters.
    std::string host        = "127.0.0.1";
    uint16_t port           = 7878;
    uint16_t client_id      = 1;
    double run_cycle_sec    = 2.0;
    double startup_wait_sec = 60.0;
    double proc_time_sec    = 0.4;
    int proc_count_max      = 5;

    // Simple parser for command line arguments.
    for (int i = 1; i < argc; ++i) {
        std::string arg = argv[i];
        if (arg == "--host" && i + 1 < argc)
            host = argv[++i];
        else if (arg == "--port" && i + 1 < argc)
            port = std::stoi(argv[++i]);
        else if (arg == "--client_id" && i + 1 < argc)
            client_id = std::stoi(argv[++i]);
        else if (arg == "--run_cycle_sec" && i + 1 < argc)
            run_cycle_sec = std::stod(argv[++i]);
        else if (arg == "--startup_wait_sec" && i + 1 < argc)
            startup_wait_sec = std::stod(argv[++i]);
        else if (arg == "--proc_time_sec" && i + 1 < argc)
            proc_time_sec = std::stod(argv[++i]);
        else if (arg == "--proc_count" && i + 1 < argc)
            proc_count_max = std::stoi(argv[++i]);
    }

    RtClient client;
    if (!rtclient_init(&client, host.c_str(), port, client_id, run_cycle_sec, startup_wait_sec)) {
        std::cerr << "Failed to initialize client" << std::endl;
        return -1;
    }

    if (!rtclient_join(&client)) {
        log_msg("failed to join, exit");
        rtclient_cleanup(&client);
        return -1;
    }
    log_msg("client joined");

    int proc_count = 0;
    while (true) {
        RtMessage msg         = rtclient_wait_next(&client);
        const char* cycle_str = rtclient_get_extra(&msg, "cycle");
        std::string cycle_val = cycle_str ? cycle_str : "N/A";

        // Log the receipt of a message from wait_next.
        log_msg("wait_next done: " + std::string(get_msg_type_name(msg.mtype)) + ", cycle=" + cycle_val + ", count=" + std::to_string(proc_count));

        if (msg.mtype == RT_MSG_START) {
            char proc_time_buf[32];
            std::sprintf(proc_time_buf, "%.3f", proc_time_sec);
            log_msg("got START, do some process with " + std::string(proc_time_buf) + " sec ...");

            // Simulate workload processing with sleep.
            std::this_thread::sleep_for(std::chrono::milliseconds((int)(proc_time_sec * 1000)));

            rtclient_notify_done(&client);
        } else if (msg.mtype == RT_MSG_SKIP) {
            log_msg("got SKIP, retry");
        } else if (msg.mtype == RT_MSG_LATE) {
            log_msg("got LATE, retry");
        } else {
            const char* reason_str = rtclient_get_extra(&msg, "reason");
            std::string reason_val = reason_str ? reason_str : "N/A";
            log_msg("got ERROR, going to exit (reason=" + reason_val + ", cid=" + std::to_string(msg.cid) + ")");
            break;
        }

        proc_count++;
        if (proc_count >= proc_count_max) {
            log_msg("completed, going to exit");
            break;
        }
    }

    rtclient_exit(&client);
    rtclient_cleanup(&client);

    return 0;
}
