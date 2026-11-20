"""Download the audio of every video in urls.txt (signed Akamai links from the signed-in page; expire ~2 h)."""
import subprocess, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
HERE = Path(__file__).parent
AB = {"ee": "e", "md": "mid", "de": "deadline", "oi": "oi", "pm": "platform", "gn": "gen", "td": "trid",
      "ns": "nbs", "uk": "uipk", "os": "os", "og": "og"}
lines = (HERE / "urls.txt").read_text().splitlines()
E, OI = lines[0][2:], lines[1][3:]
game = {l.split("\t")[1]: l.split("\t")[0] for l in (HERE / "list.tsv").read_text(encoding="utf-8").splitlines()}
def url(row):
    bv, path, dl, trid, sig, hmac, order, og = row.split()
    val = {"e": E, "mid": "0", "deadline": dl, "oi": OI, "platform": "pc", "gen": "playurlv3", "trid": trid,
           "nbs": "1", "uipk": "5", "os": "akam", "og": og}
    keys = [AB[order[i:i + 2]] for i in range(0, len(order), 2)]
    q = "&".join(f"{k}={val[k]}" for k in keys)
    return bv, (f"https://upos-hz-mirrorakam.akamaized.net/upgcxcode/{path}?{q}&upsig={sig}"
                f"&uparams={','.join(keys)}&hdnts=exp={dl}~hmac={hmac}")
def get(row):
    bv, u = url(row)
    out = HERE / "audio" / f"{bv}.m4a"
    if out.exists() and out.stat().st_size > 100000:
        return
    out.parent.mkdir(exist_ok=True)
    for attempt in range(3):
        r = subprocess.run(["curl", "-s", "-o", str(out), "-w", "%{http_code}", "--max-time", "900",
                            "-H", "Referer: https://www.bilibili.com/", "-A", "Mozilla/5.0", u],
                           capture_output=True, text=True)
        if r.stdout == "200" and out.stat().st_size > 100000:
            print(f"[{time.strftime('%H:%M:%S')}] OK {bv} {out.stat().st_size//1024} KB", flush=True); return
    print(f"[{time.strftime('%H:%M:%S')}] FAIL {bv} http={r.stdout}", flush=True)
with ThreadPoolExecutor(1) as ex:
    list(ex.map(get, lines[2:]))
print("DOWNLOADS DONE", flush=True)
