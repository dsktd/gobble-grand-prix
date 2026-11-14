#!/usr/bin/env python3
"""Headless phone controller test, no virtual input devices: two WebSocket clients act as two
phones. Both join the lobby, must get distinct seats, then each drives only its own kart.

    phone_join_test.py <host> <pin>      (the game must be on the title menu)
"""

import asyncio
import json
import secrets
import ssl
import sys
import time

import websockets


class Phone:
    def __init__(self, ws, me):
        self.ws, self.me = ws, me
        self.seat = None
        self.acks = 0
        self.full = False

    async def listen(self):
        async for raw in self.ws:
            msg = json.loads(raw)
            if msg.get("id") != self.me:
                continue
            if msg["t"] == "seat":
                self.seat = msg.get("p")
            elif msg["t"] == "ack":
                self.acks += 1
            elif msg["t"] == "full":
                self.full = True

    async def press(self, key):
        await self.ws.send(json.dumps({"t": "menu", "k": key}))
        await asyncio.sleep(0.4)

    async def drive(self, seconds):
        end, seq = time.monotonic() + seconds, 0
        while time.monotonic() < end:
            seq += 1
            await self.ws.send(json.dumps({"t": "in", "s": seq, "ms": time.monotonic() * 1000,
                                           "steer": 0.3, "gas": True, "brake": False,
                                           "drift": False, "item": False}))
            await asyncio.sleep(1 / 60)


async def connect(host, pin, insecure):
    ws = await websockets.connect(f"wss://{host}:7777/ws?pin={pin}&token={secrets.token_hex(12)}", ssl=insecure)
    phone = Phone(ws, json.loads(await ws.recv())["id"])
    asyncio.create_task(phone.listen())
    return phone


async def main(host, pin):
    insecure = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    insecure.check_hostname = False
    insecure.verify_mode = ssl.CERT_NONE
    a = await connect(host, pin, insecure)
    b = await connect(host, pin, insecure)
    failures = []

    await a.press("ok")  # title menu: Start Race -> lobby
    await a.press("ok")  # join
    await b.press("ok")  # join
    print(f"seats: phone {a.me} -> P{a.seat}, phone {b.me} -> P{b.seat}")
    if not (a.seat and b.seat and a.seat != b.seat):
        failures.append("two phones did not get two distinct seats")

    await a.press("ok")  # start: racer pick
    await a.press("ok")  # ready
    await b.press("ok")  # ready
    await a.press("ok")  # track pick: race
    await asyncio.sleep(4.5)  # countdown

    a.acks = b.acks = 0
    await b.drive(2.0)
    print(f"only B drives: acks A={a.acks} B={b.acks}")
    if a.acks or not b.acks:
        failures.append("B's input reached A's kart, or did not reach B's")
    a.acks = b.acks = 0
    await a.drive(2.0)
    print(f"only A drives: acks A={a.acks} B={b.acks}")
    if b.acks or not a.acks:
        failures.append("A's input reached B's kart, or did not reach A's")

    print("PASS" if not failures else "FAIL: " + "; ".join(failures))
    return 1 if failures else 0


sys.exit(asyncio.run(main(sys.argv[1], sys.argv[2])))
