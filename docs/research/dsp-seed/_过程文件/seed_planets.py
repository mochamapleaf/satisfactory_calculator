"""List every planet and its resources for one DSP galaxy seed (prototype).

Uses the native generator of DoubleUTH/DSP-Seed-Finder (Rust, Apache-2.0,
https://github.com/DoubleUTH/DSP-Seed-Finder). Build it once:

    git clone --depth 1 https://github.com/DoubleUTH/DSP-Seed-Finder
    cd DSP-Seed-Finder && cargo build --release --bin dsp_seed

Then:

    python seed_planets.py --bin <path>/target/release/dsp_seed(.exe) \
        --seed 12345678 --stars 64 --mult 1.0 [--actual] [--json out.json]

The binary is a websocket server on 127.0.0.1:62879 (its "native mode"); this
script starts it, sends one {"type":"Generate"} message, prints a table and
optionally saves the raw galaxy JSON. Only the Python standard library is used.

Amounts: ore veins in units of ore; oil (Oil) amounts are seep rates in the
generator's internal unit, x4e-5 = oil/s per seep (see vein.rs comment
"times 4e-5 for oil"). Gas giants: "gases" = [(item_id, rate), ...], the
per-collector base rate before tech multipliers. Estimated veins (default) give
min/max ranges; --actual runs the full terrain+vein generation (slow, exact
w.r.t. the reimplementation, not verified against the game by us).
"""
import argparse, base64, json, os, socket, struct, subprocess, sys, time

PORT = 62879
VEIN_NAMES = {
    "Iron": "铁", "Copper": "铜", "Silicium": "硅", "Titanium": "钛", "Stone": "石",
    "Coal": "煤", "Oil": "原油涌泉", "Fireice": "可燃冰", "Diamond": "金伯利",
    "Fractal": "分形硅", "Crysrub": "有机晶体", "Grat": "光栅石", "Bamboo": "刺笋",
    "Mag": "单极磁石",
}
GAS_NAMES = {1120: "氢", 1121: "重氢", 1011: "可燃冰"}


def ws_connect(host, port):
    s = socket.create_connection((host, port), timeout=600)
    key = base64.b64encode(os.urandom(16)).decode()
    s.sendall((f"GET / HTTP/1.1\r\nHost: {host}:{port}\r\nUpgrade: websocket\r\n"
               f"Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\n"
               "Sec-WebSocket-Version: 13\r\n\r\n").encode())
    buf = b""
    while b"\r\n\r\n" not in buf:
        buf += s.recv(4096)
    if b" 101 " not in buf.split(b"\r\n")[0]:
        raise RuntimeError("handshake failed: " + buf[:200].decode(errors="replace"))
    return s, buf.split(b"\r\n\r\n", 1)[1]


def ws_send_text(s, text):
    data = text.encode()
    mask = os.urandom(4)
    n = len(data)
    head = bytes([0x81])
    if n < 126:
        head += bytes([0x80 | n])
    elif n < 65536:
        head += bytes([0x80 | 126]) + struct.pack(">H", n)
    else:
        head += bytes([0x80 | 127]) + struct.pack(">Q", n)
    s.sendall(head + mask + bytes(b ^ mask[i % 4] for i, b in enumerate(data)))


def ws_recv_text(s, pending=b""):
    buf = pending
    def need(k):
        nonlocal buf
        while len(buf) < k:
            chunk = s.recv(1 << 20)
            if not chunk:
                raise RuntimeError("connection closed")
            buf += chunk
    out = b""
    while True:
        need(2)
        fin, op, ln = buf[0] & 0x80, buf[0] & 0x0F, buf[1] & 0x7F
        off = 2
        if ln == 126:
            need(4); ln = struct.unpack(">H", buf[2:4])[0]; off = 4
        elif ln == 127:
            need(10); ln = struct.unpack(">Q", buf[2:10])[0]; off = 10
        need(off + ln)
        payload, buf = buf[off:off + ln], buf[off + ln:]
        if op in (0x1, 0x0):
            out += payload
            if fin:
                return out.decode()


