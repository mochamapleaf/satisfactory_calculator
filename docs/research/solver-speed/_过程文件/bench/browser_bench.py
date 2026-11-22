"""Browser benchmark of the whole-building engines (Chromium, real worker + WASM).
Needs `npx vite --port 5174` running in react-frontend (serves the dev-only bench.html).
python -X utf8 browser_bench.py OUT.jsonl ENGINE [ms=4000] [filter]
Prints one line per scenario (progress for the watchdog).
"""
import json, sys, time
from playwright.sync_api import sync_playwright

out, engine = sys.argv[1], sys.argv[2]
ms = int(sys.argv[3]) if len(sys.argv) > 3 else 4000
filt = sys.argv[4] if len(sys.argv) > 4 else ''
with sync_playwright() as p, open(out, 'w', encoding='utf-8') as f:
    b = p.chromium.launch()
    page = b.new_page()

    def on_console(m):
        if m.text.startswith('BENCH '):
            f.write(m.text[6:] + '\n')
            f.flush()
            print(m.text[6:], flush=True)
        elif m.type == 'error':
            print('console error:', m.text, flush=True)

    page.on('console', on_console)
    page.goto('http://localhost:5174/bench.html')
    page.wait_for_function("document.getElementById('out').textContent === 'ready'", timeout=60000)
    t0 = time.time()
    page.evaluate(f"window.runBench({json.dumps(engine)}, {ms}, {json.dumps(filt)})")
    print(f'[done] {time.time() - t0:.0f} s', flush=True)
    b.close()
