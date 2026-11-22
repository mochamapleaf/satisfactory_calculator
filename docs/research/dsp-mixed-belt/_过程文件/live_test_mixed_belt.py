"""Live browser test of the Mixed belts (混带) tab. python -X utf8 -I live_test_mixed_belt.py <shots_dir> [base]
Plans 笑笑's lines on vite preview, opens the tab, compares the numbers with the report's worked cases, screenshots
desktop (1400 px) and phone (390 px)."""
import base64, json, sys, time, urllib.parse
from playwright.sync_api import sync_playwright

shots = sys.argv[1]
base = sys.argv[2] if len(sys.argv) > 2 else 'http://localhost:4173/'
results = []


def frag(s): return base64.b64encode(urllib.parse.quote(json.dumps(s)).encode()).decode()
def say(**k): print(json.dumps(k, ensure_ascii=False)[:900], flush=True)
def check(name, ok, detail=''):
    results.append((name, bool(ok), str(detail)[:300])); say(CHECK=name, ok=bool(ok), detail=str(detail)[:300])


def dsp(target, rate, effects, supplies, **kw):
    s = {'game': 'dsp', 'objective': 'buildings', 'integer': True, 'time_limit_ms': 3000, 'disabled_buildings': [],
         'mixed_belts': True, 'effects': effects, 'targets': [{'item': target, 'rate': rate}],
         'supplies': [{'item': i, 'rate': 1e6} for i in supplies]}
    s.update(kw)
    return s


CASES = {
    # ep19 Mk.II proliferator (绿剂): recomposing ×3, Mk.III speed-up, 30 machines → P 6, min 4.5 / 4.5, double unit
    'mk2': dsp('proliferator-2', 10800, ['proliferator-3-speed'], ['diamond', 'proliferator-1', 'proliferator-3'],
               belt_layout={'row_len': 15}),
    # ep19 carbon nanotube (碳管): quantum chemical plant ×2, speed-up, 32 machines → P 32, min 2 / 2, returns 8 / 6
    'cnt': dsp('carbon-nanotube', 3840, ['proliferator-3-speed'], ['graphene', 'titanium-ingot', 'proliferator-3'],
               disabled_recipes=['carbon-nanotube-advanced']),
    # ep19 Mk.III proliferator (蓝剂): extra products, one belt for two rows, R 15 → P 30, min 1.5
    'mk3': dsp('proliferator-3', 14850, ['proliferator-3-products'], ['proliferator-2', 'carbon-nanotube'],
               belt_layout={'row_len': 15, 'rows': 2}),
    # ep20 processors: extra products, two rows → interval 8 positions (P 32)
    'cpu': dsp('processor', 3000, ['proliferator-3-products'], ['circuit-board', 'microcrystalline-component', 'proliferator-3'],
               belt_layout={'rows': 2}),
    # ep20 turbine (绿马达): extra products, R 15, 60 machines → P 20, min 3.75, returns 4.5 / 12
    'turbine': dsp('electromagnetic-turbine', 6750, ['proliferator-3-products'], ['electric-motor', 'magnetic-coil', 'proliferator-3'],
                   belt_layout={'row_len': 15}),
    # ep20 electric motor: three ingredients → ladder groups of 10
    'motor': dsp('electric-motor', 4500, ['proliferator-3-speed'], ['gear', 'iron-ingot', 'magnetic-coil', 'proliferator-3']),
}

ROWS_JS = """() => [...document.querySelectorAll('.mbtable tbody tr')].map(tr => [...tr.cells].map(c => c.textContent.trim()))"""
DETAIL_JS = """(name) => {
  const sec = [...document.querySelectorAll('.mbrecipe')].find(s => s.querySelector('h3').textContent.includes(name)) ;
  if (!sec) return null;
  const kv = {};
  for (const tr of sec.querySelectorAll('.mbcols table tbody tr')) { if (tr.cells.length === 2) kv[tr.cells[0].textContent.trim()] = tr.cells[1].textContent.trim() }
  return { kv, seq: sec.querySelector('.seq')?.textContent ?? '', notes: [...sec.querySelectorAll('.notes li')].map(l => l.textContent.trim()),
           svg: !!sec.querySelector('svg.belt'), hint: [...sec.querySelectorAll('.hint')].map(h => h.textContent).join(' | ') };
}"""


def open_case(page, s):
    page.goto(base + '#' + frag(s))
    page.wait_for_selector('.tabs button', timeout=30000)
    t0 = time.time()
    while time.time() - t0 < 20:
        if page.query_selector('.solving') is None and page.query_selector('table tbody tr'):
            break
        time.sleep(0.2)
    time.sleep(0.3)
    page.get_by_role('tab', name='Mixed belts').click()
    page.wait_for_selector('.mixedbelt', timeout=10000)
    time.sleep(0.3)


def row_for(rows, rid_name):
    return next((r for r in rows if rid_name in r[0]), None)


