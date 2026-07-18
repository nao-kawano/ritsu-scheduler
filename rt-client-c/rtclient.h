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

/* -------------------------------------------------------------------------- */
/* Compile options / Source configuration options. */

/*
 * Uncomment the following line to disable all internal logging outputs to stdout.
 * Alternatively, you can define this macro via your compiler flags (e.g., -DRTCLIENT_NO_LOG).
 */
// #define RTCLIENT_NO_LOG

/* -------------------------------------------------------------------------- */

#define RT_MESSAGE_LEN_MAX    512
#define RT_MESSAGE_EXTRAS_MAX 16
#define RT_KEY_LEN_MAX        64
#define RT_VAL_LEN_MAX        128

// Protocol message type string representations.
#define RT_STR_JOIN   "JOIN"
#define RT_STR_READY  "READY"
#define RT_STR_DONE   "DONE"
#define RT_STR_EXIT   "EXIT"
#define RT_STR_JOINED "JOINED"
#define RT_STR_START  "START"
#define RT_STR_OK     "OK"
#define RT_STR_SKIP   "SKIP"
#define RT_STR_LATE   "LATE"
#define RT_STR_ERROR  "ERROR"

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

/* -------------------------------------------------------------------------- */

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

#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#else
#include <sys/time.h>
#include <time.h>
#endif

// Outputs millisecond-precision timestamped log messages.
// This function can be compiled out by defining RTCLIENT_NO_LOG.
static void log_io(bool is_send, RtMessageType mtype, uint16_t cid, uint8_t mid, const char* details) {
#ifdef RTCLIENT_NO_LOG
    (void)is_send;
    (void)mtype;
    (void)cid;
    (void)mid;
    (void)details;
#else
    int hours = 0, minutes = 0, seconds = 0, millis = 0;
#if defined(_WIN32)
    SYSTEMTIME st;
    GetLocalTime(&st);
    hours   = st.wHour;
    minutes = st.wMinute;
    seconds = st.wSecond;
    millis  = st.wMilliseconds;
#else
    struct timeval tv;
    gettimeofday(&tv, NULL);
    struct tm* tm_info = localtime(&tv.tv_sec);
    if (tm_info) {
        hours   = tm_info->tm_hour;
        minutes = tm_info->tm_min;
        seconds = tm_info->tm_sec;
    }
    millis = tv.tv_usec / 1000;
#endif

    const char* direction = is_send ? ">> send" : "<< recv";
    const char* type_name = "UNKNOWN";
    // clang-format off
    switch (mtype) {
        case RT_MSG_JOIN:   type_name = RT_STR_JOIN; break;
        case RT_MSG_READY:  type_name = RT_STR_READY; break;
        case RT_MSG_DONE:   type_name = RT_STR_DONE; break;
        case RT_MSG_EXIT:   type_name = RT_STR_EXIT; break;
        case RT_MSG_JOINED: type_name = RT_STR_JOINED; break;
        case RT_MSG_START:  type_name = RT_STR_START; break;
        case RT_MSG_OK:     type_name = RT_STR_OK; break;
        case RT_MSG_SKIP:   type_name = RT_STR_SKIP; break;
        case RT_MSG_LATE:   type_name = RT_STR_LATE; break;
        case RT_MSG_ERROR:  type_name = RT_STR_ERROR; break;
    }
    // clang-format on

    if (details && details[0] != '\0') {
        printf("%02d:%02d:%02d.%03d %s %s CID:%03d MID:%d %s\n",
               hours, minutes, seconds, millis, direction, type_name, cid, mid, details);
    } else {
        printf("%02d:%02d:%02d.%03d %s %s CID:%03d MID:%d\n",
               hours, minutes, seconds, millis, direction, type_name, cid, mid);
    }
    fflush(stdout);
#endif
}

