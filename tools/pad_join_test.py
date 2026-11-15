#!/usr/bin/env python3
"""The Pages pad must join a game over iroh from Chromium and WebKit (iOS browsers are all
WebKit). pad-host stands in for the game; the page is served from a local copy of the site
(pad/site.sh). Then every screen of the pad is laid out on phone and iPad viewports with their
safe-area insets: nothing may leave the safe area, clip its text, or be a touch target under 56 px,
and on tablets the race controls must sit within thumb reach of a bottom corner.

    pad_join_test.py <pad-host binary> <site dir> [screenshot dir]
"""

import asyncio
import functools
import http.server
import json
import os
import pathlib
import subprocess
import sys
import threading

from playwright.async_api import async_playwright


def launch(p, engine):
    # Full Chromium: the headless shell fails web fonts that the first paint asks for
    return getattr(p, engine).launch(**({"channel": "chromium"} if engine == "chromium" else {}))


async def join(p, engine, url):
    browser = await launch(p, engine)
    page = await browser.new_page()
    errors = []
    page.on("console", lambda m: m.type == "error" and errors.append(m.text))
    await page.goto(url)
    await page.click("#go")
    try:
        await page.wait_for_function("document.getElementById('cover').style.display === 'none'", timeout=20_000)
        ok = True
    except Exception:
        ok = False
    status = await page.text_content("#status")
    await browser.close()
    return ok, status, errors


# name: viewport and safe-area insets (top, right, bottom, left), from the design frames
DEVICES = {
    "iphone-l": ((852, 393), (0, 59, 21, 59)),
    "iphone-p": ((393, 852), (59, 0, 34, 0)),
    "android-l": ((915, 412), (0, 0, 16, 40)),
    # iPads (Safari): status bar on top, home indicator below; Split View and Stage Manager sizes
    "ipad-air-p": ((820, 1180), (24, 0, 20, 0)),
    "ipad-air-l": ((1180, 820), (24, 0, 20, 0)),
    "ipad-pro-p": ((1024, 1366), (24, 0, 20, 0)),
    "ipad-pro-l": ((1366, 1024), (24, 0, 20, 0)),
    "ipad-mini-p": ((744, 1133), (24, 0, 20, 0)),
    "ipad-split-s": ((375, 1024), (24, 0, 20, 0)),
    "ipad-split-m": ((507, 1024), (24, 0, 20, 0)),
    "ipad-split-l": ((690, 1024), (24, 0, 20, 0)),
    "ipad-stage": ((900, 700), (24, 0, 20, 0)),
}
HIT = 56
# Tablet layout (the page's media query): race controls within REACH px of a bottom corner
REACH = 260
RACE_CONTROLS = "#gas, #brake, #drift, #item"
# CI runs both; PAD_ENGINES=chromium where WebKit cannot start
ENGINES = os.environ.get("PAD_ENGINES", "chromium webkit").split()
SEAT = {"t": "seat", "p": 2, "color": "#4fa3e0", "name": "Cal"}
LONG_SEAT = {"t": "seat", "p": 4, "color": "#b3203c", "name": "Wwwwwwwwwwww"}
RACERS = ["Eight Ball Eddie", "Stripe Sally", "Chalky Blue", "Rack Rita", "Hustler Hank", "Felt Felicia"]
PICK = {"t": "racers", "pick": 2, "locked": False, "list": RACERS}


def msgs(*messages):
    return "".join(f"receive({json.dumps(json.dumps(m))});" for m in messages)


# Each screen the page can show, reached through its own functions and the game's messages
STATES = [
    ("01-join", ""),
    ("02-connecting", "cover('connecting', 'Connecting through the internet...')"),
    ("03-lan", "cover('connecting', 'Connecting...')"),
    ("04-notfound", "cover('notfound', 'Game not found, scan the QR code again.')"),
    ("05-disconnected", "closed()"),
    ("06-full", "cover(null);" + msgs({"t": "full"})),
    ("07-lobby", "cover(null); setNet('online', 18)"),
    ("08-seated", "cover(null); setNet('online', 18);" + msgs(SEAT)),
    ("09-pick", "cover(null); setNet('online', 64);" + msgs(LONG_SEAT, PICK)),
    ("10-locked", "cover(null);" + msgs(SEAT, {**PICK, "locked": True})),
    ("11-race", "cover(null); setNet('online', 64);" + msgs(LONG_SEAT, PICK, {"t": "racers", "list": None})),
    (
        "12-tilt",
        "cover(null); setNet('online', 18);"
        + msgs(SEAT, {"t": "screen", "race": True})
        + "$('gyro').click(); $('gas').classList.add('on')",
    ),
    ("13-menu", "cover(null);" + msgs(SEAT, {"t": "screen", "race": True}) + "$('gyro').click(); $('menubtn').click()"),
]

