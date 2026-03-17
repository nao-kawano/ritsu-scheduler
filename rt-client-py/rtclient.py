import time
import socket
import datetime
from enum import Enum

PACKET_FORMAT: str = "{}@{:d}:{:03d}"
PACKET_ENCODING: str = "utf-8"


def log(message: str) -> None:
    """
    Log a message with a timestamp.
    Args:
        message (str): The message to log.
    """
    now = datetime.datetime.now()
    timestamp = now.strftime("%H:%M:%S.%f")
    print(f"{timestamp} {message}")


class RequestType(Enum):
    """
    Enum representing the types of requests the client can send.
    """
    JOIN = "JOIN"  # Request to join the server.
    READY = "READY"  # Request to indicate the client is ready for the next task.
    DONE = "DONE"  # Request to indicate the client has completed the task.
    EXIT = "EXIT"  # Request to exit the server.


class ResponseType(Enum):
    """
    Enum representing the types of responses the server can send.
    """
    OK = "OK"  # Response indicating success.
    SKIP = "SKIP"  # Response indicating the task should be skipped.
    LATE = "LATE"  # Response indicating the process is too late.
    ERROR = "ERROR"  # Response indicating an error occurred.

    @classmethod
    def from_str(cls, string_value: str) -> "ResponseType":
        """
        Convert a string value to a ResponseType enum member.
        Args:
            string_value (str): The string value to convert.
        Returns:
            ResponseType: The corresponding ResponseType enum member.
        Raises:
            ValueError: If the string value does not match any ResponseType.
        """
        try:
            return ResponseType(string_value)
        except ValueError:
            raise ValueError(f"Invalid ResponseType: {string_value}")


class Config:
    """
    Configuration class for the RtClient.
    """
    PACKET_SIZE: int = 512  # Size of the packets used for communication.

    # Retry time for JOIN request in seconds.
    RETRY_SEC_JOIN: float = 0.003
    # Retry count for JOIN request.
    RETRY_COUNT_JOIN: int = 5

    # Retry time for READY request during startup in seconds.
    RETRY_SEC_READY_STARTUP: float = 5.000
    # Retry count for READY request during startup. Set in __init__.
    RETRY_COUNT_READY_STARTUP: int = 0

    # # Retry time for READY request in seconds. Set in __init__.
    RETRY_SEC_READY: float = 0.000
    # Retry count for READY request.
    RETRY_COUNT_READY: int = 3

    # Retry time for DONE request in seconds.
    RETRY_SEC_DONE: float = 0.003
    # Retry count for DONE request.
    RETRY_COUNT_DONE: int = 5

    # Retry time for EXIT request in seconds.
    RETRY_SEC_EXIT: float = 0.003
    # Retry count for EXIT request.
    RETRY_COUNT_EXIT: int = 5

    def __init__(self, run_cycle_sec: float, startup_wait_sec: float, retry_sec: float | None = None, retry_count: int | None = None) -> None:
        """
        Initialize the Config object.
        Args:
            run_cycle_sec (float): The run cycle time in seconds.
            startup_wait_sec (float): The startup wait time in seconds.
            retry_sec (float, optional): The retransmission interval in seconds.
            retry_count (int, optional): The number of retries.
        """
        self.RETRY_COUNT_READY_STARTUP = int(
            startup_wait_sec / self.RETRY_SEC_READY_STARTUP)
        self.RETRY_SEC_READY = run_cycle_sec * 1.1  # with mergin
        if retry_sec is not None:
            self.RETRY_SEC_JOIN = retry_sec
            self.RETRY_SEC_DONE = retry_sec
            self.RETRY_SEC_EXIT = retry_sec
        if retry_count is not None:
            self.RETRY_COUNT_JOIN = retry_count
            self.RETRY_COUNT_DONE = retry_count
            self.RETRY_COUNT_EXIT = retry_count


