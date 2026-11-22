"""Live browser test of whole-building quality plans (step 15) on vite preview :4173.
python -I live_int.py  -> screenshots + texts in shots-int/
"""
import base64, json, sys, time, urllib.parse
from pathlib import Path
from playwright.sync_api import sync_playwright

OUT = Path(__file__).resolve().parent / "shots-int"
OUT.mkdir(exist_ok=True)
URL = "http://localhost:4173/"
log = []


def hash_of(s):
    j = json.dumps(s, separators=(",", ":"))
    return base64.b64encode(urllib.parse.quote(j, safe="-_.!~*'()").encode()).decode()


def settings(target, rate, engine, **kw):
    s = {"game": "factorio", "dataset": "2x1", "targets": [{"item": target, "rate": rate}], "integer": True,
         "engine": engine, "module_quality": 4, "time_limit_ms": 4000}
    s.update(kw)
    return s


def wait_done(page, limit=40):
    t0 = time.time()
    page.wait_for_timeout(600)
    while time.time() - t0 < limit:
        if page.locator(".solving").count() == 0 and page.locator(".results").count() > 0:
            return time.time() - t0
        page.wait_for_timeout(250)
    return None


def shot(page, name):
    page.screenshot(path=str(OUT / f"{name}.png"), full_page=True)
    (OUT / f"{name}.txt").write_text(page.locator("main").inner_text(), encoding="utf-8")


def card(page):
    return page.locator(".cards").inner_text().replace("\n", " | ")


def run(page, name, s, limit=40):
    page.goto(URL + "?r=" + str(time.time()) + "#" + hash_of(s))
    t = wait_done(page, limit)
    shot(page, name)
    main = page.locator("main").inner_text()
    rows = page.locator("tr.shared").count()
    log.append(f"{name}: done in {t if t is None else round(t, 1)} s | {card(page)} | shared rows {rows} | "
               f"fractional warning {'fractional machines' in main}")
    return main


def main(only_stop=False):
  with sync_playwright() as p:
    b = p.chromium.launch()
    page = b.new_page(viewport={"width": 1400, "height": 1000})
    for eng in ([] if only_stop else ["microlp", "highs", "auto"]):
        run(page, f"gear4-{eng}", settings("iron-gear-wheel@4", 10, eng))
    for eng in ([] if only_stop else ["microlp", "highs", "auto"]):
        run(page, f"qmod4-{eng}", settings("quality-module@4", 1, eng))
    if not only_stop:
        run(page, "gear4-lp", settings("iron-gear-wheel@4", 10, "auto", integer=False))
        run(page, "gear4-machine-legendary", settings("iron-gear-wheel@4", 10, "microlp", machine_quality=4))
    # Stop / Keep solving: legendary plastic with Vulcanus (LDS shuffle) and Gleba is not proven within 8 s
    lds = settings("plastic-bar@4", 60, "microlp", planets=["nauvis", "vulcanus", "gleba", "fulgora", "aquilo", "space-platform"],
                   research_productivity={"casting-low-density-structure": 1.5}, recycle_allow=["low-density-structure"],
                   time_limit_ms=8000)
    for eng in ["microlp", "highs"]:
        lds["engine"] = eng
        page.goto(URL + "?r=" + str(time.time()) + "#" + hash_of(lds))
        page.wait_for_timeout(1800)
        status = page.locator(".solving").inner_text() if page.locator(".solving").count() else "(not solving)"
        stop = page.get_by_role("button", name="Stop", exact=True)
        had_stop = stop.count() > 0
        if had_stop:
            stop.first.click()
        for _ in range(20):
            page.wait_for_timeout(300)
            if page.locator(".solving").count() == 0:
                break
        shot(page, f"lds-{eng}-stopped")
        note = page.locator(".note.bigmodel").inner_text() if page.locator(".note.bigmodel").count() else "(no note)"
        log.append(f"lds-{eng}: while solving '{status}' | Stop button {had_stop} | after stop: {note.splitlines()[0] if note else ''} | {card(page) if page.locator('.cards').count() else 'no plan'}")
        keep = page.get_by_role("button", name="Keep solving (+8 s)")
        if keep.count():
            keep.first.click()
            page.wait_for_timeout(1500)
            st = page.locator(".solving").inner_text() if page.locator(".solving").count() else "(not solving)"
            t = wait_done(page, 30)
            shot(page, f"lds-{eng}-kept")
            note = page.locator(".note.bigmodel").inner_text().splitlines()[0] if page.locator(".note.bigmodel").count() else "(no note: proven or done)"
            log.append(f"lds-{eng} keep solving: '{st}' -> done {t if t is None else round(t, 1)} s | {note} | {card(page)}")
    b.close()


if __name__ == "__main__":
    stop_only = "stop" in sys.argv
    main(stop_only)
    (OUT / ("live-log-stop.txt" if stop_only else "live-log.txt")).write_text("\n".join(log), encoding="utf-8")
    print("\n".join(log))
