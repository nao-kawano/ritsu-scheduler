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

#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <winsock2.h>
#include <ws2tcpip.h>
typedef SOCKET RT_SOCKET;
#else /* defined(_WIN32) */
#include <netinet/in.h>
#include <sys/socket.h>
typedef int RT_SOCKET;
#endif /* defined(_WIN32) */

/* -------------------------------------------------------------------------- */
/* Compile options / Source configuration options. */

/*
 * Uncomment the following line to disable all internal logging outputs to stdout.
 * Alternatively, you can define this macro via your compiler flags (e.g., -DRTCLIENT_NO_LOG).
 */
// #define RTCLIENT_NO_LOG

/* -------------------------------------------------------------------------- */

// Global protocol specifications.
#define RT_PROTOCOL_VERSION "1"
#define RT_MESSAGE_LEN_MAX  512

// Internal buffer capacity limits for the C client.
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
    // Server network settings and cached destination address.
    char server_host[256];
    uint16_t server_port;
    struct sockaddr_in server_addr;

    // Client identity and configuration parameters.
    uint16_t client_id;
    RtClientConfig config;

    // Low-level socket handle resource.
    RT_SOCKET sock;

    // Protocol session and sequence control states.
    bool connected;
    bool startup;
    uint8_t message_id;
} RtClient;

/* -------------------------------------------------------------------------- */

#ifdef __cplusplus
extern "C" {
#endif /* __cplusplus */

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
// Returns true if a matching response was received; false on error or timeout.
bool rtclient_wait_next(RtClient* client, RtMessage* out_msg);

// Sends a DONE message to the server to notify cycle completion.
RtMessageType rtclient_notify_done(RtClient* client);

// Retrieves the value of a specific extra parameter key from the message.
// Returns NULL if the specified key is not found.
const char* rtclient_get_extra(const RtMessage* msg, const char* key);

// Maps RtMessageType values to their string representations.
const char* rtclient_get_msg_str(RtMessageType mtype);

// Maps string representations to their corresponding RtMessageType values.
// Returns true on success, or false if the string is invalid.
bool rtclient_get_msg_type(const char* type_str, RtMessageType* out_mtype);

#ifdef __cplusplus
}
#endif /* __cplusplus */

#endif  // RTCLIENT_H

/* =========================================================================
   IMPLEMENTATION SECTION
   ========================================================================= */
#ifdef RTCLIENT_IMPLEMENTATION

#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#if defined(_WIN32)
#include <timeapi.h>
#include <windows.h>
#if defined(_MSC_VER)
#pragma comment(lib, "ws2_32.lib")
#pragma comment(lib, "winmm.lib")
#endif /* defined(_MSC_VER) */
typedef int socklen_t;
#define RT_SOCK_IS_INVALID(s) ((s) == INVALID_SOCKET)
#define RT_SOCKET_ERROR       SOCKET_ERROR
#define RT_INVALID_SOCKET     INVALID_SOCKET
#define rt_inet_pton          InetPtonA
#define rt_closesocket        closesocket
#else /* defined(_WIN32) */
#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>
#define RT_SOCK_IS_INVALID(s) ((s) < 0)
#define RT_SOCKET_ERROR       (-1)
#define RT_INVALID_SOCKET     (-1)
#define rt_inet_pton          inet_pton
#define rt_closesocket        close
#endif /* defined(_WIN32) */

/* -------------------------------------------------------------------------- */
/* Utility Helpers */

// Returns the current monotonic time in seconds.
static double rt_get_time_sec(void) {
#if defined(_WIN32)
    LARGE_INTEGER freq, count;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&count);
    return (double)count.QuadPart / freq.QuadPart;
#else  /* defined(_WIN32) */
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
#endif /* defined(_WIN32) */
}

/* -------------------------------------------------------------------------- */
/* Logging Helpers */

