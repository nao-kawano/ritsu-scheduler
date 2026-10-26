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
#define NODE_NAME       "odometry_node"
#define DEFAULT_CID     30
#define DEFAULT_PORT    10000
#define MAX_CYCLES      25
#define MIN_DURATION_MS 18.0
#define MAX_DURATION_MS 23.0

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

// Format odometry message according to the 25-cycle scenario.
static void format_odometry_msg(int cycle, char* buf, size_t buf_size) {
    double x   = 0.0;
    double y   = 0.0;
    double yaw = 0.0;

    if (cycle <= 7) {
        x   = (double)cycle * 0.10;
        y   = 0.00;
        yaw = 0.00;
        snprintf(buf, buf_size, "Pose updated: x=%.2fm, y=%.2fm, yaw=%.2frad", x, y, yaw);
    } else if (cycle <= 14) {
        // Phase 2: Obstacle avoidance manoeuvre
        static const double x_coords[] = {0.80, 0.90, 1.00, 1.10, 1.20, 1.25, 1.30};
        static const double y_coords[] = {0.05, 0.08, 0.10, 0.13, 0.16, 0.18, 0.20};
        static const double yaws[]     = {0.02, 0.05, 0.08, 0.11, 0.13, 0.15, 0.15};

        int idx = cycle - 8;
        if (idx < 0) idx = 0;
        if (idx > 6) idx = 6;
        x   = x_coords[idx];
        y   = y_coords[idx];
        yaw = yaws[idx];
        snprintf(buf, buf_size, "Pose updated: x=%.2fm, y=%.2fm, yaw=%+.2frad", x, y, yaw);
    } else if (cycle <= 20) {
        // Phase 3: Passing and returning to original lane
        static const double x_coords[] = {1.40, 1.50, 1.60, 1.70, 1.80, 1.90};
        static const double y_coords[] = {0.35, 0.33, 0.31, 0.29, 0.27, 0.25};
        static const double yaws[]     = {-0.05, -0.08, -0.10, -0.10, -0.08, -0.05};

        int idx = cycle - 15;
        if (idx < 0) idx = 0;
        if (idx > 5) idx = 5;
        x   = x_coords[idx];
        y   = y_coords[idx];
        yaw = yaws[idx];
        snprintf(buf, buf_size, "Pose updated: x=%.2fm, y=%.2fm, yaw=%+.2frad", x, y, yaw);
    } else {
        // Phase 4: Cruising resumed
        int step = cycle - 21;
        x        = 2.00 + (double)step * 0.10;
        y        = 0.00;
        yaw      = 0.00;
        snprintf(buf, buf_size, "Pose updated: x=%.2fm, y=%.2fm, yaw=%.2frad", x, y, yaw);
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

            char status_buf[128];
            format_odometry_msg(cycle, status_buf, sizeof(status_buf));

            printf("[Cycle %02d/%d] [%-14s] %s (elapsed: %.1fms)\n",
                   cycle, MAX_CYCLES, NODE_NAME, status_buf, elapsed_ms);
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
