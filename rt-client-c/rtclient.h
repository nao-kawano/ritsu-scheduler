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
#ifndef RTCLIENT_H
#define RTCLIENT_H

#include <stdbool.h>
#include <stdint.h>

#define RT_MESSAGE_LEN_MAX    512
#define RT_MESSAGE_EXTRAS_MAX 16
#define RT_KEY_LEN_MAX        64
#define RT_VAL_LEN_MAX        128

// Message types compatible with Rust and Python clients.
typedef enum {
    RT_MSG_JOIN,
    RT_MSG_READY,
    RT_MSG_DONE,
    RT_MSG_EXIT,
    RT_MSG_JOINED,
    RT_MSG_START,
    RT_MSG_OK,
    RT_MSG_SKIP,
    RT_MSG_LATE,
    RT_MSG_ERROR
} RtMessageType;

// Key-Value pair for extra parameters.
typedef struct {
    char key[RT_KEY_LEN_MAX];
    char value[RT_VAL_LEN_MAX];
} RtExtraParam;

// Structure representing a message.
typedef struct {
    RtMessageType mtype;
    uint8_t mid;   // Message ID (0 ~ 9)
    uint16_t cid;  // Client ID (0 ~ 999)
    RtExtraParam extras[RT_MESSAGE_EXTRAS_MAX];
    uint32_t extras_count;
} RtMessage;

// Configuration options for the client.
typedef struct {
    double retry_sec_join;
    uint32_t retry_count_join;
    double retry_sec_ready_startup;
    uint32_t retry_count_ready_startup;
    double retry_sec_ready;
    uint32_t retry_count_ready;
    double retry_sec_done;
    uint32_t retry_count_done;
    double retry_sec_exit;
    uint32_t retry_count_exit;
} RtClientConfig;

// Structure representing the client instance state and settings.
typedef struct {
    char server_host[256];
    uint16_t server_port;
    uint16_t client_id;
    RtClientConfig config;

#if defined(_WIN32)
    uintptr_t sock;  // Under Windows, this maps to SOCKET (UINT_PTR).
#else
    int sock;  // Under POSIX, this maps to a file descriptor.
#endif

    bool connected;
    bool startup;
    uint8_t message_id;
} RtClient;

#ifdef __cplusplus
extern "C" {
#endif

// Initializes the client instance with default configurations.
bool rtclient_init(RtClient* client, const char* host, uint16_t port, uint16_t client_id, double run_cycle_sec, double startup_wait_sec);

// Initializes the client instance with custom configurations.
bool rtclient_init_with_config(RtClient* client, const char* host, uint16_t port, uint16_t client_id, const RtClientConfig* config);

// Cleans up resources allocated for the client instance.
void rtclient_cleanup(RtClient* client);

// Sends a JOIN message to the server and awaits response.
bool rtclient_join(RtClient* client);

// Sends an EXIT message to the server.
void rtclient_exit(RtClient* client);

// Sends a READY message and waits for the next cycle start command.
RtMessage rtclient_wait_next(RtClient* client);

// Sends a DONE message to the server to notify cycle completion.
RtMessageType rtclient_notify_done(RtClient* client);

// Retrieves the value of a specific extra parameter key from the message.
// Returns NULL if the specified key is not found.
const char* rtclient_get_extra(const RtMessage* msg, const char* key);

#ifdef __cplusplus
}
#endif

#endif  // RTCLIENT_H

/* =========================================================================
   IMPLEMENTATION SECTION
   ========================================================================= */
#ifdef RTCLIENT_IMPLEMENTATION

#include <stdio.h>
#include <string.h>

// Stub implementation of rtclient_init.
bool rtclient_init(RtClient* client, const char* host, uint16_t port, uint16_t client_id, double run_cycle_sec, double startup_wait_sec) {
    (void)client;
    (void)host;
    (void)port;
    (void)client_id;
    (void)run_cycle_sec;
    (void)startup_wait_sec;
    return true;
}

// Stub implementation of rtclient_init_with_config.
bool rtclient_init_with_config(RtClient* client, const char* host, uint16_t port, uint16_t client_id, const RtClientConfig* config) {
    (void)client;
    (void)host;
    (void)port;
    (void)client_id;
    (void)config;
    return true;
}

// Stub implementation of rtclient_cleanup.
void rtclient_cleanup(RtClient* client) {
    (void)client;
}

// Stub implementation of rtclient_join.
bool rtclient_join(RtClient* client) {
    (void)client;
    return true;
}

// Stub implementation of rtclient_exit.
void rtclient_exit(RtClient* client) {
    (void)client;
}

// Stub implementation of rtclient_wait_next. Returns RT_MSG_START by default.
RtMessage rtclient_wait_next(RtClient* client) {
    RtMessage msg;
    memset(&msg, 0, sizeof(msg));
    msg.mtype = RT_MSG_START;
    msg.mid   = 0;
    msg.cid   = client ? client->client_id : 0;
    return msg;
}

// Stub implementation of rtclient_notify_done.
RtMessageType rtclient_notify_done(RtClient* client) {
    (void)client;
    return RT_MSG_OK;
}

// Stub implementation of rtclient_get_extra.
const char* rtclient_get_extra(const RtMessage* msg, const char* key) {
    (void)msg;
    (void)key;
    return NULL;
}

#endif  // RTCLIENT_IMPLEMENTATION
