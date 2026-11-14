#!/usr/bin/env python3
"""The Pages pad must join a game over iroh from Chromium and WebKit (iOS browsers are all
WebKit). pad-host stands in for the game; the page is served from a local copy of the site.

    pad_join_test.py <pad-host binary> <site dir with pad/index.html and pad/pkg/>
"""

import asyncio
import functools
import http.server
import subprocess
import sys
import threading

from playwright.async_api import async_playwright


async def join(p, engine, url):
    browser = await getattr(p, engine).launch()
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


async def main(host_bin, site):
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=site)
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    host = subprocess.Popen([host_bin, "120"], stdout=subprocess.PIPE, text=True)
    line = host.stdout.readline().split()
    assert line[:1] == ["id"], f"pad-host: {line}"
    url = f"http://127.0.0.1:{server.server_port}/pad/#g={line[1]}&pin=1234&lan=127.0.0.1:7777"
    failed = False
    async with async_playwright() as p:
        for engine in ("chromium", "webkit"):
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
    return 1 if failed or joins < 2 else 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main(sys.argv[1], sys.argv[2])))