// Parses a raw packet string into an RtMessage structure.
// This function parses the string in-place in a copy buffer to avoid dynamic allocation.
static bool parse_message(const char* raw_str, RtMessage* out_msg) {
    if (!raw_str || !out_msg) {
        return false;
    }
    // Clear the output structure to prevent residual data ("ghosts of the past").
    memset(out_msg, 0, sizeof(RtMessage));

    // Copy to a local buffer for in-place tokenization.
    char buf[RT_MESSAGE_LEN_MAX];
    size_t len = strlen(raw_str);
    if (len >= RT_MESSAGE_LEN_MAX) {
        return false;
    }
    memcpy(buf, raw_str, len + 1);

    // 1. Separate header and payload at ':'.
    char* colon = strchr(buf, ':');
    if (!colon) {
        return false;
    }
    *colon        = '\0';
    char* header  = buf;
    char* payload = colon + 1;

    // 2. Separate message type and message ID at '@'.
    char* at = strchr(header, '@');
    if (!at) {
        return false;
    }
    *at            = '\0';
    char* type_str = header;
    char* mid_str  = at + 1;

    // Map message type.
    // clang-format off
    if (strcmp(type_str, RT_STR_JOIN) == 0)         out_msg->mtype = RT_MSG_JOIN;
    else if (strcmp(type_str, RT_STR_READY) == 0)   out_msg->mtype = RT_MSG_READY;
    else if (strcmp(type_str, RT_STR_DONE) == 0)    out_msg->mtype = RT_MSG_DONE;
    else if (strcmp(type_str, RT_STR_EXIT) == 0)    out_msg->mtype = RT_MSG_EXIT;
    else if (strcmp(type_str, RT_STR_JOINED) == 0)  out_msg->mtype = RT_MSG_JOINED;
    else if (strcmp(type_str, RT_STR_START) == 0)   out_msg->mtype = RT_MSG_START;
    else if (strcmp(type_str, RT_STR_OK) == 0)      out_msg->mtype = RT_MSG_OK;
    else if (strcmp(type_str, RT_STR_SKIP) == 0)    out_msg->mtype = RT_MSG_SKIP;
    else if (strcmp(type_str, RT_STR_LATE) == 0)    out_msg->mtype = RT_MSG_LATE;
    else if (strcmp(type_str, RT_STR_ERROR) == 0)   out_msg->mtype = RT_MSG_ERROR;
    else return false;
    // clang-format on

    // Parse message ID (must be a single digit '0'-'9').
    if (mid_str[0] == '\0' || mid_str[1] != '\0' || mid_str[0] < '0' || mid_str[0] > '9') {
        return false;
    }
    out_msg->mid = (uint8_t)(mid_str[0] - '0');

    // 3. Parse client ID and extras in the payload.
    // The payload is comma-separated: "CID,KEY=VAL,KEY=VAL..."
    char* next_comma = strchr(payload, ',');
    if (next_comma) {
        *next_comma = '\0';
    }

    // Parse client ID (high speed manual parsing instead of sscanf).
    uint32_t cid_val = 0;
    char* cid_p      = payload;
    if (*cid_p == '\0') {
        return false;
    }
    while (*cid_p != '\0') {
        if (*cid_p < '0' || *cid_p > '9') {
            return false;
        }
        cid_val = cid_val * 10 + (*cid_p - '0');
        if (cid_val > 999) {
            return false;
        }
        cid_p++;
    }
    out_msg->cid = (uint16_t)cid_val;

    // Initialize extras count.
    out_msg->extras_count = 0;

    // 4. Parse extras.
    char* p = next_comma ? next_comma + 1 : NULL;
    while (p && *p != '\0' && out_msg->extras_count < RT_MESSAGE_EXTRAS_MAX) {
        char* comma = strchr(p, ',');
        if (comma) {
            *comma = '\0';
        }

        // Split "KEY=VALUE" at '='.
        char* eq            = strchr(p, '=');
        RtExtraParam* param = &out_msg->extras[out_msg->extras_count];
        if (eq) {
            *eq       = '\0';
            char* key = p;
            char* val = eq + 1;

            // Bounds check for key and value lengths.
            size_t key_len = strlen(key);
            size_t val_len = strlen(val);
            if (key_len >= RT_KEY_LEN_MAX || val_len >= RT_VAL_LEN_MAX) {
                return false;
            }
            memcpy(param->key, key, key_len + 1);
            memcpy(param->value, val, val_len + 1);
        } else {
            // Key only, empty value.
            size_t key_len = strlen(p);
            if (key_len >= RT_KEY_LEN_MAX) {
                return false;
            }
            memcpy(param->key, p, key_len + 1);
            param->value[0] = '\0';
        }

        out_msg->extras_count++;
        p = comma ? comma + 1 : NULL;
    }

    return true;
}

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

// Retrieves the value of a specific extra parameter key from the message.
const char* rtclient_get_extra(const RtMessage* msg, const char* key) {
    if (!msg || !key) {
        return NULL;
    }
    for (uint32_t i = 0; i < msg->extras_count; i++) {
        if (strcmp(msg->extras[i].key, key) == 0) {
            return msg->extras[i].value;
        }
    }
    return NULL;
}

#endif  // RTCLIENT_IMPLEMENTATION
