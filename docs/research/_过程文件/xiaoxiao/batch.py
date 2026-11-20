"""Transcribe 笑笑meow's DSP + Factorio collections from audio/<BV>.m4a (see fetch_audio.py), resumable.
Run: G:/saavy-gpu-eval/env/python.exe -u batch.py   (from this folder)
Waits (with [hb] heartbeats) while another job holds the shared GPU lock; once the GPU is free each
video runs as a subprocess with a 30-min timeout (stall check). Progress -> batch.log."""
import os, subprocess, sys, time
from pathlib import Path
HERE = Path(__file__).parent
TOOLS = "C:/Users/Xinyu/projects/_review/decision-desk/tools"
LOCK = Path(os.environ.get("TEMP", "C:/Temp")) / "post_fetch_gpu.lock"
rows = [l.split("\t") for l in (HERE / "list.tsv").read_text(encoding="utf-8").splitlines() if l.strip()]
log = lambda m: print(f"[{time.strftime('%H:%M:%S')}] {m}", flush=True)
fails = {}
while True:
    todo = [(i, g, bv, t) for i, (g, bv, t) in enumerate(rows, 1)
            if not (HERE / g / bv / "transcript.txt").exists() and fails.get(bv, 0) < 2]
    ready = [x for x in todo if (HERE / "audio" / f"{x[2]}.m4a").exists()
             and (HERE / "audio" / f"{x[2]}.m4a").stat().st_size > 100000]
    if not todo:
        break
    if not ready:
        log(f"[hb] {len(todo)} left, no audio yet; waiting"); time.sleep(300); continue
    i, game, bv, title = ready[0]
    last = 0
    while LOCK.exists():  # someone else's GPU job
        if time.time() - last > 300:
            log(f"[hb] GPU busy ({LOCK.read_text(errors='replace')[:40] if LOCK.exists() else ''}); {len(todo)} left")
            last = time.time()
        time.sleep(15)
    out = HERE / game / bv
    out.mkdir(parents=True, exist_ok=True)
    (out / "title.txt").write_text(title, encoding="utf-8")
    log(f"{i}/{len(rows)} {game} {bv}")
    t0 = time.time()
    code = (f"import sys; sys.path.insert(0, r'{TOOLS}'); from pathlib import Path; import post_fetch; "
            f"post_fetch.transcribe(Path(r'{HERE / 'audio' / (bv + '.m4a')}'), Path(r'{out}'))")
    try:
        r = subprocess.run([sys.executable, "-c", code], timeout=1800, capture_output=True, text=True,
                           encoding="utf-8", errors="replace")
        ok = (out / "transcript.txt").exists()
        log(f"   {'OK' if ok else 'FAIL'} {time.time()-t0:.0f}s {'' if ok else r.stderr[-300:]}")
    except subprocess.TimeoutExpired:
        ok = False
        log(f"   TIMEOUT 30 min (stalled) {bv}")
    if not ok:
        fails[bv] = fails.get(bv, 0) + 1
log("DONE")