// Thread-safe core logger with millisecond precision timestamp.
static void rt_log_print(const char* fmt, ...) {
    int hours = 0, minutes = 0, seconds = 0, millis = 0;
    // Get current local time with millisecond precision.
#if defined(_WIN32)
    SYSTEMTIME st;
    GetLocalTime(&st);
    hours   = st.wHour;
    minutes = st.wMinute;
    seconds = st.wSecond;
    millis  = st.wMilliseconds;
#else  /* defined(_WIN32) */
    struct timeval tv;
    gettimeofday(&tv, NULL);
    struct tm* tm_info = localtime(&tv.tv_sec);
    if (tm_info) {
        hours   = tm_info->tm_hour;
        minutes = tm_info->tm_min;
        seconds = tm_info->tm_sec;
    }
    millis = tv.tv_usec / 1000;
#endif /* defined(_WIN32) */

    // Format timestamp prefix (HH:MM:SS.mmm).
    char line_buf[RT_MESSAGE_LEN_MAX + 128];
    int len = snprintf(line_buf, sizeof(line_buf), "%02d:%02d:%02d.%03d ", hours, minutes, seconds, millis);
    if (len < 0 || (size_t)len >= sizeof(line_buf)) {
        return;
    }

    // Append formatted message body.
    va_list args;
    va_start(args, fmt);
    int body_len = vsnprintf(line_buf + len, sizeof(line_buf) - len, fmt, args);
    va_end(args);
    if (body_len < 0) {
        return;
    }

    // Ensure trailing newline (truncate if necessary).
    size_t total_len = strlen(line_buf);
    if (total_len >= sizeof(line_buf) - 1) {
        total_len = sizeof(line_buf) - 2;
    }
    line_buf[total_len]     = '\n';
    line_buf[total_len + 1] = '\0';

    // Write to stdout atomically (thread-safe).
#if defined(_WIN32)
    _lock_file(stdout);
    fputs(line_buf, stdout);
    fflush(stdout);
    _unlock_file(stdout);
#else  /* defined(_WIN32) */
    flockfile(stdout);
    fputs(line_buf, stdout);
    fflush(stdout);
    funlockfile(stdout);
#endif /* defined(_WIN32) */
}

// Formats and logs outgoing communication messages.
static void rt_log_comm_send(const RtClient* client, RtMessageType mtype, uint32_t count, uint32_t retry_count, double timeout_sec) {
    if (!client) {
        return;
    }
    const char* type_name = rtclient_get_msg_str(mtype);
    rt_log_print(">> send %s CID:%03d MID:%d (%u/%u) t/o=%.3fs",
                 type_name, client->client_id, client->message_id, count + 1, 1 + retry_count, timeout_sec);
}

// Formats and logs incoming communication messages.
static void rt_log_comm_recv(const RtClient* client, RtMessageType mtype, uint8_t mid, const char* req_name) {
    if (!client) {
        return;
    }
    const char* type_name = rtclient_get_msg_str(mtype);
    if (req_name && req_name[0] != '\0') {
        rt_log_print("<< recv %s for %s CID:%03d MID:%d", type_name, req_name, client->client_id, mid);
    } else {
        rt_log_print("<< recv %s CID:%03d MID:%d", type_name, client->client_id, mid);
    }
}

/* Logging macros for compile-time elimination */
#ifndef RTCLIENT_NO_LOG
#define RT_LOG(...)                                                      rt_log_print(__VA_ARGS__)
#define RT_LOG_COMM_SEND(client, mtype, count, retry_count, timeout_sec) rt_log_comm_send(client, mtype, count, retry_count, timeout_sec)
#define RT_LOG_COMM_RECV(client, mtype, mid, req_name)                   rt_log_comm_recv(client, mtype, mid, req_name)
#else
#define RT_LOG(...)                                                      ((void)0)
#define RT_LOG_COMM_SEND(client, mtype, count, retry_count, timeout_sec) ((void)0)
#define RT_LOG_COMM_RECV(client, mtype, mid, req_name)                   ((void)0)
#endif

/* -------------------------------------------------------------------------- */
/* Socket Helpers */

// Safely closes the socket and resets its descriptor to invalid.
static void rt_socket_close(RtClient* client) {
    if (!client || RT_SOCK_IS_INVALID(client->sock)) {
        return;
    }

    rt_closesocket(client->sock);
    client->sock = RT_INVALID_SOCKET;
}

// Configures the receive timeout for the UDP socket.
static void rt_socket_set_timeout(RtClient* client, double timeout_sec) {
    if (!client || RT_SOCK_IS_INVALID(client->sock)) {
        return;
    }

    // Force a positive timeout (minimum 1ms) to prevent infinite blocking or crash.
    double valid_timeout = (timeout_sec > 0.0) ? timeout_sec : 0.001;
#if defined(_WIN32)
    DWORD timeout_ms = (DWORD)(valid_timeout * 1000.0);
    if (timeout_ms == 0) {
        timeout_ms = 1;
    }
    setsockopt(client->sock, SOL_SOCKET, SO_RCVTIMEO, (const char*)&timeout_ms, sizeof(timeout_ms));
#else  /* defined(_WIN32) */
    struct timeval tv;
    tv.tv_sec  = (time_t)valid_timeout;
    tv.tv_usec = (suseconds_t)((valid_timeout - (double)tv.tv_sec) * 1000000.0);
    if (tv.tv_sec == 0 && tv.tv_usec == 0) {
        tv.tv_usec = 1000;
    }
    setsockopt(client->sock, SOL_SOCKET, SO_RCVTIMEO, (const char*)&tv, sizeof(tv));
#endif /* defined(_WIN32) */
}