with sync_playwright() as p:
    b = p.chromium.launch()
    errors = []
    ctx = b.new_context(viewport={'width': 1400, 'height': 1000})
    page = ctx.new_page()
    page.on('pageerror', lambda e: errors.append(str(e)[:300]))
    page.on('console', lambda m: m.type == 'error' and errors.append(m.text[:300]))

    # --- Mk.II proliferator
    open_case(page, CASES['mk2'])
    rows = page.evaluate(ROWS_JS)
    say(case='mk2', rows=rows)
    r = row_for(rows, '增产剂 Mk.II') or (rows[0] if rows else None)
    check('mk2 scheme A, 30 machines, P 6', r and r[1] == '30' and r[2].startswith('A') and r[3] == 'P 6', r)
    check('mk2 min 4.5, double unit', r and r[4] == '4.5' and '双绿带' in r[5], r)
    d = page.evaluate(DETAIL_JS, '增产剂 Mk.II')
    say(case='mk2', detail=d)
    check('mk2 steady min 4.5 / 4.5', d and d['kv'].get('最低流量 steady-state min (A / B)') == '4.5 / 4.5', d and d['kv'])
    check('mk2 returns sum 25.5 > 12', d and '合计 25.5 > 12' in d['seq'], d and d['seq'][-160:])
    check('mk2 diagram drawn', d and d['svg'])
    page.screenshot(path=f'{shots}/mk2-desktop.png', full_page=True)
    # manual override: sigma 4/5 then k = 5
    sec = page.locator('.mbrecipe').first
    sec.locator('label:has-text("摇匀 σ") select').select_option('4')
    time.sleep(0.4)
    check('override keeps the plan (no re-solve)', page.query_selector('.solving') is None)
    d2 = page.evaluate(DETAIL_JS, '增产剂 Mk.II')
    check('override σ 4/5 recomputed, recommended shown', d2 and '推荐 recommended: P = 6' in d2['hint'] and d2['kv'].get('最低流量 steady-state min (A / B)') != '4.5 / 4.5', d2 and (d2['kv'].get('最低流量 steady-state min (A / B)'), d2['hint'][:200]))
    sec.locator('label:has-text("第一次补料位置 k") input').fill('5')
    time.sleep(0.4)
    d3 = page.evaluate(DETAIL_JS, '增产剂 Mk.II')
    check('override k 5 recomputed', d3 and d3['kv'] != d2['kv'], d3 and d3['kv'].get('最低流量 steady-state min (A / B)'))
    page.screenshot(path=f'{shots}/mk2-override.png', full_page=True)
    sec.locator('a:has-text("Reset")').click()
    time.sleep(0.4)
    d4 = page.evaluate(DETAIL_JS, '增产剂 Mk.II')
    check('reset back to 4.5 / 4.5', d4 and d4['kv'].get('最低流量 steady-state min (A / B)') == '4.5 / 4.5')
    # period 9 override: S = -1.5, flagged
    sec.locator('label:has-text("周期 P") select').select_option('9')
    time.sleep(0.4)
    d5 = page.evaluate(DETAIL_JS, '增产剂 Mk.II')
    check('P 9 override: S = -1.5 and a warning', d5 and d5['kv'].get('S(P) = C − P(a²+ab+b²)/(a+b)') == '-1.5' and any('m = 1.5' in n for n in d5['notes']), d5 and (d5['kv'].get('S(P) = C − P(a²+ab+b²)/(a+b)'), d5['notes'][:3]))
    sec.locator('a:has-text("Reset")').click()
    time.sleep(0.3)

    # --- carbon nanotube
    open_case(page, CASES['cnt'])
    rows = page.evaluate(ROWS_JS); say(case='cnt', rows=rows)
    r = row_for(rows, '碳纳米管')
    check('cnt scheme A, 32 machines, P 32, min 2', r and r[1] == '32' and r[3] == 'P 32' and r[4] == '2', r)
    d = page.evaluate(DETAIL_JS, '碳纳米管'); say(case='cnt', detail=d)
    check('cnt steady 2 / 2', d and d['kv'].get('最低流量 steady-state min (A / B)') == '2 / 2', d and d['kv'])
    check('cnt returns ≥ 12 → double unit', d and '双绿带' in d['kv'].get('控制单元 control unit', ''), d and d['kv'].get('回流 return (A / B) · 份额 shares'))
    page.screenshot(path=f'{shots}/cnt-desktop.png', full_page=True)
    # his own layout as an override: G first at 8, σ 4/5 → returns 8 / 6
    sec = page.locator('.mbrecipe').first
    sec.locator('label:has-text("先补 first") select').select_option('0')
    sec.locator('label:has-text("第一次补料位置 k") input').fill('8')
    sec.locator('label:has-text("摇匀 σ") select').select_option('4')
    time.sleep(0.5)
    d = page.evaluate(DETAIL_JS, '碳纳米管'); say(case='cnt his layout', kv=d and d['kv'])
    gi = 0 if '石墨烯' in (d['seq'] if d else '').split('补')[1] else 1
    check('cnt his layout: returns 8 / 6', d and d['kv'].get('回流 return (A / B) · 份额 shares', '').startswith('8 / 6' if gi == 0 else '6 / 8'), d and d['kv'].get('回流 return (A / B) · 份额 shares'))
    page.screenshot(path=f'{shots}/cnt-his-layout.png', full_page=True)

    # --- Mk.III proliferator, two rows
    open_case(page, CASES['mk3'])
    rows = page.evaluate(ROWS_JS); say(case='mk3', rows=rows)
    r = row_for(rows, '增产剂 Mk.III')
    check('mk3 P 30, min 1.5 (two rows)', r and r[3] == 'P 30' and r[4] == '1.5', r)
    d = page.evaluate(DETAIL_JS, '增产剂 Mk.III'); say(case='mk3', detail=d and d['kv'])
    page.screenshot(path=f'{shots}/mk3-desktop.png', full_page=True)

    # --- processors, two rows
    open_case(page, CASES['cpu'])
    rows = page.evaluate(ROWS_JS); say(case='cpu', rows=rows)
    r = row_for(rows, '处理器')
    check('cpu P 32 (interval 8 positions)', r and r[3] == 'P 32', r)
    d = page.evaluate(DETAIL_JS, '处理器'); say(case='cpu', detail=d and (d['kv'], d['notes']))
    page.screenshot(path=f'{shots}/cpu-desktop.png', full_page=True)

    # --- turbines
    open_case(page, CASES['turbine'])
    rows = page.evaluate(ROWS_JS); say(case='turbine', rows=rows)
    r = row_for(rows, '电磁涡轮')
    check('turbine 60 machines P 20 min 3.75', r and r[1] == '60' and r[3] == 'P 20' and r[4] == '3.75', r)
    d = page.evaluate(DETAIL_JS, '电磁涡轮'); say(case='turbine', detail=d and d['kv'])
    check('turbine returns 4.5 / 12 (or 12 / 4.5)', d and d['kv'].get('回流 return (A / B) · 份额 shares', '').split(' · ')[0] in ('4.5 / 12', '12 / 4.5'), d and d['kv'].get('回流 return (A / B) · 份额 shares'))
    txt = page.locator('.mbrecipe').first.inner_text()
    check('turbine output belt full at 16', '16 台满带' in txt)
    page.screenshot(path=f'{shots}/turbine-desktop.png', full_page=True)

    # --- electric motor (3 ingredients)
    open_case(page, CASES['motor'])
    rows = page.evaluate(ROWS_JS); say(case='motor', rows=rows)
    r = row_for(rows, '电动机')
    check('motor scheme B groups of 10', r and r[2].startswith('B') and r[3].startswith('10 ×'), r)
    page.screenshot(path=f'{shots}/motor-desktop.png', full_page=True)

    # --- settings bar: belt tier change recomputes without a re-solve
    open_case(page, CASES['mk2'])
    page.locator('.mbsettings label:has-text("带 Belt") select').first.select_option('1')
    time.sleep(0.5)
    rows = page.evaluate(ROWS_JS); say(case='mk2 green belt', rows=rows)
    check('green mixed belt changes the layout, no re-solve', page.query_selector('.solving') is None and rows and rows[0][3] != 'P 6', rows and rows[0])
    page.locator('.mbsettings label:has-text("带 Belt") select').first.select_option('2')
    time.sleep(0.3)

    # --- phone width
    ctx2 = b.new_context(viewport={'width': 390, 'height': 844}, device_scale_factor=2)
    ph = ctx2.new_page()
    ph.on('pageerror', lambda e: errors.append(str(e)[:300]))
    for name in ('mk2', 'turbine'):
        open_case(ph, CASES[name])
        dims = ph.evaluate("""() => ({ doc: document.documentElement.scrollWidth, win: innerWidth,
            belt: document.querySelector('.beltwrap')?.scrollWidth ?? 0, beltbox: document.querySelector('.beltwrap')?.clientWidth ?? 0 })""")
        say(case=f'{name} 390', dims=dims)
        check(f'{name} 390 px: page does not scroll sideways', dims['doc'] <= dims['win'] + 1, dims)
        check(f'{name} 390 px: belt diagram scrolls inside its box', dims['belt'] > dims['beltbox'] and dims['beltbox'] <= 390, dims)
        ph.locator('.mixedbelt').screenshot(path=f'{shots}/{name}-390.png')
    # dark theme
    ctx3 = b.new_context(viewport={'width': 1400, 'height': 1000}, color_scheme='dark')
    dk = ctx3.new_page()
    open_case(dk, CASES['mk2'])
    dk.screenshot(path=f'{shots}/mk2-dark.png', full_page=True)

    check('no page errors', not errors, errors[:3])
    b.close()

ok = sum(1 for r in results if r[1])
print(f'{ok}/{len(results)} checks pass')
for n, o, d in results:
    if not o:
        print('FAIL', n, d)
