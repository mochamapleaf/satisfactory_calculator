"""Live browser test for review 5 of the Mixed belts (混带) tab (docs/review/fable-review-5.md).
python -X utf8 -I live_test_review5.py <shots_dir> [base]
Runs on vite preview (Chromium, headless): every finding of the review that shows in the page."""
import base64, json, sys, time, urllib.parse
from playwright.sync_api import sync_playwright

shots = sys.argv[1]
base = sys.argv[2] if len(sys.argv) > 2 else 'http://localhost:4173/'
results = []


def frag(s): return base64.b64encode(urllib.parse.quote(json.dumps(s)).encode()).decode()
def say(**k): print(json.dumps(k, ensure_ascii=False)[:700], flush=True)
def check(name, ok, detail=''):
    results.append((name, bool(ok))); say(CHECK=name, ok=bool(ok), detail=str(detail)[:500])


def dsp(target, rate, effects, supplies, **kw):
    s = {'game': 'dsp', 'objective': 'buildings', 'integer': True, 'time_limit_ms': 3000, 'disabled_buildings': [],
         'mixed_belts': True, 'effects': effects, 'targets': [{'item': target, 'rate': rate}],
         'supplies': [{'item': i, 'rate': 1e12} for i in supplies]}
    s.update(kw)
    return s


MK2 = dict(effects=['proliferator-3-speed'], supplies=['diamond', 'proliferator-1', 'proliferator-3'])
MK3 = dict(effects=['proliferator-3-products'], supplies=['proliferator-2', 'carbon-nanotube'])
ROWS = "() => [...document.querySelectorAll('.mbtable tbody tr')].map(tr => [...tr.cells].map(c => c.textContent.trim()))"
NOTES = "() => [...document.querySelectorAll('.notes li')].map(l => l.textContent.trim())"
KV = """() => { const kv = {}; for (const tr of document.querySelectorAll('.mbcols table tbody tr')) if (tr.cells.length === 2) kv[tr.cells[0].textContent.trim()] = tr.cells[1].textContent.trim(); return kv }"""


def settle(page, tab=True, limit=25):
    page.wait_for_selector('.tabs button', timeout=30000)
    t0 = time.time()
    while time.time() - t0 < limit:
        if page.query_selector('.solving') is None:
            break
        time.sleep(0.2)
    time.sleep(0.4)
    if tab:
        page.get_by_role('tab', name='Mixed belts').click()
        page.wait_for_selector('.mixedbelt', timeout=10000)
        time.sleep(0.3)


def open_case(page, s, **kw):
    page.goto(base + '#' + frag(s))
    page.reload()
    settle(page, **kw)


def field(page, label):
    return page.locator('.mbsettings label', has_text=label).locator('input, select').first


