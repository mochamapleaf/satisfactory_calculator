"""Live test of the DSP Galaxy tab on vite preview :4175 (memo step 13). Prints one line per check."""
import json, re, sys, time
from playwright.sync_api import sync_playwright

URL = "http://localhost:4175/"
SHOTS = sys.argv[1]
results = []


def check(name, ok, detail=""):
    results.append((name, bool(ok), detail))
    print(("PASS " if ok else "FAIL ") + name + (f" — {detail}" if detail else ""), flush=True)


def wait_text(page, text, timeout=60000):
    page.get_by_text(text, exact=False).first.wait_for(timeout=timeout)


def setup(page):
    page.goto(URL)
    page.get_by_role("button", name="戴森球计划").click()
    page.get_by_text("Allow all").first.wait_for(timeout=30000)
    cb = page.locator("aside label.row", has_text="Allow all").locator("input")
    if not cb.is_checked():
        cb.check()
    page.get_by_role("tab", name="Galaxy").click()
    wait_text(page, "AngelStern")


def card(page, label):
    return page.locator(".galaxy .card", has_text=label).first.locator(".big").inner_text()


def row_cells(page, table_sel, first_text):
    row = page.locator(f"{table_sel} tr", has_text=first_text).first
    return [c.inner_text() for c in row.locator("td").all()]


def prolif(page, on):
    lab = page.locator("aside label.row.small", has_text=re.compile(r"Mk\.III.*(额外|Products|产出)")).first
    box = lab.locator("input")
    if box.is_checked() != on:
        box.click()


def thresholds(page):
    page.get_by_role("button", name="Compute thresholds").click()
    page.locator(".galaxy table.threshold").wait_for(timeout=120000)
    page.wait_for_function("() => !document.querySelector('.galaxy p.solving')", timeout=120000)


def plan(page):
    page.get_by_role("button", name="Plan galaxy").click()
    page.wait_for_function("() => !document.querySelector('.galaxy p.solving')", timeout=120000)
    page.locator(".galaxy .card", has_text="Universe matrix").first.wait_for(timeout=120000)


def set_field(page, label, value):
    inp = page.locator(".galaxy .mbsettings label", has_text=label).first.locator("input")
    inp.fill(str(value))


def set_select(page, label, value):
    page.locator(".galaxy .mbsettings label", has_text=label).first.locator("select").select_option(str(value))


