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

#define RTCLIENT_IMPLEMENTATION
#include "rtclient.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#if defined(_WIN32)
#include <timeapi.h>
#include <windows.h>
#if defined(_MSC_VER)
#pragma comment(lib, "winmm.lib")
#endif /* defined(_MSC_VER) */
#else  /* defined(_WIN32) */
#include <unistd.h>
#endif /* defined(_WIN32) */

// Node identity and cycle configuration constants.
#define NODE_NAME       "motor_node"
#define DEFAULT_CID     50
#define DEFAULT_PORT    10000
#define MAX_CYCLES      25
#define MIN_DURATION_MS 18.0
#define MAX_DURATION_MS 22.0

// Helper function to sleep for a specified duration in milliseconds.
static void sleep_ms(double ms) {
#if defined(_WIN32)
    timeBeginPeriod(1);
    Sleep((DWORD)(ms + 0.5));
    timeEndPeriod(1);
#else
    struct timespec ts;
    ts.tv_sec  = (time_t)(ms / 1000.0);
    ts.tv_nsec = (long)((ms - (double)(ts.tv_sec * 1000)) * 1000000.0);
    nanosleep(&ts, NULL);
#endif
}

// Helper function to query high-resolution timestamp in milliseconds.
static double get_time_ms(void) {
#if defined(_WIN32)
    static LARGE_INTEGER freq;
    static int initialized = 0;
    if (!initialized) {
        QueryPerformanceFrequency(&freq);
        initialized = 1;
    }
    LARGE_INTEGER now;
    QueryPerformanceCounter(&now);
    return (double)now.QuadPart * 1000.0 / (double)freq.QuadPart;
#else
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec * 1000.0 + (double)ts.tv_nsec / 1000000.0;
#endif
}

// Generate pseudo-random float between min_val and max_val.
static double random_range(double min_val, double max_val) {
    double r = (double)rand() / (double)RAND_MAX;
    return min_val + r * (max_val - min_val);
}

// Get wheel servos scenario status message.
static const char* get_scenario_message(int cycle) {
    if (cycle <= 7) {
        return "Wheel servos: FL:300rpm, FR:300rpm, RL:300rpm, RR:300rpm";
    } else if (cycle <= 14) {
        return "Wheel servos: FL:240rpm, FR:340rpm, RL:240rpm, RR:340rpm";
    } else if (cycle <= 20) {
        return "Wheel servos: FL:330rpm, FR:270rpm, RL:330rpm, RR:270rpm";
    } else {
        return "Wheel servos: FL:300rpm, FR:300rpm, RL:300rpm, RR:300rpm";
    }
}

int main(int argc, char* argv[]) {
    const char* host        = "127.0.0.1";
    uint16_t port           = DEFAULT_PORT;
    uint16_t client_id      = DEFAULT_CID;
    double run_cycle_sec    = 0.2;
    double startup_wait_sec = 60.0;

    for (int i = 1; i < argc; ++i) {
        if (strcmp(argv[i], "--host") == 0 && i + 1 < argc) {
            host = argv[++i];
        } else if (strcmp(argv[i], "--port") == 0 && i + 1 < argc) {
            port = (uint16_t)atoi(argv[++i]);
        } else if (strcmp(argv[i], "--client-id") == 0 && i + 1 < argc) {
            client_id = (uint16_t)atoi(argv[++i]);
        }
    }

    srand((unsigned int)time(NULL) ^ (unsigned int)client_id);

    RtClient client;
    if (!rtclient_init(&client, host, port, client_id, run_cycle_sec, startup_wait_sec)) {
        fprintf(stderr, "[%s] Failed to initialize client\n", NODE_NAME);
        return 1;
    }

    if (!rtclient_join(&client)) {
        fprintf(stderr, "[%s] Failed to join server\n", NODE_NAME);
        rtclient_cleanup(&client);
        return 1;
    }
    printf("[%s] Joined server at %s:%u (CID: %u)\n", NODE_NAME, host, port, client_id);

    int proc_count = 0;
    RtMessage msg;

    while (true) {
        rtclient_wait_next(&client, &msg);

        if (msg.mtype == RT_MSG_START) {
            // Gracefully stop on the next START after completing MAX_CYCLES.
            if (proc_count >= MAX_CYCLES) {
                break;
            }
            const char* cycle_str = rtclient_get_extra(&msg, "cycle");
            int cycle             = cycle_str ? (atoi(cycle_str) + 1) : (proc_count + 1);

            double target_duration = random_range(MIN_DURATION_MS, MAX_DURATION_MS);
            double start_time      = get_time_ms();
            sleep_ms(target_duration);
            double elapsed_ms = get_time_ms() - start_time;

            const char* scenario_msg = get_scenario_message(cycle);
            printf("[Cycle %02d/%d] [%-14s] %s (elapsed: %.1fms)\n",
                   cycle, MAX_CYCLES, NODE_NAME, scenario_msg, elapsed_ms);
            fflush(stdout);

            rtclient_notify_done(&client, &msg);
            if (msg.mtype == RT_MSG_ERROR) {
                fprintf(stderr, "[%s] Received ERROR on notify_done\n", NODE_NAME);
                break;
            }
            proc_count++;
        } else if (msg.mtype == RT_MSG_SKIP || msg.mtype == RT_MSG_LATE) {
            continue;
        } else {
            if (proc_count < MAX_CYCLES) {
                fprintf(stderr, "[%s] Exiting due to unexpected message type: %d\n", NODE_NAME, msg.mtype);
            }
            break;
        }
    }

    rtclient_exit(&client);
    rtclient_cleanup(&client);

    printf("\n[Process completed] Press Enter key to close...\n");
    fflush(stdout);
    getchar();

    return 0;
}
