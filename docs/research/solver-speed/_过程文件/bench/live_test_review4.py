"""Review-4 live re-test after the fixes. python -X utf8 live2.py <shots> [base] [only]"""
import base64, json, re, sys, time, urllib.parse
from playwright.sync_api import sync_playwright
shots = sys.argv[1]
base = sys.argv[2] if len(sys.argv) > 2 else 'http://localhost:4173/'
only = sys.argv[3].split(',') if len(sys.argv) > 3 else None
results = []
def frag(s): return base64.b64encode(urllib.parse.quote(json.dumps(s)).encode()).decode()
def url(s): return base + '#' + frag(s)
def say(**k): print(json.dumps(k, ensure_ascii=False)[:700], flush=True)
def check(name, ok, detail=''):
    results.append((name, bool(ok), detail)); say(CHECK=name, ok=bool(ok), detail=str(detail)[:300])
SF_HARD = {'game': 'satisfactory', 'objective': 'buildings', 'allow_all_alternates': True, 'integer': True, 'time_limit_ms': 4000,
    'targets': [{'item': f'Desc_SpaceElevatorPart_{i}_C', 'rate': 5} for i in (5, 6, 7, 8, 9)]
    + [{'item': 'Desc_ComputerSuper_C', 'rate': 5}, {'item': 'Desc_MotorLightweight_C', 'rate': 5}]}
SLOOP = {'game': 'satisfactory', 'objective': 'buildings', 'allow_all_alternates': True, 'integer': True, 'time_limit_ms': 4000,
    'auto_somersloops': True, 'somersloop_budget': 20,
    'targets': [{'item': 'Assembly Director System', 'rate': 4}, {'item': 'Magnetic Field Generator', 'rate': 4},
                {'item': 'Thermal Propulsion Rocket', 'rate': 2}, {'item': 'Nuclear Pasta', 'rate': 1}]}
DSP = {'game': 'dsp', 'objective': 'buildings', 'integer': True, 'time_limit_ms': 6000, 'targets': [{'item': 'universe-matrix', 'rate': 60}]}
FQ = {'game': 'factorio', 'dataset': '2x1', 'objective': 'resources', 'integer': True, 'time_limit_ms': 4000,
      'targets': [{'item': 'electronic-circuit@4', 'rate': 10}]}
def state(page):
    return page.evaluate("""() => ({
        solving: document.querySelector('.solving')?.textContent ?? null,
        card: [...document.querySelectorAll('.card .big')].map(e => e.textContent).join(' | '),
        rows: document.querySelectorAll('table tbody tr').length,
        note: document.querySelector('.note')?.textContent ?? null,
        warn: [...document.querySelectorAll('.warn')].map(e => e.textContent.slice(0,160)).join(' | ') || null,
        muted: [...document.querySelectorAll('main > p.muted')].map(e => e.textContent).join(' | ') || null,
        buttons: [...document.querySelectorAll('button')].map(b => b.textContent).filter(t => /Stop|Cancel|Keep|Solve again|Try again/.test(t)),
    })""")
def bld(s):
    m = re.match(r'\s*([\d.]+)', s['card'] or '')
    return float(m.group(1)) if m else None
def timeline(page, name, secs, stop_when_idle=True, dt=0.05, quiet=False):
    t0 = time.time(); last = None; out = []
    while time.time() - t0 < secs:
        s = state(page); t = round(time.time() - t0, 2)
        key = json.dumps(s, ensure_ascii=False)
        if key != last:
            s2 = dict(s, t=t, workers=len(page.workers)); out.append(s2)
            if not quiet: say(case=name, **s2)
            last = key
        if stop_when_idle and s['solving'] is None and (s['rows'] > 0 or s['warn']) and t > 0.6: break
        time.sleep(dt)
    return out