// Clears the socket receive buffer by discarding any pending packets.
static void rt_socket_clear_recv_buffer(RtClient* client) {
    if (!client || RT_SOCK_IS_INVALID(client->sock)) {
        return;
    }

#if defined(_WIN32) /* defined(_WIN32) */
    // Under Windows, ioctlsocket does not support retrieving the previous state,
    // so we explicitly switch it on and off with descriptive variables.
    u_long nonblocking_on = 1;
    ioctlsocket(client->sock, FIONBIO, &nonblocking_on);
    char junk[RT_MESSAGE_LEN_MAX];
    while (recv(client->sock, junk, sizeof(junk), 0) > 0) {
        // Discarding stale packet.
    }
    u_long nonblocking_off = 0;
    ioctlsocket(client->sock, FIONBIO, &nonblocking_off);
#else  /* defined(_WIN32) */
    // Under POSIX, fcntl allows saving and restoring the socket flags.
    int flags = fcntl(client->sock, F_GETFL, 0);
    fcntl(client->sock, F_SETFL, flags | O_NONBLOCK);
    char junk[RT_MESSAGE_LEN_MAX];
    while (recv(client->sock, junk, sizeof(junk), 0) > 0) {
        // Discarding stale packet.
    }
    fcntl(client->sock, F_SETFL, flags);
#endif /* defined(_WIN32) */
}

/* -------------------------------------------------------------------------- */
/* Protocol Messaging Core */

// Parses a raw packet string into an RtMessage structure.
// This function parses the string in-place in a copy buffer to avoid dynamic allocation.
static bool rt_parse_message(const char* raw_str, RtMessage* out_msg) {
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

    // Separate header and payload at ':'.
    char* colon = strchr(buf, ':');
    if (!colon) {
        return false;
    }
    *colon        = '\0';
    char* header  = buf;
    char* payload = colon + 1;

    // Parse header.
    {
        // Separate message type and message ID at '@'.
        char* at = strchr(header, '@');
        if (!at) {
            return false;
        }
        *at            = '\0';
        char* type_str = header;
        char* mid_str  = at + 1;

        // Map message type.
        if (!rtclient_get_msg_type(type_str, &out_msg->mtype)) {
            return false;
        }
        // Parse message ID (must be a single digit '0'-'9').
        if (mid_str[0] == '\0' || mid_str[1] != '\0' || mid_str[0] < '0' || mid_str[0] > '9') {
            return false;
        }
        out_msg->mid = (uint8_t)(mid_str[0] - '0');
    }
    // Parse payload.
    {
        // Parse client ID and extras in the payload.
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
        // Parse extras.
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
    }

    return true;
}

// Waits for a response packet that matches the expected ClientID and MessageID.
static bool rt_wait_for_response(RtClient* client, double timeout_sec, RtMessageType req_type, uint8_t expected_mid, RtMessage* out_msg) {
    if (!client || !out_msg || RT_SOCK_IS_INVALID(client->sock)) {
        return false;
    }

    char recv_buf[RT_MESSAGE_LEN_MAX];
    const char* req_name = rtclient_get_msg_str(req_type);
    double wait_start    = rt_get_time_sec();

    while (1) {
        double elapsed = rt_get_time_sec() - wait_start;
        if (elapsed >= timeout_sec) {
            RT_LOG("timeout, retrying... %s", req_name);
            return false;
        }
        double remaining = timeout_sec - elapsed;
        rt_socket_set_timeout(client, remaining);

        struct sockaddr_in from_addr;
        socklen_t from_len = sizeof(from_addr);
        int n              = recvfrom(client->sock, recv_buf, sizeof(recv_buf) - 1, 0, (struct sockaddr*)&from_addr, &from_len);
        if (n < 0) {
#if defined(_WIN32)
            int err = WSAGetLastError();
            if (err == WSAETIMEDOUT) {
                RT_LOG("timeout, retrying... %s", req_name);
                return false;
            }
#else  /* defined(_WIN32) */
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                RT_LOG("timeout, retrying... %s", req_name);
                return false;
            } else if (errno == EINTR) {
                continue;
            }
#endif /* defined(_WIN32) */
            continue;
        }
        recv_buf[n] = '\0';

        RtMessage msg;
        if (rt_parse_message(recv_buf, &msg)) {
            if (msg.mid == expected_mid && msg.cid == client->client_id) {
                RT_LOG_COMM_RECV(client, msg.mtype, expected_mid, req_name);
                *out_msg = msg;
                return true;
            } else {
                RT_LOG("<< mid/cid mismatch, expected MID:%d CID:%03d, actual MID:%d CID:%03d, discard and keep waiting",
                       expected_mid, client->client_id, msg.mid, msg.cid);
                continue;
            }
        } else {
            RT_LOG("parse error: %s", recv_buf);
            continue;
        }
    }
    return false;
}