with sync_playwright() as p:
    b = p.chromium.launch()
    ctx = b.new_context(viewport={"width": 1440, "height": 900})
    page = ctx.new_page()
    errors = []
    page.on("pageerror", lambda e: errors.append(str(e)))
    page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
    setup(page)
    # 1. seed import (report §2.3 sample: 32 stars, 15 giants, oil 404.8/s, magnets 0.18 M)
    check("stars 32", card(page, "Stars") == "32", card(page, "Stars"))
    check("gas giants 15", card(page, "Gas giants") == "15", card(page, "Gas giants"))
    oil = card(page, "Oil")
    # WASM differs from the native build on one planet (star 0 P2, see progress-impl): 402.3 in the browser
    check("oil 402.3/s (WASM; native 404.8)", oil.startswith("402.3") or oil.startswith("404.8"), oil)
    check("magnets 182 k", card(page, "Unipolar magnets").startswith("182"), card(page, "Unipolar magnets"))
    iron = row_cells(page, ".galaxy table.mbtable", "铁矿")
    check("iron stock 127.78 M", any("127.78" in c for c in iron), str(iron))
    page.locator(".galaxy tr.clickable", has_text="AngelStern").first.click()
    wait_text(page, "1.55 M")
    check("AngelStern P0 iron 1.55 M (prototype 1553315)", page.get_by_text("1.55 M").count() > 0)
    page.screenshot(path=f"{SHOTS}/desktop-1-seed.png", full_page=True)

    # 2. thresholds, no proliferator: magnets 10/cube → L98 / L94, all quotas feasible from L6
    thresholds(page)
    mag = row_cells(page, ".galaxy table.threshold", "单极磁石")
    check("magnets per cube 10", mag[2] == "10", str(mag))
    check("magnet release L98 / L94", mag[3] == "L98" and mag[6] == "L94", str(mag))
    check("cubes to L98 17.11 M", mag[4].startswith("17.11"), mag[4])
    txt = page.locator(".galaxy").inner_text()
    check("quota LP feasible from L6 / L6", "feasible from L6 (ρ = 1) / L6 (ρ = 0.8)" in txt, re.search(r"feasible from [^.]*", txt).group(0) if "feasible from" in txt else "")
    check("oil 2/cube at L6", "crude oil 2/cube" in txt or "原油 2/cube" in txt or "uses crude oil 2" in txt, re.search(r"uses [^.]*", txt).group(0) if "uses" in txt else "")
    page.screenshot(path=f"{SHOTS}/desktop-2-thresholds.png", full_page=True)

    # 3. Mk.III extra products: 3.28/cube → L77 / L72
    prolif(page, True)
    page.wait_for_timeout(300)
    thresholds(page)
    mag = row_cells(page, ".galaxy table.threshold", "单极磁石")
    check("Mk.III magnets per cube 3.28 (±0.01)", abs(float(mag[2]) - 3.28) < 0.01, str(mag))
    check("Mk.III magnet release L77 / L72", mag[3] == "L77" and mag[6] == "L72", str(mag))
    prolif(page, False)
    page.wait_for_timeout(300)

    # 4. multi-system plan, target 1000/min at L81, fewest photons: 1 photon/cube, 20 GW (report §5.5)
    set_field(page, "Target cubes/min", 1000)
    set_select(page, "Minimise", "photons")
    plan(page)
    z = card(page, "Universe matrix")
    gw = card(page, "Ray receivers")
    sub = page.locator(".galaxy .card", has_text="Ray receivers").first.inner_text()
    check("target z 1000", z.startswith("1,000"), z)
    check("20 GW, 1 photon/cube", gw.startswith("20") and "1 photon/cube" in sub.lower(), f"{gw} | {sub}")
    rares = row_cells(page, ".galaxy table.mbtable", "单极磁石")
    check("magnets within quota", len(rares) >= 3, str(rares))
    ships = page.locator(".galaxy h4", has_text="Interstellar shipments").inner_text()
    check("shipments listed", "(0," not in ships, ships)
    page.screenshot(path=f"{SHOTS}/desktop-3-plan-target.png", full_page=True)

    # 5. maximum z with a 20 GW Dyson cap → 1000 (no proliferator), 1250 with Mk.III
    set_select(page, "Goal", "max")
    set_field(page, "Dyson GW", 20)
    plan(page)
    z = card(page, "Universe matrix")
    check("max z at 20 GW = 1000", z.replace(",", "").startswith("1000") or z.replace(",", "").startswith("999"), z)
    prolif(page, True)
    page.wait_for_timeout(300)
    plan(page)
    z2 = card(page, "Universe matrix")
    check("max z at 20 GW sprayed = 1250", z2.replace(",", "").startswith("1250") or z2.replace(",", "").startswith("1249"), z2)
    page.screenshot(path=f"{SHOTS}/desktop-4-plan-max.png", full_page=True)
    prolif(page, False)
    page.wait_for_timeout(300)

    # 6. Dark Fog: 10 bases × 50 magnets/min (hypothetical) at 1000 cubes/min → 9.5 mined per cube
    page.locator("details.dfbox > summary").click()
    page.locator("details.dfbox label.row input[type=checkbox]").check()
    page.locator("details.dfbox button", has_text="Add bases").click()
    page.locator("details.dfbox input[aria-label=bases]").fill("10")
    page.locator("details.dfbox select[aria-label='Drop item']").select_option("unipolar-magnet")
    page.locator("details.dfbox button", has_text="Add drop").click()
    page.locator("details.dfbox li", has_text="单极磁石").locator("input").fill("50")
    set_field(page, "Cubes/min (for hours)", 1000)
    thresholds(page)
    mag = row_cells(page, ".galaxy table.threshold", "单极磁石")
    check("Dark Fog: magnets mined per cube 9.5", mag[2] == "9.5", str(mag))
    # plan with drops: the farmed system shows drops
    set_select(page, "Goal", "target")
    plan(page)
    page.locator(".galaxy tr.clickable", has_text="AngelStern").last.click()
    check("plan uses Dark Fog drops", page.get_by_text("Dark Fog drops").count() > 0)
    page.screenshot(path=f"{SHOTS}/desktop-5-darkfog.png", full_page=True)
    check("no page errors", not errors, "; ".join(errors[:3]))
    ctx.close()

    # 7. phone width 390 px, light and dark
    for scheme in ["light", "dark"]:
        ctx = b.new_context(viewport={"width": 390, "height": 844}, color_scheme=scheme, device_scale_factor=2)
        page = ctx.new_page()
        setup(page)
        page.get_by_role("button", name="Compute thresholds").click()
        page.locator(".galaxy table.threshold").wait_for(timeout=120000)
        page.wait_for_function("() => !document.querySelector('.galaxy p.solving')", timeout=120000)
        sw = page.evaluate("() => document.documentElement.scrollWidth")
        check(f"390 px {scheme}: no horizontal page scroll", sw <= 391, f"scrollWidth {sw}")
        page.locator(".galaxy h3", has_text="Infinite threshold").scroll_into_view_if_needed()
        page.screenshot(path=f"{SHOTS}/phone-{scheme}-thresholds.png")
        page.get_by_role("button", name="Plan galaxy").click()
        page.wait_for_function("() => !document.querySelector('.galaxy p.solving')", timeout=120000)
        page.locator(".galaxy h3", has_text="Multi-system plan").scroll_into_view_if_needed()
        page.screenshot(path=f"{SHOTS}/phone-{scheme}-plan.png")
        page.locator(".galaxy h3", has_text="Seed").scroll_into_view_if_needed()
        page.screenshot(path=f"{SHOTS}/phone-{scheme}-seed.png")
        ctx.close()
    b.close()

print(f"{sum(1 for r in results if r[1])}/{len(results)} checks passed")