class RtClient:
    """
    A client for the Ritsu.
    """

    def __init__(self, host: str, port: int, client_id: int, run_cycle_sec: float, startup_wait_sec: float, retry_sec: float | None = None, retry_count: int | None = None) -> None:
        """
        Initialize the RtClient object.
        Args:
            host (str): The host address of the server.
            port (int): The port number of the server.
            client_id (int): The ID of the client.
            run_cycle_sec (float): The run cycle time in seconds.
            startup_wait_sec (float): The startup wait time in seconds.
            retry_sec (float, optional): The retransmission interval in seconds.
            retry_count (int, optional): The number of retries.
        """
        self.host: str = host
        self.port: int = port
        self.client_id: int = client_id
        self.config: Config = Config(run_cycle_sec, startup_wait_sec, retry_sec, retry_count)
        self.sock: socket.socket | None = None
        self.connected: bool = False
        self.startup: bool = True
        self.message_id: int = 0

    def join(self) -> bool:
        """
        Join to the Ritsu server.
        Returns:
            bool: True if the join was successful, False otherwise.
        """
        if self.connected:
            log("already joined, skip")
            return True
        else:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            self.sock.bind(("0.0.0.0", 0))
            resp_type = self._send_request(RequestType.JOIN,
                                           self.config.RETRY_SEC_JOIN,
                                           self.config.RETRY_COUNT_JOIN)
            if resp_type == ResponseType.OK:
                self.connected = True
                self.startup = True
                return True
            else:
                return False

    def exit(self) -> None:
        """
        Exit from the Ritsu server.
        """
        if not self.connected:
            log("not connected, skip")
        else:
            _ = self._send_request(RequestType.EXIT,
                                   self.config.RETRY_SEC_EXIT,
                                   self.config.RETRY_COUNT_EXIT)
            self.sock.close()
            self.sock = None
            self.connected = False

    def wait_next(self) -> ResponseType:
        """
        Wait for the next process timing from the server.
        Returns:
            ResponseType: The response type from the server.
        Raises:
            RuntimeError: If the client is not connected.
        """
        if not self.connected:
            raise RuntimeError("wait_next called before connected")

        timeout_sec = self.config.RETRY_SEC_READY_STARTUP if self.startup else self.config.RETRY_SEC_READY
        retry_count = self.config.RETRY_COUNT_READY_STARTUP if self.startup else self.config.RETRY_COUNT_READY

        resp_type = self._send_request(RequestType.READY, timeout_sec, retry_count)
        self.startup = False

        return resp_type

    def notify_done(self) -> ResponseType:
        """
        Notify the server that the task is done.
        Returns:
            ResponseType: The response type from the server.
        Raises:
            RuntimeError: If the client is not connected.
        """
        if not self.connected:
            raise RuntimeError("notify_done called before connected")
        resp_type = self._send_request(RequestType.DONE,
                                       self.config.RETRY_SEC_DONE,
                                       self.config.RETRY_COUNT_DONE)
        return resp_type

    def _send_request(self, req_type: RequestType, timeout_sec: float, retry_count: int) -> ResponseType:
        """
        Send a request to the server and handle the response.
        Args:
            req_type (RequestType): The type of request to send.
            timeout_sec (float): The timeout in seconds for the request.
            retry_count (int): The number of times to retry the request.
        Returns:
            ResponseType: The response type from the server.
        """
        self._clear_recv_buffer()
        ret_resp_type: ResponseType = ResponseType.ERROR
        packet: bytes = self._create_packet(req_type)
        for count in range(1 + retry_count):
            log(f">> send {req_type.value} CID:{self.client_id:03d} MID:{self.message_id} ({count+1}/{1+retry_count}) t/o={timeout_sec:.3f}s")
            self.sock.sendto(packet, (self.host, self.port))
            mtype = self._wait_for_matching_response(timeout_sec, req_type, self.message_id)
            if mtype is not None:
                ret_resp_type = mtype
                break
        return ret_resp_type

    def _wait_for_matching_response(self, timeout_sec: float, req_type: RequestType, expected_mid: int) -> ResponseType | None:
        """
        Wait for a response that matches the expected MessageID.
        Args:
            timeout_sec (float): The timeout in seconds.
            req_type (RequestType): The type of request waiting for response.
            expected_mid (int): The expected MessageID.
        Returns:
            ResponseType | None: The matching ResponseType or None if timeout.
        """
        wait_start = time.time()
        while True:
            elapsed = time.time() - wait_start
            if elapsed >= timeout_sec:
                log(f"timeout, retrying... {req_type.value}")
                return None
            remaining = timeout_sec - elapsed
            self.sock.settimeout(remaining)
            try:
                data, _ = self.sock.recvfrom(self.config.PACKET_SIZE)
                resp_type, resp_id = self._parse_packet(data)
                if resp_id == expected_mid:
                    log(f"<< recv {resp_type.value} for {req_type.value} CID:{self.client_id:03d} MID:{expected_mid}")
                    return resp_type
                log(
                    f"<< mid mismatch, expected MID:{expected_mid}, actual MID:{resp_id}, discard and keep waiting")
            except socket.timeout:
                log(f"timeout, retrying... {req_type.value}")
                return None
            except Exception:
                # If parse error or other error, keep waiting
                continue

    def _create_packet(self, request: RequestType) -> bytes:
        """
        Create a packet to send to the server.
        Args:
            request (RequestType): The type of request to create the packet for.
        Returns:
            bytes: The encoded packet.
        """
        self.message_id = (self.message_id + 1) % 10  # update message_id before send.
        packet_str: str = PACKET_FORMAT.format(request.value, self.message_id, self.client_id)
        return packet_str.encode(PACKET_ENCODING)

    def _parse_packet(self, data: bytes) -> tuple[ResponseType, int]:
        """
        Parse a packet received from the server.
        Args:
            data (bytes): The data received from the server.
        Returns:
            Tuple[ResponseType, int]: A tuple containing the response type and message ID.
        Raises:
            ValueError: If the packet format is invalid.
        """
        packet_str: str = data.decode(encoding=PACKET_ENCODING)
        parts = packet_str.split(":", maxsplit=1)
        if len(parts) != 2:
            raise ValueError(f"Invalid format, separator(:) not found: {packet_str}")
        # -- header
        headers = parts[0].split("@", maxsplit=1)
        if len(headers) != 2:
            raise ValueError(f"Invalid format, separator(@) not found: {parts[0]}")
        resp_type = ResponseType.from_str(headers[0])
        resp_id = int(headers[1])
        if resp_id < 0 or resp_id > 9:  # message id must be 1 digit.
            raise ValueError(f"Invalid message id {parts[0]}")
        # -- payload
        bodies = parts[1].split(",")
        if len(bodies[0]) != 3:  # client id must be 3-digit.
            raise ValueError(f"Invalid client id {parts[1]}")
        client_id = int(bodies[0])
        if client_id != self.client_id:
            raise ValueError(
                f"Invalid client id mismatch, expected {self.client_id}, actual {client_id}")
        # TODO: parse extras.

        return (resp_type, resp_id)

    def _clear_recv_buffer(self) -> None:
        """Clears the receive buffer by reading and discarding any data present.

        This method is used to ensure that the buffer is empty before
        performing a new operation, preventing interference from stale data.
        """
        self.sock.setblocking(False)
        while True:
            try:
                _, _ = self.sock.recvfrom(self.config.PACKET_SIZE)
            except BlockingIOError:
                break
        self.sock.setblocking(True)