# Every visible element must sit inside the safe area; buttons must be big enough and not clip
CHECK = """([t, r, b, l, hit, reach, REACH]) => {
  const W = innerWidth, H = innerHeight, bad = [];
  const tablet = matchMedia("(min-width: 500px) and (min-height: 600px)").matches;
  const name = (el) => el.id || `${el.tagName.toLowerCase()}.${el.className || ""} "${el.textContent.trim().slice(0, 20)}"`;
  for (const el of document.body.querySelectorAll("*")) {
    if (el.closest("template") || el.classList.contains("safe")) continue;
    const box = el.getBoundingClientRect(), style = getComputedStyle(el);
    if (!box.width || !box.height || style.visibility === "hidden" || el.closest("[hidden]")) continue;
    if (box.left < l - 0.5 || box.top < t - 0.5 || box.right > W - r + 0.5 || box.bottom > H - b + 0.5)
      bad.push(`${name(el)} outside the safe area: ${[box.left, box.top, box.right, box.bottom].map(Math.round)}`);
    if (el.matches("button, a.btn, input")) {
      if (box.width < hit - 0.5 || box.height < hit - 0.5) bad.push(`${name(el)} target ${Math.round(box.width)}x${Math.round(box.height)}`);
      if (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1) bad.push(`${name(el)} clips its text`);
    }
  }
  if (tablet && document.getElementById("app").dataset.view === "race")
    for (const el of document.querySelectorAll(reach)) {
      const box = el.getBoundingClientRect();
      const dy = H - (box.top + box.height / 2), dx = Math.min(box.left + box.width / 2, W - box.left - box.width / 2);
      if (Math.max(dx, dy) > REACH) bad.push(`${name(el)} is ${Math.round(Math.max(dx, dy))} px from a bottom corner`);
    }
  if (!document.fonts.check("600 20px Fredoka")) bad.push("Fredoka not loaded");
  for (const img of document.images) if (img.getBoundingClientRect().width && !img.naturalWidth) bad.push(`glyph ${img.src} missing`);
  return bad;
}"""


async def layout(p, engine, url, shots):
    browser = await launch(p, engine)
    failures = []
    for device, ((w, h), insets) in DEVICES.items():
        page = await browser.new_page(viewport={"width": w, "height": h}, has_touch=True)
        page.on("dialog", lambda d: d.accept())
        for state, script in STATES:
            await page.goto(url.replace("/pad/#", f"/pad/?s={state}#"))
            t, r, b, l = insets
            await page.add_style_tag(content=f":root{{--sa-t:{t}px;--sa-r:{r}px;--sa-b:{b}px;--sa-l:{l}px}}")
            if script:
                await page.evaluate(f"() => {{ {script} }}")
            await page.evaluate(
                "Promise.all([500, 600, 700].map((w) => document.fonts.load(`${w} 20px Fredoka`))"
                ".concat([...document.images].map((i) => i.decode().catch(() => {}))))"
            )
            if shots:
                await page.screenshot(path=str(shots / f"{state}-{device}-{engine}.png"))
            for problem in await page.evaluate(CHECK, [t, r, b, l, HIT, RACE_CONTROLS, REACH]):
                failures.append(f"{engine} {device} {state}: {problem}")
        await page.close()
    await browser.close()
    return failures


async def main(host_bin, site, shots=None):
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=site)
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    host = subprocess.Popen([host_bin, "120"], stdout=subprocess.PIPE, text=True)
    line = host.stdout.readline().split()
    assert line[:1] == ["id"], f"pad-host: {line}"
    url = f"http://127.0.0.1:{server.server_port}/pad/#g={line[1]}&pin=1234&lan=127.0.0.1:7777"
    failed = False
    async with async_playwright() as p:
        for engine in ENGINES:
            ok, status, errors = await join(p, engine, url)
            print(f"{engine}: {'joined' if ok else 'FAILED'} ({status})")
            for error in errors[:5]:
                print(f"  {error}")
            # WebKit refuses some relay hosts (trailing-dot names): a join that only works
            # through a fallback still counts as broken
            failed |= not ok or any("TLS" in error for error in errors)
        host.kill()
        joins = sum(1 for l in host.stdout if l.startswith("joined"))
        print(f"pad-host saw {joins} joins")
        failed |= joins < len(ENGINES)
        if shots:
            shots = pathlib.Path(shots)
            shots.mkdir(parents=True, exist_ok=True)
        page = f"http://127.0.0.1:{server.server_port}/pad/#pin=1234&lan=127.0.0.1:7777"
        for engine in ENGINES:
            problems = await layout(p, engine, page, shots)
            print(f"{engine}: {len(STATES)} screens x {len(DEVICES)} viewports, {len(problems)} layout problems")
            for problem in problems:
                print(f"  {problem}")
            failed |= bool(problems)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main(*sys.argv[1:4])))
