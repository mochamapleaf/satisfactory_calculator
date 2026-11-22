"""Live browser check of the streamed whole-building solve (vite preview on :4173).
python -X utf8 live_test.py <shots dir> [base url]
Prints a JSON log of what was seen; screenshots into <shots dir>.
"""
import base64, json, sys, time, urllib.parse
from playwright.sync_api import sync_playwright

shots = sys.argv[1]
base = sys.argv[2] if len(sys.argv) > 2 else 'http://localhost:4173/'
log = []


def url(settings):
    return base + '#' + base64.b64encode(urllib.parse.quote(json.dumps(settings)).encode()).decode()


def say(**k):
    log.append(k)
    print(json.dumps(k, ensure_ascii=False), flush=True)


SF_HARD = {
    'game': 'satisfactory', 'objective': 'buildings', 'allow_all_alternates': True, 'integer': True, 'time_limit_ms': 4000,
    'targets': [{'item': f'Desc_SpaceElevatorPart_{i}_C', 'rate': 5} for i in (5, 6, 7, 8, 9)]
    + [{'item': 'Desc_ComputerSuper_C', 'rate': 5}, {'item': 'Desc_MotorLightweight_C', 'rate': 5}],
}
SLOOP = {
    'game': 'satisfactory', 'objective': 'buildings', 'allow_all_alternates': True, 'integer': True, 'time_limit_ms': 4000,
    'auto_somersloops': True, 'somersloop_budget': 20,
    'targets': [{'item': 'Assembly Director System', 'rate': 4}, {'item': 'Magnetic Field Generator', 'rate': 4},
                {'item': 'Thermal Propulsion Rocket', 'rate': 2}, {'item': 'Nuclear Pasta', 'rate': 1}],
}
DSP = {'game': 'dsp', 'objective': 'compute', 'integer': True, 'time_limit_ms': 4000, 'targets': [{'item': 'universe-matrix', 'rate': 60}]}


def state(page):
    return page.evaluate("""() => ({
        solving: document.querySelector('.solving')?.textContent ?? null,
        card: [...document.querySelectorAll('.card .big')].map(e => e.textContent).join(' | '),
        rows: document.querySelectorAll('table tbody tr').length,
        note: document.querySelector('.note')?.textContent ?? null,
        warn: [...document.querySelectorAll('.warn')].map(e => e.textContent).join(' | ') || null,
        buttons: [...document.querySelectorAll('button')].map(b => b.textContent).filter(t => /Stop|Cancel|Keep|Solve again/.test(t)),
    })""")


def watch(page, name, secs, shot_at=()):
    """Poll the page from navigation on; screenshot at the given seconds."""
    t0 = time.time()
    first = None
    shot_at = list(shot_at)
    last = None
    while time.time() - t0 < secs:
        s = state(page)
        t = time.time() - t0
        if first is None and s['rows'] > 0:
            first = t
            say(case=name, event='first plan visible', t=round(t, 2), **s)
        key = (s['solving'], s['card'], s['note'])
        if key != last:
            say(case=name, t=round(t, 2), **s)
            last = key
        if shot_at and t >= shot_at[0]:
            page.screenshot(path=f'{shots}/{name}-{shot_at[0]:.1f}s.png', full_page=False)
            shot_at.pop(0)
        if s['solving'] is None and s['rows'] > 0 and not shot_at:
            break
        time.sleep(0.1)
    return first


with sync_playwright() as p:
    b = p.chromium.launch()
    page = b.new_page(viewport={'width': 1400, 'height': 900})
    page.on('console', lambda m: m.type == 'error' and say(console_error=m.text))

    # 1. hard plan: first plan at once, improves while solving, final note
    page.goto(url(SF_HARD))
    watch(page, 'sf_hard', 12, shot_at=(1.0, 2.5))
    page.screenshot(path=f'{shots}/sf_hard-final.png')
    say(case='sf_hard', event='final', **state(page))

    # 2. stop keeps the best plan; keep solving resumes
    page.goto('about:blank')
    page.goto(url({**SF_HARD, 'time_limit_ms': 10000}))
    page.wait_for_selector('table tbody tr', timeout=10000)
    time.sleep(1.5)
    say(case='stop', event='before stop', **state(page))
    page.get_by_role('button', name='Stop').click()
    time.sleep(0.5)
    s = state(page)
    say(case='stop', event='after stop', **s)
    page.screenshot(path=f'{shots}/stop-kept-plan.png')
    page.get_by_role('button', name='Keep solving').first.click()
    time.sleep(1.0)
    say(case='stop', event='keep solving 1 s', **state(page))
    page.screenshot(path=f'{shots}/keep-solving.png')
    watch(page, 'keep', 14)
    say(case='stop', event='keep solving final', **state(page))
    page.screenshot(path=f'{shots}/keep-solving-final.png')

    # 3. Somersloop budget returns a plan
    page.goto('about:blank')
    page.goto(url(SLOOP))
    watch(page, 'sloop', 12)
    page.screenshot(path=f'{shots}/somersloop-budget.png')
    say(case='sloop', event='final', **state(page),
        sloops=page.evaluate("() => [...document.querySelectorAll('.card')].map(c => c.textContent).filter(t => /Somersloop/i.test(t))"))

    # 4. DSP compute objective
    page.goto('about:blank')
    page.goto(url(DSP))
    watch(page, 'dsp_compute', 12)
    page.screenshot(path=f'{shots}/dsp-compute.png')
    say(case='dsp_compute', event='final', **state(page))
    b.close()
json.dump(log, open(f'{shots}/live_log.json', 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
