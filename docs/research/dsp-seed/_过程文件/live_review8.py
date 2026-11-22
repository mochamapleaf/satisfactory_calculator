"""Review 8: live test of the DSP Galaxy tab on vite preview :4175 (fixed build). One line per check.
usage: python live_review8.py <screenshot dir>"""
import re, sys
from playwright.sync_api import sync_playwright

URL = "http://localhost:4175/"
SHOTS = sys.argv[1]
results = []


def check(name, ok, detail=""):
    results.append((name, bool(ok), detail))
    print(("PASS " if ok else "FAIL ") + name + (f" — {detail}" if detail else ""), flush=True)


def setup(page):
    page.goto(URL)
    page.get_by_role("button", name="戴森球计划").click()
    page.get_by_text("Allow all").first.wait_for(timeout=30000)
    cb = page.locator("aside label.row", has_text="Allow all").locator("input")
    if not cb.is_checked():
        cb.check()
    page.get_by_role("tab", name="Galaxy").click()
    page.get_by_text("AngelStern").first.wait_for(timeout=60000)


def card(page, label):
    return page.locator(".galaxy .card", has_text=label).first.locator(".big").inner_text()


def field(page, label):
    return page.locator(".galaxy .mbsettings label", has_text=label).first.locator("input")


def type_into(page, label, text):
    """Real key presses (select all, type), then leave the field."""
    inp = field(page, label)
    inp.click()
    page.keyboard.press("Control+A")
    if text == "":
        page.keyboard.press("Delete")
    else:
        page.keyboard.type(text, delay=40)
    page.keyboard.press("Tab")
    return inp.input_value()


def idle(page):
    page.wait_for_function("() => !document.querySelector('.galaxy p.solving')", timeout=180000)


def thresholds(page):
    page.get_by_role("button", name="Compute thresholds").click()
    page.locator(".galaxy table.threshold").wait_for(timeout=120000)
    idle(page)


def plan(page):
    page.get_by_role("button", name="Plan galaxy").click()
    idle(page)
    page.locator(".galaxy .card", has_text="Universe matrix").first.wait_for(timeout=120000)


def stale(page, which):
    return page.locator(".galaxy p.warn.stale", has_text=which).count() > 0


def prolif(page, on):
    lab = page.locator("aside label.row.small", has_text=re.compile(r"Mk\.III.*(额外|Products|产出)")).first
    box = lab.locator("input")
    if box.is_checked() != on:
        box.click()


def row_cells(page, table_sel, first_text):
    row = page.locator(f"{table_sel} tr", has_text=first_text).first
    return [c.inner_text() for c in row.locator("td").all()]


