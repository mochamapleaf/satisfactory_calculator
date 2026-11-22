"""Review 6 live test of the Map tab. python -X utf8 live_test_review6.py <shots dir> [base url] [only,cases]
Needs `vite preview --port 4174` of the built worktree. Prints one JSON line per state change and a CHECK line per check."""
import base64, json, os, re, sys, time, urllib.parse
from playwright.sync_api import sync_playwright
shots = sys.argv[1]
base = sys.argv[2] if len(sys.argv) > 2 else 'http://localhost:4174/'
only = sys.argv[3].split(',') if len(sys.argv) > 3 else None
os.makedirs(shots, exist_ok=True)
results = []
def frag(s): return base64.b64encode(urllib.parse.quote(json.dumps(s)).encode()).decode()
def say(**k): print(json.dumps(k, ensure_ascii=False)[:900], flush=True)
def check(name, ok, detail=''):
    results.append((name, bool(ok))); say(CHECK=name, ok=bool(ok), detail=str(detail)[:500])
def want(n): return only is None or n in only
PLATE = {'game': 'satisfactory', 'targets': [{'item': 'Desc_IronPlate_C', 'rate': 12000}], 'ignore_limits': True}
PLATE600 = {'game': 'satisfactory', 'targets': [{'item': 'Desc_IronPlate_C', 'rate': 600}]}
HUGE = {'game': 'satisfactory', 'targets': [{'item': 'Desc_IronPlate_C', 'rate': 2000000}], 'ignore_limits': True}
LATE = {'game': 'satisfactory', 'objective': 'buildings', 'allow_all_alternates': True,
        'targets': [{'item': f'Desc_SpaceElevatorPart_{i}_C', 'rate': 5} for i in (5, 6, 7, 8, 9)]
        + [{'item': 'Desc_ComputerSuper_C', 'rate': 5}, {'item': 'Desc_MotorLightweight_C', 'rate': 5}]}
SUP = {'game': 'satisfactory', 'targets': [{'item': 'Desc_Motor_C', 'rate': 10}], 'supplies': [{'item': 'Desc_Rotor_C', 'rate': 100}]}
def gstate(page):
    return page.evaluate("""() => ({
      solving: document.querySelector('.geo p.solving')?.textContent ?? null,
      cards: [...document.querySelectorAll('.geosum .card')].map(e => e.textContent).join(' | '),
      sum: document.querySelector('.geosum p.small')?.textContent ?? null,
      parts: document.querySelector('.geosum table')?.textContent ?? null,
      warn: [...document.querySelectorAll('.geo .warn')].map(e => e.textContent.slice(0, 400)).join(' | ') || null,
      muted: [...document.querySelectorAll('.geo p.muted')].map(e => e.textContent).join(' | ') || null,
      note: [...document.querySelectorAll('.geo .note')].map(e => e.textContent).join(' | ') || null,
      sens: [...document.querySelectorAll('.geoside table')].find(t => /Hub cost F/.test(t.textContent))?.textContent ?? null,
      hints: [...document.querySelectorAll('.geoside > p.hint')].map(e => e.textContent).join(' | '),
      mined: document.querySelector('.geosum p.hint')?.textContent ?? null,
      legend: document.querySelector('.geolegend')?.textContent ?? null,
      btn: [...document.querySelectorAll('.geoctl button')].map(b => b.textContent).join(','),
      dots: document.querySelectorAll('.sweep .pt').length,
      overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    })""")
def open_map(page, s, basesite=None, **sel):
    page.goto(base + '#' + frag(s)); page.reload()
    page.get_by_role('tab', name='Map').click(timeout=60000)
    page.wait_for_selector('.geoctl')
    if basesite: page.select_option('select[aria-label="Base site"]', basesite)
    for k, v in sel.items(): page.select_option(f'select[aria-label="{k}"]', str(v))
def wait_done(page, name, secs):
    t0 = time.time(); last = None; seen = []
    while time.time() - t0 < secs:
        s = gstate(page)
        if s['solving']: seen.append(s['solving'])
        key = json.dumps([s['cards'], s['warn'], s['muted'], s['note'], s['sens'], bool(s['solving']), (s['solving'] or '').split(':')[0]])
        if key != last:
            say(case=name, t=round(time.time() - t0, 1), solving=s['solving'], cards=s['cards'], warn=s['warn'], note=s['note'], sens=s['sens']); last = key
        if s['solving'] is None and time.time() - t0 > 1: return s, seen, time.time() - t0
        time.sleep(0.4)
    say(case=name, TIMEOUT=secs); return gstate(page), seen, secs
