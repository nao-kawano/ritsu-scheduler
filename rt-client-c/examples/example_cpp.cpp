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

#if defined(_WIN32)
#include <timeapi.h>
#if defined(_MSC_VER)
#pragma comment(lib, "winmm.lib")
#endif /* defined(_MSC_VER) */
#endif /* defined(_WIN32) */

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
        // clang-format off
        std::string arg = argv[i];
        if (arg == "--host" && i + 1 < argc) host = argv[++i];
        else if (arg == "--port" && i + 1 < argc) port = static_cast<uint16_t>(std::stoi(argv[++i]));
        else if (arg == "--client-id" && i + 1 < argc) client_id = static_cast<uint16_t>(std::stoi(argv[++i]));
        else if (arg == "--run-cycle-sec" && i + 1 < argc) run_cycle_sec = std::stod(argv[++i]);
        else if (arg == "--startup-wait-sec" && i + 1 < argc) startup_wait_sec = std::stod(argv[++i]);
        else if (arg == "--proc-time-sec" && i + 1 < argc) proc_time_sec = std::stod(argv[++i]);
        else if (arg == "--proc-count" && i + 1 < argc) proc_count_max = std::stoi(argv[++i]);
        // clang-format on
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
    RtMessage msg;
    while (true) {
        rtclient_wait_next(&client, &msg);
        const char* cycle_str = rtclient_get_extra(&msg, "cycle");
        std::string cycle_val = cycle_str ? cycle_str : "N/A";
        log_msg("wait_next done: " + std::string(rtclient_get_msg_str(msg.mtype)) + ", cycle=" + cycle_val + ", count=" + std::to_string(proc_count));
        if (msg.mtype == RT_MSG_START) {
            char proc_time_buf[32];
            std::sprintf(proc_time_buf, "%.3f", proc_time_sec);
            log_msg("got START, do some process with " + std::string(proc_time_buf) + " sec ...");

#if defined(_WIN32)
            timeBeginPeriod(1);
#endif /* defined(_WIN32) */
            // Simulate workload processing with sleep.
            std::this_thread::sleep_for(std::chrono::milliseconds((int)(proc_time_sec * 1000)));
#if defined(_WIN32)
            timeEndPeriod(1);
#endif /* defined(_WIN32) */

            // Client must send DONE and send READY (wait_next).
            rtclient_notify_done(&client, &msg);
            if (msg.mtype == RT_MSG_ERROR) {
                log_msg("failed to notify done or got ERROR, exit");
                break;
            }
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