// Sends a request message and blocks waiting for a matching response, with retries.
static bool rt_send_request(RtClient* client, RtMessageType req_type, double timeout_sec, uint32_t retry_count, const RtExtraParam* extras, uint32_t extras_count, RtMessage* out_msg) {
    if (!client || !out_msg || RT_SOCK_IS_INVALID(client->sock)) {
        return false;
    }

    // Clear buffer before sending.
    rt_socket_clear_recv_buffer(client);

    // Update message ID (0 ~ 9).
    client->message_id = (client->message_id + 1) % 10;

    // Build the request packet manually without snprintf overhead.
    char send_buf[RT_MESSAGE_LEN_MAX];
    size_t pos = 0;

    // Append message type string.
    const char* type_str = rtclient_get_msg_str(req_type);
    size_t type_len      = strlen(type_str);
    if (pos + type_len >= RT_MESSAGE_LEN_MAX) {
        return false;
    }
    memcpy(send_buf + pos, type_str, type_len);
    pos += type_len;

    // Append '@' and 1-digit message ID (0 ~ 9).
    if (pos + 2 >= RT_MESSAGE_LEN_MAX) {
        return false;
    }
    send_buf[pos++] = '@';
    send_buf[pos++] = (char)('0' + (client->message_id % 10));

    // Append ':' and 3-digit client ID (000 ~ 999).
    if (pos + 4 >= RT_MESSAGE_LEN_MAX) {
        return false;
    }
    send_buf[pos++] = ':';
    uint16_t cid    = client->client_id;
    send_buf[pos++] = (char)('0' + ((cid / 100) % 10));
    send_buf[pos++] = (char)('0' + ((cid / 10) % 10));
    send_buf[pos++] = (char)('0' + (cid % 10));

    // Append extra parameters if present (,KEY=VAL).
    if (extras && extras_count > 0) {
        for (uint32_t i = 0; i < extras_count; i++) {
            size_t key_len = strlen(extras[i].key);
            size_t val_len = strlen(extras[i].value);
            size_t req_len = 1 + key_len + (val_len > 0 ? (1 + val_len) : 0);
            if (pos + req_len >= RT_MESSAGE_LEN_MAX) {
                return false;
            }

            send_buf[pos++] = ',';
            memcpy(send_buf + pos, extras[i].key, key_len);
            pos += key_len;
            if (val_len > 0) {
                send_buf[pos++] = '=';
                memcpy(send_buf + pos, extras[i].value, val_len);
                pos += val_len;
            }
        }
    }

    // Append null terminator.
    send_buf[pos] = '\0';

#if defined(_WIN32)
    timeBeginPeriod(1);
#endif /* defined(_WIN32) */
    // Send packet loop with retries.
    bool success = false;
    for (uint32_t count = 0; count <= retry_count; count++) {
        RT_LOG_COMM_SEND(client, req_type, count, retry_count, timeout_sec);
        int n = sendto(client->sock, send_buf, (int)pos, 0, (struct sockaddr*)&client->server_addr, sizeof(client->server_addr));
        if (n < 0) {
            RT_LOG("sendto failed");
            break;
        }
        if (rt_wait_for_response(client, timeout_sec, req_type, client->message_id, out_msg)) {
            success = true;
            break;
        }
    }
#if defined(_WIN32)
    timeEndPeriod(1);
#endif /* defined(_WIN32) */

    return success;
}

/* -------------------------------------------------------------------------- */