with sync_playwright() as p:
    b = p.chromium.launch()
    errors = []
    ctx = b.new_context(viewport={'width': 1400, 'height': 1000}, permissions=['clipboard-read', 'clipboard-write'])
    page = ctx.new_page()
    page.on('pageerror', lambda e: errors.append(str(e)[:300]))

    # ---- H1: a fractional number typed into an integer setting
    open_case(page, dsp('proliferator-2', 10800, **MK2))
    field(page, '每排台数').fill('7.5')
    time.sleep(1.0)
    rows = page.evaluate(ROWS)
    check('H1 R = 7.5: page still there, layout shown', len(rows) == 1 and rows[0][7] == '通过' and not errors, (rows, errors))
    check('H1 R = 7.5 is taken as 8', field(page, '每排台数').input_value() == '8', field(page, '每排台数').input_value())
    page.screenshot(path=f'{shots}/h1-fractional-R.png')
    page.locator('.mbsettings summary').click()
    for label, typed, want in [('短组', '-3', '0'), ('梯口组长上限', '1e12', '1000'), ('单控制单元台数上限', '0.4', ''), ('且 > 台', '2.5', '3')]:
        f = field(page, label); f.fill(typed); time.sleep(0.3)
        check(f'H1 {label} = {typed} → {want or "unset"}', f.input_value() == want and len(page.evaluate(ROWS)) == 1, f.input_value())
    page.reload(); settle(page)
    check('H1 after reload: plan and layout, no page error', len(page.evaluate(ROWS)) == 1 and not errors, errors)
    # a link / saved settings written by the build before the fix (bad numbers inside)
    bad = dsp('proliferator-2', 10800, **MK2, belt_layout={'row_len': 7.5, 'ladder_short': -1, 'belt': 9, 'margin': None, 'unit_cap': 'x',
                                                              'overrides': {'proliferator-2|df-recomposing-assembler|proliferator-3-speed': {'k': 1e12, 'sigma5': 9}}})
    open_case(page, bad, tab=False)
    plan_rows = page.evaluate("() => document.querySelectorAll('main table tbody tr').length")
    page.get_by_role('tab', name='Mixed belts').click(); page.wait_for_selector('.mixedbelt'); time.sleep(0.3)
    rows = page.evaluate(ROWS)
    check('H1 link with bad belt settings: the plan solves and the tab lays out', plan_rows > 0 and len(rows) == 1 and not errors, (plan_rows, rows, errors))

    # ---- M1: return flow is part of the choice
    open_case(page, dsp('proliferator-2', 9720, **MK2))
    rows = page.evaluate(ROWS); kv = page.evaluate(KV)
    check('M1 Mk.II 27 machines: 通过, P 6, returns 7.5 / 9 within shares', rows[0][1] == '27' and rows[0][3] == 'P 6' and rows[0][7] == '通过' and kv.get('回流 return (A / B) · 份额 shares', '').startswith('7.5 / 9'), (rows, kv))
    page.screenshot(path=f'{shots}/m1-mk2-27.png', full_page=True)
    open_case(page, dsp('proliferator-2', 10440, **MK2))
    rows = page.evaluate(ROWS); notes = page.evaluate(NOTES)
    check('M1 Mk.II 29 machines: 通过 with P 3 and the reason', rows[0][1] == '29' and rows[0][3] == 'P 3' and rows[0][7] == '通过' and any('instead of the largest' in n for n in notes), (rows, notes))
    cands = page.evaluate("() => [...document.querySelectorAll('.mbcols table')].pop().innerText")
    check('M1 candidates table says why P 6 is out', 'return flow' in cands, cands)
    open_case(page, dsp('proliferator-2', 10800, **MK2, belt_layout={'row_len': 15}))
    rows = page.evaluate(ROWS); kv = page.evaluate(KV)
    check('M1 whole periods unchanged: 30 machines P 6, 4.5 / 4.5, returns 12 / 13.5', rows[0][3] == 'P 6' and kv.get('最低流量 steady-state min (A / B)') == '4.5 / 4.5' and kv.get('回流 return (A / B) · 份额 shares', '').startswith('12 / 13.5'), kv)

    # ---- M2: processors, one belt for two rows: interval 8 positions (P 32), 3 / 3; interval 9 by hand gives 1 / 2
    cpu = dsp('processor', 3000, ['proliferator-3-products'], ['circuit-board', 'microcrystalline-component', 'proliferator-3'], belt_layout={'rows': 2})
    open_case(page, cpu)
    rows = page.evaluate(ROWS); kv = page.evaluate(KV)
    check('M2 processors two rows: P 32, steady 3 / 3', rows[0][3] == 'P 32' and kv.get('最低流量 steady-state min (A / B)') == '3 / 3', (rows, kv))
    page.locator('.ovgrid select').first.select_option('36'); time.sleep(0.4)
    kv = page.evaluate(KV); notes = page.evaluate(NOTES)
    mins = sorted(kv.get('最低流量 steady-state min (A / B)', '').split(' / '))
    check('M2 interval 9 (P 36) by hand: 1 / 2, flagged below m', mins == ['1', '2'] and any('lowest flow 1.000 < m' in n for n in notes), (kv, notes))
    page.screenshot(path=f'{shots}/m2-processors-interval9.png', full_page=True)
    hint = page.locator('.mbcols .hint').first.inner_text()
    check('L Reset link and recommendation shown with a manual period', 'Reset' in hint and 'P = 32' in hint, hint)
    page.get_by_text('Reset', exact=True).click(); time.sleep(0.4)
    check('L Reset returns to P 32', page.evaluate(ROWS)[0][3] == 'P 32')

    # ---- M3: another belt
    open_case(page, dsp('electromagnetic-turbine', 900, ['proliferator-3-products'], ['electric-motor', 'magnetic-coil', 'proliferator-3'], belt_layout={'belt': 1}))
    rows = page.evaluate(ROWS); notes = page.evaluate(NOTES)
    check('M3 green mixed belt: flagged as an extrapolation of the blue-belt unit', rows[0][7] != '通过' and any('blue belt' in n for n in notes), (rows, notes))

    # ---- M4: very long lines
    t0 = time.time()
    open_case(page, dsp('proliferator-3', 1e9, **MK3), limit=40)
    dt = time.time() - t0
    rows = page.evaluate(ROWS); notes = page.evaluate(NOTES)
    check('M4 1e9/min Mk.III proliferator: tab opens, clear failure', len(rows) == 1 and rows[0][7] == '不通过' and any('unit cap' in n for n in notes) and not errors, (round(dt, 1), rows, notes[:2], errors))
    page.locator('.mbsettings summary').click()
    t0 = time.time(); field(page, '单控制单元台数上限').fill('300'); time.sleep(0.2); page.wait_for_selector('svg.belt', timeout=20000); dt = time.time() - t0
    rows = page.evaluate(ROWS)
    check('M4 with a unit cap of 300 it lays out', rows[0][7] != '不通过' and rows[0][3].startswith('P '), (round(dt, 2), rows))
    open_case(page, dsp('proliferator-3', 1292400, **MK3))
    rows = page.evaluate(ROWS)
    rects = page.evaluate("() => document.querySelectorAll('svg.belt rect.mach').length")
    hint = ' '.join(page.evaluate("() => [...document.querySelectorAll('.mbdetail > .hint')].map(h => h.textContent)"))
    check('M4 a 12 000-machine line: short diagram, omission stated', int(rows[0][1]) > 11000 and rects < 800 and '省略' in hint, (rows[0][:4], rects, hint[:120]))
    page.screenshot(path=f'{shots}/m4-long-line.png')

    # ---- M5: stale plan (a slow solve, settings changed while on the tab)
    big = dsp('universe-matrix', 6000, ['proliferator-3-speed', 'proliferator-3-products'], [], time_limit_ms=12000)
    open_case(page, big, limit=40)
    rows0 = page.evaluate(ROWS)
    a_rows = [r for r in rows0 if r[2].startswith('A')]
    fails = [r for r in a_rows if r[7] == '不通过']
    check('M1 universe-matrix plan: scheme-A rows failing', len(fails) <= 1, (len(a_rows), [(r[0][:14], r[1], r[3]) for r in fails]))
    page.screenshot(path=f'{shots}/universe-matrix.png', full_page=True)
    t0 = time.time()
    page.locator('aside input[type="number"], .sidebar input[type="number"]').first.fill('6600')
    seen = False; stale_rows = None
    while time.time() - t0 < 12:
        if page.query_selector('.mixedbelt .note[role="status"]'):
            seen = True; stale_rows = page.evaluate(ROWS); break
        time.sleep(0.1)
    check('M5 during the new solve: "Earlier plan" note, same rows, no failed modes', seen and stale_rows is not None and [r[:2] for r in stale_rows] == [r[:2] for r in rows0], (seen, len(stale_rows or [])))
    if seen:
        page.screenshot(path=f'{shots}/m5-earlier-plan.png')
    settle(page, tab=False, limit=40)
    check('M5 after the solve the note is gone', page.query_selector('.mixedbelt .note[role="status"]') is None)

    # ---- L: turbine recommendation = layout shown; odd k with two rows; copy
    turb = dsp('electromagnetic-turbine', 6750, ['proliferator-3-products'], ['electric-motor', 'magnetic-coil', 'proliferator-3'], belt_layout={'row_len': 15})
    open_case(page, turb)
    kv = page.evaluate(KV)
    k_auto = page.locator('.ovgrid input[type="number"]').get_attribute('placeholder')
    page.locator('.ovgrid select').nth(2).select_option('4'); time.sleep(0.4)
    hint = page.locator('.mbcols .hint').first.inner_text()
    check('L recommendation text names the layout that auto showed', f'k = {k_auto},' in hint and '3.75' in hint, (k_auto, hint))
    page.get_by_text('Reset', exact=True).click(); time.sleep(0.3)
    seq = page.locator('.seq').inner_text()
    page.locator('.seqbox button').click(); time.sleep(0.3)
    clip = page.evaluate('navigator.clipboard.readText()')
    check('L Copy puts the sequence on the clipboard and says so', clip == seq and page.locator('.seqbox button').inner_text() == 'Copied', (clip[:60], page.locator('.seqbox button').inner_text()))
    mk3 = dsp('proliferator-3', 14850, **MK3, belt_layout={'row_len': 15, 'rows': 2})
    open_case(page, mk3)
    rows = page.evaluate(ROWS); kv = page.evaluate(KV)
    check('his Mk.III line, two rows R 15: P 30, min 1.5 (2.25 / 1.5 as his k = 18)', rows[0][3] == 'P 30' and rows[0][4] == '1.5', (rows, kv.get('最低流量 steady-state min (A / B)')))
    page.locator('.ovgrid input[type="number"]').fill('17'); time.sleep(0.4)
    notes = page.evaluate(NOTES)
    check('L odd k with two rows is flagged', any('even number' in n for n in notes), notes)
    page.screenshot(path=f'{shots}/l-odd-k-two-rows.png', full_page=True)

    # ---- no plan: the settings bar stays
    open_case(page, dsp('proliferator-2', 10800, effects=['proliferator-3-speed'], supplies=[], disabled_recipes=['proliferator-2']), tab=False)
    page.get_by_role('tab', name='Mixed belts').click(); time.sleep(0.5)
    txt = page.inner_text('main')
    check('no plan: settings bar still there, "Solve a plan first"', page.query_selector('.mbsettings') is not None and 'Solve a plan first' in txt, txt[-200:])

    # ---- 390 px and dark
    ph = b.new_context(viewport={'width': 390, 'height': 844}, color_scheme='dark')
    pp = ph.new_page(); pp.on('pageerror', lambda e: errors.append(str(e)[:300]))
    open_case(pp, dsp('proliferator-3', 1292400, **MK3))
    dims = pp.evaluate("() => ({ doc: document.documentElement.scrollWidth, win: innerWidth, belt: document.querySelector('svg.belt')?.getBoundingClientRect().width, box: document.querySelector('.beltwrap')?.clientWidth })")
    check('390 px dark, long line: page does not scroll sideways, diagram scrolls in its box', dims['doc'] <= dims['win'] and dims['belt'] > dims['box'], dims)
    pp.screenshot(path=f'{shots}/390-dark-long-line.png', full_page=True)
    open_case(pp, cpu)
    pp.locator('.ovgrid select').first.select_option('36'); time.sleep(0.4)
    dims = pp.evaluate("() => ({ doc: document.documentElement.scrollWidth, win: innerWidth })")
    check('390 px dark, manual layout with notes: no sideways scroll', dims['doc'] <= dims['win'], dims)
    pp.screenshot(path=f'{shots}/390-dark-override.png', full_page=True)

    check('no page errors in the whole run', not errors, errors)
    b.close()

bad = [n for n, ok in results if not ok]
print(f'{len(results) - len(bad)}/{len(results)} checks pass', 'FAILED: ' + '; '.join(bad) if bad else '')