with sync_playwright() as p:
    b = p.chromium.launch()
    ctx = b.new_context(viewport={"width": 1440, "height": 900})
    page = ctx.new_page()
    errors = []
    page.on("pageerror", lambda e: errors.append(str(e)))
    page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
    setup(page)

    # H1: the browser gives the native numbers
    oil = card(page, "Oil")
    check("H1 oil 404.8/s in the browser (was 402.3)", oil.startswith("404.8"), oil)
    txt = page.locator(".galaxy").inner_text()
    check("H1 fingerprint a42df4b4a9e7ccc9 shown (= native test)", "a42df4b4a9e7ccc9" in txt)
    check("H1 old 24.1 vs 26.7 note gone", "24.1 vs 26.7" not in txt)
    page.locator(".galaxy tr.clickable", has_text="AngelStern").first.click()
    page.get_by_text("26.7/s").first.wait_for(timeout=10000)
    check("H1 home planet oil 26.7/s (19 seeps)", page.get_by_text("26.7/s (19)").count() > 0)
    gen_s = card(page, "Generated")
    check("generator time shown", True, f"{gen_s} for 32 stars in the browser")

    # M4: licence shipped with the build and linked
    for f, want in [("DSP-Seed-Finder-LICENSE.txt", "Apache License"), ("DSP-Seed-Finder-NOTICE.txt", "detmath")]:
        r = page.request.get(URL + "licenses/" + f)
        check(f"M4 licenses/{f} served", r.status == 200 and want in r.text(), f"HTTP {r.status}")
    check("M4 licence links in the tab", page.locator(".galaxy a[href$='DSP-Seed-Finder-LICENSE.txt']").count() == 1 and page.locator(".galaxy a[href$='DSP-Seed-Finder-NOTICE.txt']").count() == 1)

    # L: no 10× multiplier
    opts = page.locator(".galaxy .mbsettings label", has_text="Resources").first.locator("option").all_inner_texts()
    check("L 10 resource settings, no 10×", len(opts) == 10 and not any(o.startswith("10×") for o in opts), str(opts))

    # M2: typing with real key presses
    v = type_into(page, "Stars", "48")
    check("M2 typing 48 into Stars gives 48 (was 322 → 64)", v == "48", v)
    txt = page.locator(".galaxy").inner_text()
    check("M2 note: fields differ from the imported galaxy", "differ from the imported galaxy" in txt)
    v = type_into(page, "Stars", "5")
    check("M2 Stars 5 is clamped to 32 when the field is left", v == "32", v)
    v = type_into(page, "Quota level L", "12")
    check("M2 typing 12 into Quota level gives 12 (was 62)", v == "12", v)
    v = type_into(page, "Quota level L", "81")
    v = type_into(page, "Cubes/min (for hours)", "")
    check("M2 emptied field falls back to its value", v == "750", v)
    v = type_into(page, "Building cap", "5000")
    v2 = type_into(page, "Building cap", "")
    check("M2 building cap: 5000 then empty = none", v == "5000" and v2 == "", f"{v} / '{v2}'")
    page.screenshot(path=f"{SHOTS}/r8-1-seed.png", full_page=True)

    # thresholds (report numbers) and M3 stale mark
    thresholds(page)
    mag = row_cells(page, ".galaxy table.threshold", "单极磁石")
    check("thresholds: magnets 10/cube, L98 / L94", mag[2] == "10" and mag[3] == "L98" and mag[6] == "L94", str(mag))
    ice = row_cells(page, ".galaxy table.threshold", "可燃冰")
    check("L fire ice row tagged '+ gas giants'", "+ gas giants" in ice[0].lower(), ice[0])
    check("M3 fresh thresholds: no out-of-date mark", not stale(page, "thresholds"))
    type_into(page, "Quota level L", "60")
    check("M3 level changed: thresholds marked out of date", stale(page, "thresholds"))
    page.screenshot(path=f"{SHOTS}/r8-2-thresholds-stale.png", full_page=True)
    thresholds(page)
    check("M3 recomputed: mark gone, header says L60", not stale(page, "thresholds") and "quota/cube at l60" in page.locator(".galaxy table.threshold").inner_text().lower())
    type_into(page, "Quota level L", "81")
    prolif(page, True)
    page.wait_for_timeout(300)
    check("M3 sidebar proliferator changed: thresholds marked out of date", stale(page, "thresholds"))
    thresholds(page)
    mag = row_cells(page, ".galaxy table.threshold", "单极磁石")
    check("thresholds Mk.III: 3.28/cube, L77 / L72", abs(float(mag[2]) - 3.28) < 0.01 and mag[3] == "L77" and mag[6] == "L72", str(mag))
    prolif(page, False)
    page.wait_for_timeout(300)

    # plan: target 1000, fewest photons
    type_into(page, "Target cubes/min", "1000")
    page.locator(".galaxy .mbsettings label", has_text="Minimise").first.locator("select").select_option("photons")
    plan(page)
    z = card(page, "Universe matrix")
    gw = card(page, "Ray receivers")
    sub = page.locator(".galaxy .card", has_text="Ray receivers").first.inner_text()
    check("plan: z 1000, 20 GW, 1 photon/cube", z.startswith("1,000") and gw.startswith("20") and "1 photon/cube" in sub.lower(), f"{z} | {gw} | {sub}")
    line = page.locator(".galaxy p.small.muted", has_text="Buildings:").first.inner_text()
    m = re.search(r"oil extractors ([\d,.]+).*stations ([\d,.]+)\.\s*LP ([\d,]+) columns × ([\d,]+) rows, ([\d.]+) s", line, re.S)
    check("L3 oil extractors listed; M1 stations with 7200/min belts", bool(m), line.replace("\n", " "))
    if m:
        check("plan size and time (8 systems)", True, f"{m.group(3)} columns × {m.group(4)} rows, {m.group(5)} s in the browser; stations {m.group(2)}, oil extractors {m.group(1)}")
    check("M3 fresh plan: no out-of-date mark", not stale(page, "plan"))
    type_into(page, "Target cubes/min", "2000")
    check("M3 target changed: plan marked out of date", stale(page, "plan"))
    type_into(page, "Target cubes/min", "1000")
    check("M3 target back: mark gone", not stale(page, "plan"))
    # tick four more systems → out of date, then a 12-system LP
    boxes = page.locator(".galaxy input[aria-label^='Plan ']")
    added = 0
    for i in range(boxes.count()):
        if added >= 4:
            break
        if not boxes.nth(i).is_checked():
            boxes.nth(i).check()
            added += 1
    check("M3 systems ticked: plan marked out of date", stale(page, "plan"))
    page.screenshot(path=f"{SHOTS}/r8-3-plan-stale.png", full_page=True)
    plan(page)
    line = page.locator(".galaxy p.small.muted", has_text="Buildings:").first.inner_text()
    m = re.search(r"LP ([\d,]+) columns × ([\d,]+) rows, ([\d.]+) s", line)
    check("12-system LP solves in the browser", bool(m) and card(page, "Universe matrix").startswith("1,000"), line[-60:].replace("\n", " "))
    page.get_by_role("button", name="Automatic pick").click()

    # max z with 20 GW
    page.locator(".galaxy .mbsettings label", has_text="Goal").first.locator("select").select_option("max")
    type_into(page, "Dyson GW", "20")
    plan(page)
    z = card(page, "Universe matrix").replace(",", "")
    check("max z at 20 GW = 1000", z.startswith("1000") or z.startswith("999"), z)
    page.screenshot(path=f"{SHOTS}/r8-4-plan-max.png", full_page=True)

    # Dark Fog: 10 bases × 50 magnets/min at 1000 cubes/min → 9.5 mined per cube
    page.locator("details.dfbox > summary").click()
    page.locator("details.dfbox label.row input[type=checkbox]").check()
    page.locator("details.dfbox button", has_text="Add bases").click()
    page.locator("details.dfbox input[aria-label=bases]").fill("10")
    page.locator("details.dfbox select[aria-label='Drop item']").select_option("unipolar-magnet")
    page.locator("details.dfbox button", has_text="Add drop").click()
    page.locator("details.dfbox li", has_text="单极磁石").locator("input").fill("50")
    type_into(page, "Cubes/min (for hours)", "1000")
    check("M3 Dark Fog edited: both results marked out of date", stale(page, "thresholds") and stale(page, "plan"))
    thresholds(page)
    mag = row_cells(page, ".galaxy table.threshold", "单极磁石")
    check("Dark Fog: magnets mined per cube 9.5", mag[2] == "9.5", str(mag))
    page.locator(".galaxy .mbsettings label", has_text="Goal").first.locator("select").select_option("target")
    plan(page)
    page.locator(".galaxy tr.clickable", has_text="AngelStern").last.click()
    check("plan uses Dark Fog drops", page.get_by_text("Dark Fog drops").count() > 0)
    page.screenshot(path=f"{SHOTS}/r8-5-darkfog.png", full_page=True)

    # import another galaxy: 64 stars, 1× (the seed with published home-planet values)
    type_into(page, "Seed", "29031989")
    type_into(page, "Stars", "64")
    page.locator(".galaxy .mbsettings label", has_text="Resources").first.locator("select").select_option("1")
    page.get_by_role("button", name="Import seed").click()
    page.get_by_text("Ruchba").first.wait_for(timeout=60000)
    idle(page)
    txt = page.locator(".galaxy").inner_text()
    check("import 29031989 / 64 / 1×: 64 stars, fingerprint 0ac61e60b96eab08", card(page, "Stars") == "64" and "0ac61e60b96eab08" in txt, card(page, "Generated") + " generate")
    check("import clears the old results", page.locator(".galaxy table.threshold").count() == 0 and page.locator(".galaxy .card", has_text="Universe matrix").count() == 0)
    if page.get_by_text("49.78/s (18)").count() == 0:  # star 0 may still be open from the first galaxy
        page.locator(".galaxy tr.clickable", has_text="Ruchba").first.click()
    check("published value: home planet oil 49.78/s (Steam 2022)", page.get_by_text("49.78/s (18)").count() > 0)
    check("no page errors", not errors, "; ".join(errors[:3]))
    ctx.close()

    # L: a worker that cannot load ends with an error, not an endless "Generating…"
    ctx = b.new_context(viewport={"width": 1200, "height": 800})
    page = ctx.new_page()
    page.route(re.compile(r"galaxyWorker.*\.js"), lambda r: r.abort())
    page.goto(URL)
    page.get_by_role("button", name="戴森球计划").click()
    page.get_by_role("tab", name="Galaxy").click()
    try:
        page.locator(".galaxy p.warn", has_text="galaxy worker failed").wait_for(timeout=15000)
        ok = page.locator(".galaxy p.solving").count() == 0
    except Exception:
        ok = False
    check("L worker blocked: error shown, not busy for ever", ok)
    ctx.close()

    # phone 390 px, light and dark
    for scheme in ["light", "dark"]:
        ctx = b.new_context(viewport={"width": 390, "height": 844}, color_scheme=scheme, device_scale_factor=2)
        page = ctx.new_page()
        setup(page)
        thresholds(page)
        plan(page)
        type_into(page, "Quota level L", "70")
        sw = page.evaluate("() => document.documentElement.scrollWidth")
        check(f"390 px {scheme}: no horizontal page scroll (with out-of-date marks)", sw <= 391 and stale(page, "thresholds") and stale(page, "plan"), f"scrollWidth {sw}")
        page.locator(".galaxy h3", has_text="Infinite threshold").scroll_into_view_if_needed()
        page.screenshot(path=f"{SHOTS}/r8-phone-{scheme}-thresholds.png")
        page.locator(".galaxy h3", has_text="Multi-system plan").scroll_into_view_if_needed()
        page.screenshot(path=f"{SHOTS}/r8-phone-{scheme}-plan.png")
        page.locator(".galaxy h3", has_text="Seed").scroll_into_view_if_needed()
        page.screenshot(path=f"{SHOTS}/r8-phone-{scheme}-seed.png")
        ctx.close()
    b.close()

print(f"{sum(1 for r in results if r[1])}/{len(results)} checks passed")
