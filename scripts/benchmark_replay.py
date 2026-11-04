#!/usr/bin/env python3
"""Compare release binaries against the same local WebSocket trade replay."""

import argparse
import base64
import hashlib
import json
import os
import resource
import socket
import statistics
import struct
import subprocess
import threading
import time


def frame(payload):
    size = len(payload)
    header = bytes([0x81, size]) if size < 126 else b"\x81\x7e" + struct.pack("!H", size)
    return header + payload


def replay(binary, frames, trades):
    failures = []
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        listener.listen(1)
        listener.settimeout(30)

        def serve():
            try:
                with listener.accept()[0] as stream:
                    stream.settimeout(30)
                    request = b""
                    while b"\r\n\r\n" not in request:
                        data = stream.recv(4096)
                        if not data:
                            raise RuntimeError("client closed before handshake")
                        request += data
                    key = next(line.split(b":", 1)[1].strip() for line in request.split(b"\r\n")
                               if line.lower().startswith(b"sec-websocket-key:"))
                    accept = base64.b64encode(hashlib.sha1(key + b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11").digest())
                    stream.sendall(b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n"
                                   b"Connection: Upgrade\r\nSec-WebSocket-Accept: " + accept + b"\r\n\r\n")
                    if not stream.recv(4096):
                        raise RuntimeError("client closed before subscribing")
                    for batch in frames:
                        stream.sendall(batch)
                    while stream.recv(4096):
                        pass
            except ConnectionResetError:
                pass
            except Exception as error:
                failures.append(error)

        server = threading.Thread(target=serve, daemon=True)
        server.start()
        before = resource.getrusage(resource.RUSAGE_CHILDREN)
        started = time.perf_counter()
        result = subprocess.run([
            binary, "--url", f"ws://127.0.0.1:{listener.getsockname()[1]}/ws",
            "--format", "json", "--quiet", "--max-trades", str(trades),
            "--timeout", "10", "--max-reconnects", "1",
        ], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, timeout=30, check=True,
            env={**os.environ, "RUST_LOG": "off"})
        elapsed = time.perf_counter() - started
        after = resource.getrusage(resource.RUSAGE_CHILDREN)
        server.join(timeout=5)
        if server.is_alive() or failures:
            raise RuntimeError(f"replay server failed: {failures}; {result.stderr!r}")
        return elapsed, after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binaries", nargs="+")
    parser.add_argument("--trades", type=int, default=50000)
    parser.add_argument("--runs", type=int, default=5)
    args = parser.parse_args()
    if args.trades <= 0 or args.runs <= 0:
        parser.error("--trades and --runs must be positive")
    frames = []
    for start in range(0, args.trades, 64):
        data = [{"coin": "BTC", "side": "B", "px": "100000.5", "sz": "0.25",
                 "time": 1760000000000, "hash": "0x1234", "tid": tid, "users": ["buyer", "seller"]}
                for tid in range(start + 1, min(start + 64, args.trades) + 1)]
        frames.append(frame(json.dumps({"channel": "trades", "data": data}, separators=(",", ":")).encode()))
    samples = {binary: [] for binary in args.binaries}
    for run in range(args.runs):
        for binary in args.binaries if run % 2 == 0 else reversed(args.binaries):
            samples[binary].append(replay(binary, frames, args.trades))
    for binary, values in samples.items():
        wall, cpu = (statistics.median(v[index] for v in values) for index in (0, 1))
        print(json.dumps({"binary": binary, "trades": args.trades, "runs": args.runs,
                          "median_seconds": wall, "median_cpu_seconds": cpu,
                          "trades_per_second": args.trades / wall}))


if __name__ == "__main__":
    main()
