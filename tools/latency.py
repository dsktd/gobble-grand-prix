#!/usr/bin/env python3
"""Measure phone-to-kart latency like a phone would: send 60 Hz inputs, time the acks.

    latency.py <host> <pin> [seconds]

The game acks an input once the kart input system has used it, so the round trip covers
network both ways plus the wait for the next frame.
"""

import asyncio
import json
import ssl
import statistics
import sys
import time

import websockets


async def main(host, pin, seconds):
    insecure = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    insecure.check_hostname = False
    insecure.verify_mode = ssl.CERT_NONE

    try:
        async with websockets.connect(f"wss://{host}:7777/ws?pin=wrong", ssl=insecure):
            print("wrong PIN: accepted (BAD)")
    except Exception as err:
        print(f"wrong PIN: refused ({type(err).__name__})")

    async with websockets.connect(f"wss://{host}:7777/ws?pin={pin}", ssl=insecure) as ws:
        hello = json.loads(await ws.recv())
        me = hello["id"]
        rtts = []

        async def reader():
            async for raw in ws:
                msg = json.loads(raw)
                if msg.get("t") == "ack" and msg.get("id") == me:
                    rtts.append(time.perf_counter() * 1000 - msg["ms"])

        task = asyncio.create_task(reader())
        end = time.perf_counter() + seconds
        seq = 0
        while time.perf_counter() < end:
            seq += 1
            await ws.send(json.dumps({"t": "in", "s": seq, "ms": time.perf_counter() * 1000,
                                      "steer": 0.0, "gas": True, "brake": False, "drift": False, "item": False}))
            await asyncio.sleep(1 / 60)
        task.cancel()
        rtts.sort()
        print(f"sent {seq}, acks {len(rtts)}")
        if rtts:
            print(f"round trip ms: median {statistics.median(rtts):.1f}, "
                  f"p95 {rtts[int(len(rtts) * 0.95)]:.1f}, max {rtts[-1]:.1f}")


asyncio.run(main(sys.argv[1], sys.argv[2], float(sys.argv[3]) if len(sys.argv) > 3 else 10))