if __name__ == '__main__':
    # Example usage:
    import sys
    import time
    import argparse

    parser = argparse.ArgumentParser(description="Ritsu client with argparse")
    parser.add_argument("--host", type=str, default="127.0.0.1", help="Host address")
    parser.add_argument("--port", type=int, default=7878, help="Port number")
    parser.add_argument("--client_id", type=int, default=1, help="Client ID: 0~999")
    parser.add_argument("--run_cycle_sec", type=float, default=2.0,
                        help="Execution cycle time (sec): 2.0 if cycle=2, cycle_time=1.0")
    parser.add_argument("--startup_wait_sec", type=float, default=60.0,
                        help="Startup wait time (sec)")
    parser.add_argument("--proc_time_sec", type=float, default=0.4,
                        help="Simulated process time (sec)")
    parser.add_argument("--proc_count", type=int, default=5,
                        help="max run count (times)")
    args = parser.parse_args()

    PROC_COUNT_MAX: int = args.proc_count
    proc_count: int = 0

    client: RtClient = RtClient(
        args.host,
        args.port,
        args.client_id,
        args.run_cycle_sec,
        args.startup_wait_sec
    )

    joined: bool = client.join()
    if not joined:
        log(f"failed to join, exit")
        sys.exit(-1)

    log(f"client joined")
    while True:
        try:
            resp_type: ResponseType = client.wait_next()
            log(f"start count={proc_count}")
            if resp_type == ResponseType.OK:
                log(f"got OK, do some process with {args.proc_time_sec:.3f} sec ...")
                # some process here.
                time.sleep(args.proc_time_sec)
                # client must send DONE and send READY (wait_next).
                client.notify_done()
            elif resp_type == ResponseType.SKIP:
                # client must send READY (wait_next) for next proc.
                log("got SKIP, retry")
            elif resp_type == ResponseType.LATE:
                # client must send READY (wait_next) for next proc.
                log("got LATE, retry")
            else:
                # client must send EXIT if got ERROR.
                log("got ERROR, going to exit")
                break
            # logic for a sample program.
            proc_count += 1
            if proc_count >= PROC_COUNT_MAX:
                log("completed, going to exit")
                break
        except KeyboardInterrupt:
            log("abort, going to exit")
            break
    client.exit()

    pass