def want(n): return only is None or n in only
with sync_playwright() as p:
    b = p.chromium.launch()
    errors = []
    def newpage(w=1400, h=900):
        ctx = b.new_context(viewport={'width': w, 'height': h})
        page = ctx.new_page()
        page.on('console', lambda m: m.type == 'error' and (errors.append(m.text[:300]), say(console_error=m.text[:300])))
        page.on('pageerror', lambda e: (errors.append(str(e)[:300]), say(page_error=str(e)[:300])))
        return ctx, page

    if want('main'):
        # 1. Auto: first plan at once, improves, ends in the banner; Stop; Keep solving; Solve again; other tabs
        ctx, page = newpage()
        page.goto(url({**SF_HARD, 'time_limit_ms': 3000}))
        tl = timeline(page, 'main', 12, quiet=True)
        first = next((x for x in tl if x['rows'] > 0), None)
        check('main: first plan within 1 s', first and first['t'] < 1.0, first and first['t'])
        objs = [bld(x) for x in tl if x['rows'] > 0]
        check('main: plans never get worse while streaming', all(a >= c for a, c in zip(objs, objs[1:])), objs)
        fin = state(page)
        check('main: banner with Keep solving / time limit at the end', fin['note'] and 'Keep solving' in fin['note'] and 'Solve again' in fin['buttons'], fin['note'])
        page.screenshot(path=f'{shots}/main-final.png')
        before = bld(fin)
        page.get_by_role('button', name='Keep solving').first.click(); time.sleep(0.6)
        s = state(page); check('main: Keep solving resumes (status + Stop)', s['solving'] and 'Stop' in s['buttons'], s['solving'])
        page.get_by_role('button', name='Stop').click(); time.sleep(0.5)
        s = state(page); check('main: Stop keeps the plan, banner says stopped', s['rows'] > 0 and s['note'] and '(stopped)' in s['note'] and bld(s) <= before, s['note'])
        page.screenshot(path=f'{shots}/main-stopped.png')
        page.get_by_role('button', name='Keep solving').first.click()
        tl = timeline(page, 'main-keep', 10, quiet=True)
        s = state(page); check('main: Keep solving after Stop ends with a plan not worse', s['rows'] > 0 and bld(s) <= before, [before, bld(s)])
        # time limit typed into the banner: the plan in hand is not replaced by a dearer first plan
        held = bld(s)
        inp = page.locator('.note input[type=number]')
        inp.fill('6'); inp.press('Enter')
        tl = timeline(page, 'main-newlimit', 14, quiet=True)
        seen = [bld(x) for x in tl if x['rows'] > 0]
        check('main: new time limit from the banner never shows a dearer plan', seen and max(seen) <= held, [held, seen[:6]])
        check('main: new time limit is used', any(x['solving'] and '≤ 6 s' in x['solving'] for x in tl), [x['solving'] for x in tl][:4])
        check('main: status never shows the old limit after the change', not any(x['solving'] and '≤ 3 s' in x['solving'] for x in tl[1:]), [x['solving'] for x in tl][:6])
        page.screenshot(path=f'{shots}/main-newlimit-final.png')
        page.get_by_role('button', name='Solve again').click(); time.sleep(0.4)
        s = state(page); check('main: Solve again starts a solve', s['solving'] is not None, s['solving'])
        for tab in ('Item flow', 'Machine graph', 'Item values', 'Plan'):
            page.get_by_role('tab', name=tab).click(); time.sleep(0.4)
        check('main: tabs render during a solve without page errors', not errors, errors)
        ctx.close()

    if want('stale'):
        ctx, page = newpage()
        s1 = {**SF_HARD, 'time_limit_ms': 1500, 'engine': 'microlp'}
        bad = []
        for off in (0.30, 0.38, 0.42, 0.46, 0.5):
            page.goto('about:blank'); page.goto(url(s1))
            page.wait_for_function("() => /Solving 1 s/.test(document.querySelector('.solving')?.textContent ?? '')", timeout=8000)
            time.sleep(off)
            page.evaluate("h => { location.hash = h }", frag({**s1, 'time_limit_ms': 5000}))
            tl = timeline(page, f'stale{off}', 2.5, stop_when_idle=False, quiet=True)
            for x in tl[1:]:
                if x['solving'] is None or '≤ 1.5 s' in x['solving']: bad.append((off, x['t'], x['solving']))
        check('stale: after a settings change the status is always the new solve (5 offsets)', not bad, bad[:5])
        ctx.close()

    if want('highsfail'):
        for eng in ('highs', 'auto'):
            ctx, page = newpage()
            ctx.route('**/highs-*.wasm', lambda r: r.abort())
            page.goto(url({**SF_HARD, 'engine': eng}))
            tl = timeline(page, f'highsfail_{eng}', 12, quiet=True)
            fin = state(page)
            check(f'highsfail {eng}: plan and banner, HiGHS failure named', fin['rows'] > 0 and fin['note'] and 'HiGHS could not' in fin['note'], fin['note'])
            if eng == 'auto':
                check('highsfail auto: plan as good as microlp alone (716)', bld(fin) <= 716, bld(fin))
            page.screenshot(path=f'{shots}/highsfail-{eng}.png'); ctx.close()

    if want('highs'):
        # HiGHS only: stop while HiGHS blocks its worker -> worker ended, plan kept; Keep solving restarts and is never worse
        ctx, page = newpage()
        page.goto(url({**SF_HARD, 'engine': 'highs', 'time_limit_ms': 8000}))
        page.wait_for_selector("table tbody tr", timeout=15000); time.sleep(5.0)
        s = state(page); before = bld(s)
        page.get_by_role('button', name='Stop').click(); time.sleep(0.4)
        s = state(page)
        check('highs: Stop during HiGHS keeps the plan', s['rows'] > 0 and '(stopped)' in (s['note'] or ''), s['note'])
        page.get_by_role('button', name='Keep solving').first.click()
        tl = timeline(page, 'highs-keep', 14, quiet=True)
        seen = [bld(x) for x in tl if x['rows'] > 0]
        check('highs: Keep solving (restart) never shows a dearer plan', seen and max(seen) <= before, [before, seen[:6]])
        ctx.close()

    if want('quality'):
        ctx, page = newpage()
        page.goto(url(FQ))
        mx = 0; t0 = time.time()
        while time.time() - t0 < 6:
            mx = max(mx, len(page.workers)); time.sleep(0.02)
            s = state(page)
            if s['solving'] is None and s['rows'] > 0 and time.time() - t0 > 1: break
        s = state(page)
        check('quality (Auto): one worker, plan shown', mx == 1 and s['rows'] > 0, [mx, s['card']]); ctx.close()

    if want('proven'):
        for name, st in (('dsp', DSP), ('sloop', SLOOP)):
            ctx, page = newpage()
            page.goto(url(st))
            tl = timeline(page, name, 14, quiet=True)
            fin = state(page)
            say(case=name, **fin)
            if name == 'dsp':
                check('dsp Buildings (Auto): proven, no banner, card says optimal', fin['rows'] > 0 and fin['note'] is None and 'gap' not in fin['card'], fin['card'])
            else:
                check('somersloop budget (Auto): a plan, within budget banner or proven', fin['rows'] > 0, fin['card'])
            page.screenshot(path=f'{shots}/{name}-final.png'); ctx.close()

    if want('phone'):
        for w in (390, 1024):
            ctx, page = newpage(w, 844)
            page.goto(url({**SF_HARD, 'time_limit_ms': 3000}))
            page.wait_for_selector('table tbody tr', timeout=15000); time.sleep(1.2)
            page.evaluate("() => document.querySelector('.tabs').scrollIntoView()")
            page.screenshot(path=f'{shots}/w{w}-solving.png')
            m = page.evaluate("""() => { const t = document.querySelector('.tabs'), s = document.querySelector('.solving'), b = document.querySelector('.tabbtn');
                const r = e => e ? e.getBoundingClientRect() : null; const sr = r(s), br = r(b);
                return { tabs_h: r(t).height, status_h: sr?.height, status_w: sr?.width, stop_visible: br ? br.right <= innerWidth && br.left >= 0 : null, overflow: document.documentElement.scrollWidth - innerWidth } }""")
            check(f'width {w}: status line at most 2 lines, Stop on screen, no page overflow', m['status_h'] is not None and m['status_h'] <= 48 and m['stop_visible'] and m['overflow'] <= 0, m)
            timeline(page, f'w{w}', 10, quiet=True)
            page.evaluate("() => { const n = document.querySelector('.note'); scrollTo(0, n.getBoundingClientRect().top + scrollY - 120) }")
            page.screenshot(path=f'{shots}/w{w}-final.png')
            m = page.evaluate("""() => { const n = document.querySelector('.note'); const r = n.getBoundingClientRect();
                return { note_right: r.right, inner: innerWidth, overflow: document.documentElement.scrollWidth - innerWidth, btns: [...n.querySelectorAll('button, input')].every(e => e.getBoundingClientRect().right <= innerWidth) } }""")
            check(f'width {w}: banner and its controls fit', m['btns'] and m['overflow'] <= 0, m)
            ctx.close()
    b.close()
print('\n== summary ==')
for n, ok, d in results: print(('PASS ' if ok else 'FAIL ') + n + ('' if ok else f'   <- {d}'))