with sync_playwright() as p:
    b = p.chromium.launch()
    errors = []
    def newpage(w=1400, h=900, dark=False):
        ctx = b.new_context(viewport={'width': w, 'height': h}, color_scheme='dark' if dark else 'light')
        page = ctx.new_page()
        # count geo workers made / terminated by the page (Playwright's own worker list lags behind a busy worker)
        page.add_init_script("""(() => { const W = window.Worker; window.__geo = { made: 0, term: 0 };
          window.Worker = class extends W { constructor(u, o) { super(u, o); this.__g = /geoWorker/.test(String(u)); if (this.__g) window.__geo.made++ }
            terminate() { if (this.__g && !this.__t) { this.__t = true; window.__geo.term++ } super.terminate() } } })()""")
        page.on('console', lambda m: m.type == 'error' and (errors.append(m.text[:300]), say(console_error=m.text[:300])))
        page.on('pageerror', lambda e: (errors.append(str(e)[:300]), say(page_error=str(e)[:300])))
        return ctx, page

    if want('plate'):
        ctx, page = newpage(); open_map(page, PLATE, 'Sm3_0')
        page.click('.geoctl button.primary')
        s, seen, dt = wait_done(page, 'plate', 240)
        check('plate: 95.1k OE, 2 hubs, 5 cars, 3.5 km rail', '95.1k OE' in s['cards'] and s['cards'].startswith('2train hubs') and '5 cars' in s['cards'] and '3.5 km' in s['cards'], s['cards'])
        check('plate: rail saves 4.3 % against 99386, hubs Sm1_2 + Sm3_0, proven', all(x in (s['sum'] or '') for x in ('proven optimal', 'Rail saves 4.3 %', '99386 OE)', 'Sm1_2, Sm3_0', '5760/min by rail')), s['sum'])
        check('plate: cost parts keep their zeros (Total 95124, Machines 24600)', 'Total95124' in (s['parts'] or '') and 'Machines24600' in (s['parts'] or ''), s['parts'])
        check('plate: p* 2 / 2 / 0 at F 0.5 / 1 / 2x, all proven', re.search(r'0\.5× \(1745 OE\)show2proven1× \(3490 OE\)2proven2× \(6980 OE\)show0proven', s['sens'] or ''), s['sens'])
        secs = [m.group(1) for t in seen for m in [re.search(r': (\S*) s of', t)] if m]
        check('plate: elapsed seconds always a number, no stripped zeros', secs and all(re.fullmatch(r'\d+', x) for x in secs), sorted(set(secs))[:20])
        check('plate: legend names candidate hub and base', 'candidate hub' in s['legend'] and 'base' in s['legend'], s['legend'])
        page.screenshot(path=f'{shots}/plate12k-desktop-light.png', full_page=True)
        # a site: detail and "make this the base"
        page.locator('.worldmap g.site').nth(0).dispatch_event('click')
        check('plate: tapping a site shows its detail', page.locator('.geosite h2').count() == 1, page.locator('.geosite').text_content()[:120] if page.locator('.geosite').count() else '')
        ctx.close()

    if want('tabswitch'):
        ctx, page = newpage(); open_map(page, PLATE, 'Sm3_0')
        page.click('.geoctl button.primary'); time.sleep(4)
        s0 = gstate(page)
        page.get_by_role('tab', name='Plan').click(); time.sleep(0.5)
        page.get_by_role('tab', name='Map').click(); time.sleep(1.5)
        s1 = gstate(page)
        g = page.evaluate('window.__geo')
        check('tab switch: was solving, its worker terminated on leaving', s0['solving'] and g == {'made': 1, 'term': 1}, [s0['solving'], g])
        check('tab switch: back on the tab it says stopped, not solving; points kept; can solve again', s1['solving'] is None and 'Stopped' in (s1['muted'] or '') and s1['dots'] >= 1 and 'Solve again' in s1['btn'], [s1['solving'], s1['muted'], s1['dots'], s1['btn']])
        page.click('.geoctl button.primary')
        s2, _, _ = wait_done(page, 'tabswitch-again', 240)
        check('tab switch: solve again completes (95.1k OE)', '95.1k OE' in s2['cards'] and not s2['muted'], s2['cards'])
        # Stop button
        page.click('.geoctl button.primary'); time.sleep(3); page.get_by_role('button', name='Stop').click(); time.sleep(0.5)
        s3 = gstate(page)
        g = page.evaluate('window.__geo')
        check('stop: solving ends, every geo worker terminated, message shown', s3['solving'] is None and 'Stopped' in (s3['muted'] or '') and g['made'] == g['term'] == 3, [s3['muted'], g])
        ctx.close()

    if want('supplied'):
        ctx, page = newpage(); open_map(page, SUP, 'S0_m1', **{'Time per solve': 10})
        page.click('.geoctl button.primary')
        s, _, _ = wait_done(page, 'supplied', 200)
        check('supplied: plan with a supplied Rotor is sited (was "aggregate LP: infeasible")', 'OE' in s['cards'] and not s['warn'] and 'Rotor' in (s['note'] or ''), [s['cards'], s['warn'], s['note']])
        ctx.close()

    if want('norail'):
        ctx, page = newpage(); open_map(page, PLATE600, 'S0_m1', Tier=4)
        page.click('.geoctl button.primary')
        s, seen, dt = wait_done(page, 'norail', 120)
        check('tier 4: one point, no hub table, says trains are not unlocked', s['dots'] == 1 and s['sens'] is None and 'not unlocked at tier 4' in s['hints'] and 'OE' in s['cards'], [s['dots'], s['hints'][:120], dt])
        ctx.close()

    if want('short'):
        ctx, page = newpage(); open_map(page, HUGE, 'S0_m1')
        page.click('.geoctl button.primary')
        s, _, dt = wait_done(page, 'short', 120)
        check('2 000 000 plate: stops with the iron shortfall', re.search(r'cannot supply this plan at tier 8: Iron Ore 3000000/min needed', s['warn'] or '') and dt < 30, [s['warn'], dt])
        ctx.close()

    if want('late'):
        ctx, page = newpage(390, 844, True); open_map(page, LATE, 'S0_m1')
        # let the planner finish first, so the map plans the final plan
        page.wait_for_function("!document.querySelector('.tabs .solving')", timeout=120000)
        page.click('.geoctl button.primary')
        s, seen, dt = wait_done(page, 'late', 600)
        check('late game (7 targets, all alternates), 390 px dark: a plan with cost and links, no error', 'k OE' in s['cards'] and 'Model:' in s['hints'] and 'error' not in (s['warn'] or '').lower(), [s['cards'], s['sum'], s['warn'], round(dt)])
        check('late game: no horizontal overflow at 390 px', s['overflow'] == 0, s['overflow'])
        check('late game: unsolved or time-limited points are stated', ('time limit' in (s['sum'] or '')) or ('proven optimal' in (s['sum'] or '')), s['sum'])
        check('late game: a plan without hubs claims no rail saving', not (s['cards'].startswith('0train') and 'Rail saves' in (s['sum'] or '')), s['sum'])
        say(late_model=s['hints'][-260:], mined=s['mined'])
        page.screenshot(path=f'{shots}/late-390-dark.png', full_page=True)
        page.locator('.worldmap').screenshot(path=f'{shots}/late-390-dark-map.png')
        ctx.close()
        ctx, page = newpage(390, 844, True); open_map(page, PLATE, 'Sm3_0')
        page.click('.geoctl button.primary'); s, _, _ = wait_done(page, 'plate390', 240)
        check('plate at 390 px dark: same result, no overflow', '95.1k OE' in s['cards'] and s['overflow'] == 0, [s['cards'], s['overflow']])
        page.locator('.geomap').screenshot(path=f'{shots}/plate12k-390-dark-map.png')
        ctx.close()

    check('no console or page errors', not errors, errors[:5])
    b.close()
bad = [n for n, ok in results if not ok]
say(SUMMARY=f'{len(results) - len(bad)}/{len(results)} pass', failed=bad)