// Initializes the client instance with default configurations.
bool rtclient_init(RtClient* client, const char* host, uint16_t port, uint16_t client_id, double run_cycle_sec, double startup_wait_sec) {
    double count_startup = startup_wait_sec / 5.000;

    RtClientConfig config;
    config.retry_sec_join            = 0.003;
    config.retry_count_join          = 5;
    config.retry_sec_ready_startup   = 5.000;
    config.retry_count_ready_startup = (count_startup > 0.0) ? (uint32_t)count_startup : 1;
    config.retry_sec_ready           = run_cycle_sec * 2.2;
    config.retry_count_ready         = 3;
    config.retry_sec_done            = 0.003;
    config.retry_count_done          = 5;
    config.retry_sec_exit            = 0.003;
    config.retry_count_exit          = 5;

    return rtclient_init_with_config(client, host, port, client_id, &config);
}

// Initializes the client instance with custom configurations.
bool rtclient_init_with_config(RtClient* client, const char* host, uint16_t port, uint16_t client_id, const RtClientConfig* config) {
    if (!client || !host || !config) {
        return false;
    }
    memset(client, 0, sizeof(RtClient));

    strncpy(client->server_host, host, sizeof(client->server_host) - 1);
    client->server_host[sizeof(client->server_host) - 1] = '\0';

    client->server_port = port;
    client->client_id   = client_id;
    client->config      = *config;
    client->connected   = false;
    client->startup     = true;
    client->message_id  = 0;
    client->sock        = RT_INVALID_SOCKET;

#if defined(_WIN32)
    WSADATA wsaData;
    if (WSAStartup(MAKEWORD(2, 2), &wsaData) != 0) {
        return false;
    }
#endif /* defined(_WIN32) */

    // Pre-resolve and cache the server destination address.
    memset(&client->server_addr, 0, sizeof(client->server_addr));
    client->server_addr.sin_family = AF_INET;
    client->server_addr.sin_port   = htons(client->server_port);
    if (rt_inet_pton(AF_INET, client->server_host, &client->server_addr.sin_addr) != 1) {
        client->server_addr.sin_addr.s_addr = inet_addr(client->server_host);
    }

    return true;
}

// Cleans up resources allocated for the client instance.
void rtclient_cleanup(RtClient* client) {
    if (!client) {
        return;
    }

    // Ensure it's cleared, just in case.
    client->connected = false;
    rt_socket_close(client);

#if defined(_WIN32)
    WSACleanup();
#endif /* defined(_WIN32) */
}

// Sends a JOIN message to the server and awaits response.
bool rtclient_join(RtClient* client) {
    if (!client) {
        return false;
    }
    if (client->connected) {
        RT_LOG("already joined, skip");
        return true;
    }

    // Clean up the socket first if it was already open.
    if (!RT_SOCK_IS_INVALID(client->sock)) {
        rt_socket_close(client);
    }

    // Create a UDP socket.
    RT_SOCKET sock = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    if (RT_SOCK_IS_INVALID(sock)) {
        return false;
    }
    client->sock = sock;
    // Bind to local address and port.
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family      = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_ANY);
    addr.sin_port        = htons(0);
    if (bind(client->sock, (struct sockaddr*)&addr, sizeof(addr)) == RT_SOCKET_ERROR) {
        rt_socket_close(client);
        return false;
    }

    // Sends JOIN.
    RtExtraParam extras[1];
    memset(extras, 0, sizeof(extras));
    strncpy(extras[0].key, "version", sizeof(extras[0].key) - 1);
    strncpy(extras[0].value, RT_PROTOCOL_VERSION, sizeof(extras[0].value) - 1);
    RtMessage resp;
    if (rt_send_request(client, RT_MSG_JOIN, client->config.retry_sec_join, client->config.retry_count_join, extras, 1, &resp)) {
        if (resp.mtype == RT_MSG_JOINED) {
            client->connected   = true;
            client->startup     = true;
            const char* ver_str = rtclient_get_extra(&resp, "version");
            RT_LOG("joined successfully (version=%s)", ver_str ? ver_str : "unknown");
            return true;
        } else {
            const char* reason_str = rtclient_get_extra(&resp, "reason");
            RT_LOG("failed to join: %d (reason=%s)", resp.mtype, reason_str ? reason_str : "N/A");
        }
    }

    rt_socket_close(client);
    return false;
}

