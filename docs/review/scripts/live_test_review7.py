"""Review 7 live test: whole-building quality plans on vite preview :4173 (Playwright Chromium).
python -I live_test_review7.py  -> docs/review/shots-review7/ (png + txt + live-log.txt)
"""
import base64, json, re, time, urllib.parse
from pathlib import Path
from playwright.sync_api import sync_playwright

OUT = Path(__file__).resolve().parent.parent / "shots-review7"
OUT.mkdir(exist_ok=True)
URL = "http://localhost:4173/"
log = []
errors = []


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


def shot(page, name, full=True):
    page.screenshot(path=str(OUT / f"{name}.png"), full_page=full)
    (OUT / f"{name}.txt").write_text(page.locator("main").inner_text(), encoding="utf-8")


def card(page):
    return page.locator(".cards").inner_text().replace("\n", " | ")


def open_plan(page, s, limit=40):
    page.goto(URL + "?r=" + str(time.time()) + "#" + hash_of(s))
    return wait_done(page, limit)


def num(x):
    return float(x.replace(",", "").replace(" ", "").replace(" ", ""))


def check_rows(page, name):
    """Every shared row: whole machines are whole, the tier split fits them, busy % = split / whole."""
    bad = []
    rows = page.locator("tr.shared")
    for i in range(rows.count()):
        tds = rows.nth(i).locator("td")
        cell = [x for x in tds.nth(2).inner_text().split("\n") if x.strip()]
        whole = num(cell[0])
        split = [num(x.rsplit(" ", 1)[1]) for x in cell[1:]]
        busy = float(re.match(r"([\d.]+)%", tds.nth(3).inner_text()).group(1))
        work = sum(split)
        tol = 0.006 * len(split)  # the split is shown to 2 decimals
        if whole != round(whole) or work > whole + tol or abs(busy - 100 * work / whole) > 100 * tol / whole + 0.06:
            bad.append((tds.nth(0).inner_text().split("\n")[0], whole, split, busy))
    log.append(f"{name}: {rows.count()} shared rows checked (whole, split, busy %): {'OK' if not bad else bad}")


def tabs(page, name):
    for tab in ["Item flow", "Machine graph"]:
        b = page.get_by_role("tab", name=tab)
        if not b.count():
            log.append(f"{name}: no tab '{tab}' (tabs: {page.get_by_role('tab').all_inner_texts()})")
            continue
        b.first.click()
        page.wait_for_timeout(1200)
        key = tab.split()[0].lower()
        page.screenshot(path=str(OUT / f"{name}-{key}.png"), full_page=True)
        txt = page.locator("main").inner_text()
        (OUT / f"{name}-{key}.txt").write_text(txt, encoding="utf-8")
        log.append(f"{name} / {tab}: {len(txt)} chars, 'share of' lines {txt.count('share of')}, NaN {('NaN' in txt)}")
    page.get_by_role("tab").first.click()
    page.wait_for_timeout(300)


