import socket
import datetime
from enum import Enum


def log(message: str):
    now = datetime.datetime.now()
    timestamp = now.strftime("%H:%M:%S.%f")
    print("{} {}".format(timestamp, message))


class RequestType(Enum):
    JOIN = "JOIN"
    READY = "READY"
    DONE = "DONE"
    EXIT = "EXIT"


class ResponseType(Enum):
    OK = "OK"
    SKIP = "SKIP"
    ERROR = "ERROR"

    @classmethod
    def from_str(cls, string_value: str) -> "ResponseType":
        for e in ResponseType:
            if e.value == string_value:
                return e
        raise ValueError(
            "failed to convert str to ResponseType: {}".format(string_value))


class Config:
    PACKET_SIZE: int = 512  # bytes.

    RETRY_TIME_SEC_JOIN: float = 0.020  # sec.
    RETRY_COUNT_JOIN: int = 3  # times.

    RETRY_TIME_SEC_READY_STARTUP: float = 5.000
    RETRY_COUNT_READY_STARTUP: int = 0  # times. set by __init__()

    RETRY_TIME_SEC_READY: float = 0.000  # sec. set by __init__()
    RETRY_COUNT_READY: int = 3  # times.

    RETRY_TIME_SEC_DONE: float = 0.050  # sec.
    RETRY_COUNT_DONE: int = 3  # times.

    RETRY_TIME_SEC_EXIT: float = 0.020  # sec.
    RETRY_COUNT_EXIT: int = 3  # times.

    def __init__(self, run_cycle_time_ms: int, startup_wait_ms: int):
        self.RETRY_COUNT_READY_STARTUP = int(
            startup_wait_ms / (1000 * self.RETRY_TIME_SEC_READY_STARTUP))
        self.RETRY_TIME_SEC_READY = float(run_cycle_time_ms / 1000)