def generate(binary, seed, stars, mult, actual):
    proc = subprocess.Popen([binary], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    try:
        deadline = time.time() + 30
        while True:  # wait until the server listens
            try:
                s, pending = ws_connect("127.0.0.1", PORT)
                break
            except OSError:
                if time.time() > deadline or proc.poll() is not None:
                    raise RuntimeError("dsp_seed did not start")
                time.sleep(0.3)
        msg = {"type": "Generate", "seed": seed,
               "game": {"starCount": stars, "resourceMultiplier": mult,
                        "useActualVeins": actual}}
        ws_send_text(s, json.dumps(msg))
        resp = json.loads(ws_recv_text(s, pending))
        s.close()
        return resp["galaxy"]
    finally:
        proc.kill()


def fmt_veins(p):
    parts = []
    if "actualVeins" in p:
        for v in p["actualVeins"]:
            t, a = v["veinType"], v["amount"]
            if t == "Oil":
                parts.append(f"{VEIN_NAMES[t]} {a*4e-5:.2f}/s")
            else:
                parts.append(f"{VEIN_NAMES.get(t, t)} {a}")
    else:
        for v in p.get("veins", []):
            # estimated mode: group = clusters, patch = nodes per cluster,
            # amount = ore per node; upper bound of the planet total =
            # maxGroup*maxPatch*maxAmount, midpoint shown (actual ~0.8x, 存疑)
            t = v["veinType"]
            mid = lambda a, b: (a + b) / 2
            g, n, a = (mid(v["minGroup"], v["maxGroup"]), mid(v["minPatch"], v["maxPatch"]),
                       mid(v["minAmount"], v["maxAmount"]))
            if t == "Oil":
                parts.append(f"{VEIN_NAMES[t]} ~{g:.0f}泉 x {a*4e-5:.2f}/s")
            else:
                parts.append(f"{VEIN_NAMES.get(t, t)} ~{g*n*a/1e3:.0f}k")
    return ", ".join(parts)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--stars", type=int, default=64)
    ap.add_argument("--mult", type=float, default=1.0, help="resource multiplier (0.1=稀有)")
    ap.add_argument("--actual", action="store_true", help="exact veins (slow)")
    ap.add_argument("--json", help="save raw galaxy JSON here")
    a = ap.parse_args()
    sys.stdout.reconfigure(encoding="utf-8")
    t0 = time.time()
    g = generate(a.bin, a.seed, a.stars, a.mult, a.actual)
    if a.json:
        with open(a.json, "w", encoding="utf-8") as f:
            json.dump(g, f, ensure_ascii=False)
    for st in g["stars"]:
        s = st
        x, y, z = s["position"]
        print(f"\n★{s['index']} {s['name']} {s['type']} {s['spectr']} lum={s['luminosity']:.3f} "
              f"pos=({x:.1f},{y:.1f},{z:.1f})ly dyson_r={s['dysonRadius']} "
              f"hives={s['initialHiveCount']}/{s['maxHiveCount']}")
        for p in st["planets"]:
            th = p["theme"]
            name = th.get("name") or th.get("displayName") or th.get("id")
            sea = {1000: " 水", 1116: " 硫酸"}.get(th.get("waterItemId"), "")
            gas = ", ".join(f"{GAS_NAMES.get(i, i)} {r:.3f}" for i, r in p.get("gases", []))
            print(f"  P{p['index']} {p['type']:<8} {name!s:<12}{sea} "
                  + (f"gas[{gas}]" if gas else fmt_veins(p)))
    if a.actual:  # galaxy totals per resource (ore units; oil in /s)
        tot = {}
        for st in g["stars"]:
            for p in st["planets"]:
                for v in p.get("actualVeins", []):
                    tot[v["veinType"]] = tot.get(v["veinType"], 0) + v["amount"]
        print("\n# galaxy totals: " + ", ".join(
            f"{VEIN_NAMES.get(k, k)} {v*4e-5:.1f}/s" if k == "Oil"
            else f"{VEIN_NAMES.get(k, k)} {v/1e6:.2f}M" for k, v in tot.items()))
    print(f"\n# {len(g['stars'])} stars, generated in {time.time()-t0:.1f}s", file=sys.stderr)


if __name__ == "__main__":
    main()
