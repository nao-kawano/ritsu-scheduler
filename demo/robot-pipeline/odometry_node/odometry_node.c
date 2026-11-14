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

// -----------------------------------------------------------------------------
// Node Identity & Scheduler Constants
// -----------------------------------------------------------------------------
#define NODE_NAME        "odometry_node"
#define HOST             "127.0.0.1"
#define PORT             7878
#define CLIENT_ID        30
#define INTERVAL_SEC     0.2
#define STARTUP_WAIT_SEC 60.0

// Simulated odometry integration duration range in milliseconds.
#define MIN_DURATION_MS 18.0
#define MAX_DURATION_MS 23.0

// -----------------------------------------------------------------------------
// Timing, Scenario & Terminal Helpers
// -----------------------------------------------------------------------------

// Sleep for the specified duration in milliseconds.
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

// Return a high-resolution monotonic timestamp in milliseconds.
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

// Return a pseudo-random value within [min_val, max_val].
static double random_range(double min_val, double max_val) {
    double r = (double)rand() / (double)RAND_MAX;
    return min_val + r * (max_val - min_val);
}

// Format the odometry pose message for the given 0-indexed cycle.
static void format_odometry_msg(int running_cycle, char* buf, size_t buf_size) {
    double x   = 0.0;
    double y   = 0.0;
    double yaw = 0.0;

    if (running_cycle <= 6) {
        // Phase 1: Initial cruising
        x   = (double)(running_cycle + 1) * 0.10;
        y   = 0.00;
        yaw = 0.00;
        snprintf(buf, buf_size, "Pose updated: x=%.2fm, y=%.2fm, yaw=%.2frad", x, y, yaw);
    } else if (running_cycle <= 13) {
        // Phase 2: Obstacle avoidance manoeuvre
        static const double x_coords[] = {0.80, 0.90, 1.00, 1.10, 1.20, 1.25, 1.30};
        static const double y_coords[] = {0.05, 0.08, 0.10, 0.13, 0.16, 0.18, 0.20};
        static const double yaws[]     = {0.02, 0.05, 0.08, 0.11, 0.13, 0.15, 0.15};

        int idx = running_cycle - 7;
        if (idx < 0) idx = 0;
        if (idx > 6) idx = 6;
        x   = x_coords[idx];
        y   = y_coords[idx];
        yaw = yaws[idx];
        snprintf(buf, buf_size, "Pose updated: x=%.2fm, y=%.2fm, yaw=%+.2frad", x, y, yaw);
    } else if (running_cycle <= 19) {
        // Phase 3: Passing and returning to original lane
        static const double x_coords[] = {1.40, 1.50, 1.60, 1.70, 1.80, 1.90};
        static const double y_coords[] = {0.35, 0.33, 0.31, 0.29, 0.27, 0.25};
        static const double yaws[]     = {-0.05, -0.08, -0.10, -0.10, -0.08, -0.05};

        int idx = running_cycle - 14;
        if (idx < 0) idx = 0;
        if (idx > 5) idx = 5;
        x   = x_coords[idx];
        y   = y_coords[idx];
        yaw = yaws[idx];
        snprintf(buf, buf_size, "Pose updated: x=%.2fm, y=%.2fm, yaw=%+.2frad", x, y, yaw);
    } else {
        // Phase 4: Cruising resumed
        int step = running_cycle - 20;
        x        = 2.00 + (double)step * 0.10;
        y        = 0.00;
        yaw      = 0.00;
        snprintf(buf, buf_size, "Pose updated: x=%.2fm, y=%.2fm, yaw=%.2frad", x, y, yaw);
    }
}

// Block until the user presses Enter so terminal logs remain visible.
static void wait_for_enter(void) {
    printf("\n[Process completed] Press Enter key to close...\n");
    fflush(stdout);
    (void)getchar();
}

// -----------------------------------------------------------------------------
// Main Control Loop
// -----------------------------------------------------------------------------
int main(void) {
    srand((unsigned int)time(NULL) ^ (unsigned int)CLIENT_ID);

    // Initialize and join Ritsu scheduler as client 30 (fork after sensor_node).
    RtClient client;
    if (!rtclient_init(&client, HOST, PORT, CLIENT_ID, INTERVAL_SEC, STARTUP_WAIT_SEC)) {
        printf("[%s] Failed to initialize client\n", NODE_NAME);
        wait_for_enter();
        return 1;
    }

    if (!rtclient_join(&client)) {
        printf("[%s] Failed to join server at %s:%u\n", NODE_NAME, HOST, PORT);
        rtclient_cleanup(&client);
        wait_for_enter();
        return 1;
    }
    printf("[%s] Joined server at %s:%u (CID: %u, interval: %.3fs)\n",
           NODE_NAME, HOST, PORT, CLIENT_ID, INTERVAL_SEC);

    int proc_count = 0;
    RtMessage msg;
    while (true) {
        fflush(stdout);
        rtclient_wait_next(&client, &msg);

        if (msg.mtype == RT_MSG_START) {
            const char* cycle_str = rtclient_get_extra(&msg, "cycle");
            int running_cycle     = cycle_str ? atoi(cycle_str) : proc_count;

            double target_duration = random_range(MIN_DURATION_MS, MAX_DURATION_MS);
            double start_time      = get_time_ms();
            sleep_ms(target_duration);
            double proc_ms = get_time_ms() - start_time;

            char status_buf[128];
            format_odometry_msg(running_cycle, status_buf, sizeof(status_buf));
            printf("[Cyc %02d] [%s] %s proc_ms=%.2f\n",
                   running_cycle, NODE_NAME, status_buf, proc_ms);

            rtclient_notify_done(&client, &msg);
            if (msg.mtype == RT_MSG_ERROR) {
                printf("[%s] Received ERROR on notify_done, exiting\n", NODE_NAME);
                break;
            }
            proc_count++;

        } else if (msg.mtype == RT_MSG_SKIP || msg.mtype == RT_MSG_LATE) {
            proc_count++;
            const char* msg_label = (msg.mtype == RT_MSG_SKIP) ? "SKIP" : "LATE";
            printf("[%s] Received %s, skipping execution for this cycle\n", NODE_NAME, msg_label);
            continue;

        } else if (msg.mtype == RT_MSG_ERROR) {
            const char* reason = rtclient_get_extra(&msg, "reason");
            const char* cid    = rtclient_get_extra(&msg, "cid");
            printf("[%s] Exiting on ERROR (reason=%s, cid=%s)\n",
                   NODE_NAME,
                   reason ? reason : "N/A",
                   cid ? cid : "N/A");
            break;

        } else {
            printf("[%s] Exiting due to unexpected message type: %d\n", NODE_NAME, (int)msg.mtype);
            break;
        }
    }
    fflush(stdout);

    rtclient_exit(&client);
    rtclient_cleanup(&client);

    wait_for_enter();
    return 0;
}