// Sends an EXIT message to the server and closes the socket.
void rtclient_exit(RtClient* client) {
    if (!client) {
        return;
    }
    if (!client->connected || RT_SOCK_IS_INVALID(client->sock)) {
        RT_LOG("not connected, skip");
        return;
    }

    RtMessage resp;
    // Send EXIT request and block waiting for response.
    rt_send_request(client, RT_MSG_EXIT, client->config.retry_sec_exit, client->config.retry_count_exit, NULL, 0, &resp);
    rt_socket_close(client);
    client->connected = false;
}

// Sends a READY message and waits for the next cycle start command.
bool rtclient_wait_next(RtClient* client, RtMessage* out_msg) {
    if (!client || !out_msg) {
        return false;
    }

    // Pre-set default error message (fallback for non-connected or timeout)
    memset(out_msg, 0, sizeof(*out_msg));
    out_msg->mtype        = RT_MSG_ERROR;
    out_msg->cid          = client->client_id;
    out_msg->mid          = client->message_id;
    out_msg->extras_count = 0;

    // Connection and socket check
    if (!client->connected || RT_SOCK_IS_INVALID(client->sock)) {
        return false;
    }

    // Send READY request and await response
    double timeout_sec   = client->startup ? client->config.retry_sec_ready_startup : client->config.retry_sec_ready;
    uint32_t retry_count = client->startup ? client->config.retry_count_ready_startup : client->config.retry_count_ready;

    bool success    = rt_send_request(client, RT_MSG_READY, timeout_sec, retry_count, NULL, 0, out_msg);
    client->startup = false;

    return success;
}

// Sends a DONE message to the server to notify cycle completion.
RtMessageType rtclient_notify_done(RtClient* client) {
    if (!client) {
        return RT_MSG_ERROR;
    }
    if (!client->connected || RT_SOCK_IS_INVALID(client->sock)) {
        return RT_MSG_ERROR;
    }

    RtMessage resp;
    if (rt_send_request(client, RT_MSG_DONE, client->config.retry_sec_done, client->config.retry_count_done, NULL, 0, &resp)) {
        return resp.mtype;
    }

    return RT_MSG_ERROR;
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

// Maps RtMessageType values to their string representations.
const char* rtclient_get_msg_str(RtMessageType mtype) {
    // clang-format off
    switch (mtype) {
        case RT_MSG_JOIN:   return RT_STR_JOIN;
        case RT_MSG_READY:  return RT_STR_READY;
        case RT_MSG_DONE:   return RT_STR_DONE;
        case RT_MSG_EXIT:   return RT_STR_EXIT;
        case RT_MSG_JOINED: return RT_STR_JOINED;
        case RT_MSG_START:  return RT_STR_START;
        case RT_MSG_OK:     return RT_STR_OK;
        case RT_MSG_SKIP:   return RT_STR_SKIP;
        case RT_MSG_LATE:   return RT_STR_LATE;
        case RT_MSG_ERROR:  return RT_STR_ERROR;
        default:            return "UNKNOWN";
    }
    // clang-format on
}

// Maps string representations to their corresponding RtMessageType values.
// Returns true on success, or false if the string is invalid.
bool rtclient_get_msg_type(const char* type_str, RtMessageType* out_mtype) {
    if (!type_str || !out_mtype) {
        return false;
    }
    // clang-format off
    if (strcmp(type_str, RT_STR_JOIN) == 0)         *out_mtype = RT_MSG_JOIN;
    else if (strcmp(type_str, RT_STR_READY) == 0)   *out_mtype = RT_MSG_READY;
    else if (strcmp(type_str, RT_STR_DONE) == 0)    *out_mtype = RT_MSG_DONE;
    else if (strcmp(type_str, RT_STR_EXIT) == 0)    *out_mtype = RT_MSG_EXIT;
    else if (strcmp(type_str, RT_STR_JOINED) == 0)  *out_mtype = RT_MSG_JOINED;
    else if (strcmp(type_str, RT_STR_START) == 0)   *out_mtype = RT_MSG_START;
    else if (strcmp(type_str, RT_STR_OK) == 0)      *out_mtype = RT_MSG_OK;
    else if (strcmp(type_str, RT_STR_SKIP) == 0)    *out_mtype = RT_MSG_SKIP;
    else if (strcmp(type_str, RT_STR_LATE) == 0)    *out_mtype = RT_MSG_LATE;
    else if (strcmp(type_str, RT_STR_ERROR) == 0)   *out_mtype = RT_MSG_ERROR;
    else return false;
    // clang-format on
    return true;
}

#endif  // RTCLIENT_IMPLEMENTATION