class DPSClient:
    def __init__(self, host: str, port: int, client_id: int, run_cycle_time_ms: int, startup_wait_ms: int):
        self.host = host
        self.port = port
        self.client_id = client_id
        self.config: Config = Config(run_cycle_time_ms, startup_wait_ms)
        self.sock: socket.socket = None
        self.connected: bool = False
        self.startup: bool = True
        self.message_id: int = 0

    def join(self) -> bool:
        if self.connected:
            log("already joined, skip")
            return True
        else:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            resp_type = self._send_request(RequestType.JOIN,
                                           self.config.RETRY_TIME_SEC_JOIN,
                                           self.config.RETRY_COUNT_JOIN)
            if resp_type == ResponseType.OK:
                self.connected = True
                self.startup = True
                return True
            else:
                return False

    def exit(self) -> None:
        if not self.connected:
            log("not connected, skip")
            return True
        else:
            _ = self._send_request(RequestType.EXIT,
                                   self.config.RETRY_TIME_SEC_EXIT,
                                   self.config.RETRY_COUNT_EXIT)
            self.sock.close()
            self.sock = None
            self.connected = False
            return True

    def wait_next(self) -> ResponseType:
        if not self.connected:
            raise RuntimeError("wait_next called before connected")
        if self.startup:
            self.startup = False
            resp_type = self._send_request(RequestType.READY,
                                           self.config.RETRY_TIME_SEC_READY_STARTUP,
                                           self.config.RETRY_COUNT_READY_STARTUP)
        else:
            resp_type = self._send_request(RequestType.READY,
                                           self.config.RETRY_TIME_SEC_READY,
                                           self.config.RETRY_COUNT_READY)
        return resp_type

    def notify_done(self) -> ResponseType:
        if not self.connected:
            raise RuntimeError("notify_done called before connected")
        resp_type = self._send_request(RequestType.DONE,
                                       self.config.RETRY_TIME_SEC_DONE,
                                       self.config.RETRY_COUNT_DONE)
        return resp_type

    def _send_request(self, req_type: RequestType, timeout_sec: float, retry_count: int) -> ResponseType:
        ret_resp_type: ResponseType = ResponseType.ERROR
        packet: bytes = self._create_packet(req_type)
        self.sock.settimeout(timeout_sec)
        for count in range(retry_count+1):
            log("sending {}@{} to server ({}/{}) with t/o {} sec".format(
                req_type.value, self.message_id,
                count+1, 1+retry_count, timeout_sec))
            self.sock.sendto(packet, (self.host, self.port))
            try:
                data, _ = self.sock.recvfrom(self.config.PACKET_SIZE)
                (resp_type, resp_id) = self._parse_packet(data)
                # check sequence.
                if resp_id != self.message_id:
                    log("{} message id missmatch, expected {}, actual {}, continue".format(
                        req_type.value, self.message_id, resp_id))
                    continue
                # check response.
                if resp_type == ResponseType.OK:
                    log("got OK for {}".format(req_type.value))
                else:
                    log("got {} for {}".format(resp_type.value, req_type.value))
                ret_resp_type = resp_type
                break
            except socket.timeout:
                log("{} timeout, retrying...".format(req_type.value))
            except Exception as e:
                log("Error in receive for {}: {}".format(req_type.value, e))
                break
        return ret_resp_type

    def _create_packet(self, request: RequestType) -> bytes:
        # update message_id before send.
        self.message_id += 1
        if self.message_id > 9:
            self.message_id = 0
        # create packet.
        packet_str: str = "{}@{:d}:{:03d}".format(request.value, self.message_id, self.client_id)
        return packet_str.encode("utf-8")

    def _parse_packet(self, data: bytes) -> tuple[ResponseType, int]:
        packet_str: str = data.decode(encoding='utf-8')
        parts = packet_str.split(":", maxsplit=1)
        if len(parts) != 2:
            raise ValueError("Invalid format, separater(:) not found: {}".format(packet_str))
        # -- header
        headers = parts[0].split("@", maxsplit=1)
        if len(headers) != 2:
            raise ValueError("Invalid format, separater(@) not found: {}".format(parts[0]))
        resp_type = ResponseType.from_str(headers[0])
        resp_id = int(headers[1])
        if resp_id < 0 or resp_id > 9:
            raise ValueError("Invalid message id {}".format(parts[0]))
        # -- payload
        bodies = parts[1].split(",")
        if len(bodies[0]) != 3:
            raise ValueError("Invalid client id {}".format(parts[1]))
        client_id = int(bodies[0])
        if client_id != self.client_id:
            raise ValueError("Invalid client id missmatch, expected {}, actual {}".format(
                self.client_id, client_id))

        return (resp_type, resp_id)


if __name__ == '__main__':
    # Example usage:
    import time
    import argparse

    parser = argparse.ArgumentParser(description="DPSClient with argparse")
    parser.add_argument("--host", type=str, default="127.0.0.1", help="Host address")
    parser.add_argument("--port", type=int, default=7878, help="Port number")
    parser.add_argument("--client_id", type=int, default=1, help="Client ID: 0~999")
    parser.add_argument("--run_cycle_time", type=int, default=2000,
                        help="Execution cycle time (ms): 200 if cycle=2, cycle_time=100")
    parser.add_argument("--startup_wait_time", type=int, default=60 * 1000,
                        help="Startup wait time (ms)")
    parser.add_argument("--proc_time", type=int, default=400,
                        help="Simulated process time (ms)")
    parser.add_argument("--proc_count", type=int, default=5,
                        help="max run count (times)")
    args = parser.parse_args()

    PROC_COUNT_MAX = args.proc_count
    proc_count = 0

    client: DPSClient = DPSClient(
        args.host,
        args.port,
        args.client_id,
        args.run_cycle_time,
        args.startup_wait_time
    )

    joined = client.join()
    log("## client joined={}".format(joined))
    while joined:
        try:
            resp_type = client.wait_next()
            log("## start count={}".format(proc_count))
            if resp_type == ResponseType.OK:
                time.sleep(float(args.proc_time / 1000))
                client.notify_done()
            elif resp_type == ResponseType.SKIP:
                log("got SKIP, retry")
            else:
                log("got ERROR, going to exit")
                joined = False
            # for Example
            proc_count += 1
            if proc_count >= PROC_COUNT_MAX:
                log("## goint to exit")
                joined = False
            log("")
        except KeyboardInterrupt:
            break
    client.exit()

    pass