def main():
  with sync_playwright() as p:
    b = p.chromium.launch()
    ctx = b.new_context(viewport={"width": 1400, "height": 1000})
    page = ctx.new_page()
    page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
    page.on("pageerror", lambda e: errors.append(str(e)))
    # 1. gear, each engine: rows, split, busy; flow tabs
    for eng in ["microlp", "highs", "auto"]:
        t = open_plan(page, settings("iron-gear-wheel@4", 10, eng))
        shot(page, f"gear4-{eng}")
        log.append(f"gear4-{eng}: done {t and round(t, 1)} s | {card(page)}")
        check_rows(page, f"gear4-{eng}")
    tabs(page, "gear4-auto")
    # 2. beacons on a shared group (M1): beacon count = whole machines x 8
    s = settings("iron-gear-wheel@4", 10, "highs", beacon_module="speed-module-3", beacon_counts=[8], planets=["nauvis"],
                 objective="custom", cost={"resources": 1, "buildings": 50, "power": 0, "shards": 0, "surplus": 0})
    t = open_plan(page, s)
    shot(page, "gear4-beacons")
    log.append(f"gear4-beacons: done {t and round(t, 1)} s | {card(page)}")
    check_rows(page, "gear4-beacons")
    # 3. quality module, microlp and HiGHS
    for eng in ["microlp", "highs"]:
        t = open_plan(page, settings("quality-module@4", 1, eng))
        shot(page, f"qmod4-{eng}")
        log.append(f"qmod4-{eng}: done {t and round(t, 1)} s | {card(page)}")
        check_rows(page, f"qmod4-{eng}")
    tabs(page, "qmod4-highs")
    # 4. Gleba (H1): legendary plastic over all planets, plants give normal fruit; Stop / Keep solving
    lds = settings("plastic-bar@4", 60, "auto", planets=["nauvis", "vulcanus", "gleba", "fulgora", "aquilo", "space-platform"],
                   research_productivity={"casting-low-density-structure": 1.5}, recycle_allow=["low-density-structure"],
                   time_limit_ms=8000)
    page.goto(URL + "?r=" + str(time.time()) + "#" + hash_of(lds))
    page.wait_for_timeout(3000)
    stop = page.get_by_role("button", name="Stop", exact=True)
    had_stop = stop.count() > 0
    if had_stop:
        stop.first.click()
    t = wait_done(page, 30)
    shot(page, "plastic4-stopped")
    log.append(f"plastic4 all planets: Stop button {had_stop} | after stop: {card(page) if page.locator('.cards').count() else 'no plan'}")
    check_rows(page, "plastic4-stopped")
    keep = page.get_by_role("button", name=re.compile("Keep solving"))
    if keep.count():
        keep.first.click()
        t = wait_done(page, 40)
        shot(page, "plastic4-kept")
        txt = page.locator("main").inner_text()
        log.append(f"plastic4 keep solving: done {t and round(t, 1)} s | {card(page)} | legendary fruit lines: "
                   f"{len(re.findall(r'(Yumako|Jellynut) ★legendary', txt))}")
        check_rows(page, "plastic4-kept")
    else:
        log.append("plastic4: no Keep solving button")
    # 5. machine quality + old link without the field + bad values in a link (L1)
    t = open_plan(page, settings("iron-gear-wheel@4", 10, "microlp", machine_quality=4))
    log.append(f"machine legendary: {card(page)} | select = {page.get_by_label('Quality of all machines').input_value()}")
    shot(page, "gear4-machine-legendary")
    t = open_plan(page, settings("iron-gear-wheel@4", 10, "microlp"))
    log.append(f"old link (no machine_quality): {card(page)} | select = {page.get_by_label('Quality of all machines').input_value()}")
    for bad in [2.5, -1, 9]:
        t = open_plan(page, settings("iron-gear-wheel@4", 10, "microlp", machine_quality=bad))
        ok = page.locator(".cards").count() > 0
        log.append(f"link machine_quality={bad}: plan shown {ok} | select = {page.get_by_label('Quality of all machines').input_value()} | "
                   f"{card(page) if ok else page.locator('main').inner_text()[:200]}")
    page.get_by_label("Quality of all machines").select_option(index=2)
    t = wait_done(page)
    log.append(f"machine quality select -> index 2 (rare): {card(page)}")
    ctx.close()
    # 6. phone width 390 and dark mode
    for name, kw in [("phone390", dict(viewport={"width": 390, "height": 844}, device_scale_factor=2)),
                     ("dark", dict(viewport={"width": 1400, "height": 1000}, color_scheme="dark")),
                     ("phone390-dark", dict(viewport={"width": 390, "height": 844}, device_scale_factor=2, color_scheme="dark"))]:
        ctx = b.new_context(**kw)
        page = ctx.new_page()
        page.on("pageerror", lambda e: errors.append(str(e)))
        t = open_plan(page, settings("iron-gear-wheel@4", 10, "auto"))
        shot(page, f"gear4-{name}")
        over = page.evaluate("document.documentElement.scrollWidth - window.innerWidth")
        log.append(f"gear4-{name}: done {t and round(t, 1)} s | page overflow x {over} px | shared rows {page.locator('tr.shared').count()}")
        if "phone" in name:
            page.locator("tr.shared").first.scroll_into_view_if_needed()
            page.screenshot(path=str(OUT / f"gear4-{name}-table.png"))
        tabs(page, f"gear4-{name}")
        ctx.close()
    b.close()
  log.append(f"console / page errors: {errors if errors else 'none'}")


if __name__ == "__main__":
    main()
    (OUT / "live-log.txt").write_text("\n".join(log), encoding="utf-8")
    print("\n".join(log))
