"""Build local item/building icon sprite sheets for the planner, in two styles.

Usage: python tools/build_icons.py [satisfactory|dsp|factorio ...]
Writes react-frontend/public/icons/<game>-original.webp, <game>-flat.webp and <game>.json
({"size": 64, "cols": N, "index": {id: cell}}). Downloads are cached in data/raw/icons/ (gitignored).

Sources (icons are the games' own art, as used by these MIT fan tools):
- Satisfactory: greeny/SatisfactoryTools www/assets/images/items/<slug>_64.png @2192bd15
- DSP, Factorio: factoriolab/factoriolab public/data/<dsp|2.0>/icons.webp @a61e43fd (64 px cells)

Styles:
- original: the game icon, transparent background, 64 px.
- flat: a minimal 2D redraw (tools/flat2d.py): smooth silhouette, 3-4 flat colour regions with the
  shading removed, thin outline and region lines.
- mono: the same drawing in one flat tint per icon; detail only as lines.
"""
import io
import json
import sys
import urllib.request
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter, ImageEnhance

sys.path.insert(0, str(Path(__file__).resolve().parent))
from flat2d import redraw  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / "data/raw/icons"
OUT = ROOT / "react-frontend/public/icons"
SIZE = 64
ST = "https://raw.githubusercontent.com/greeny/SatisfactoryTools/2192bd15"
FL = "https://raw.githubusercontent.com/factoriolab/factoriolab/a61e43fdf9e969e13f3a0ef7f67f49886f9fe268/public/data"
FALLBACK = {"steam-500": "steam", "ray-receiver-pro": "ray-receiver", "ray-receiver-dark-fog": "ray-receiver"}


def fetch(url, dst):
    if not dst.exists():
        dst.parent.mkdir(parents=True, exist_ok=True)
        req = urllib.request.Request(url, headers={"User-Agent": "factory-planner-icons"})
        with urllib.request.urlopen(req, timeout=60) as r:
            dst.write_bytes(r.read())
    return dst


def ids_of(game_file):
    d = json.loads((ROOT / "data" / game_file).read_text(encoding="utf-8"))
    ids = [i["id"] for i in d["items"]] + [b["id"] for b in d["buildings"]]
    return d, list(dict.fromkeys(ids))


def satisfactory():
    d, ids = ids_of("game-data.json")
    raw = json.loads(fetch(f"{ST}/data/data1.0.json", CACHE / "sf-data1.0.json").read_text(encoding="utf-8"))
    slug = {k: v["slug"] for k, v in raw["items"].items()}
    slug.update({k: v["slug"] for k, v in raw["buildings"].items()})
    icons = {}
    for i in ids:
        s = slug.get(i)
        if not s:
            print("  no slug:", i)
            continue
        try:
            p = fetch(f"{ST}/www/assets/images/items/{s}_64.png", CACHE / "sf" / f"{s}_64.png")
            icons[i] = Image.open(p).convert("RGBA").resize((SIZE, SIZE), Image.LANCZOS)
        except Exception as e:  # noqa: BLE001
            print("  missing:", i, s, e)
    return icons


def factoriolab(game_file, folder):
    d, ids = ids_of(game_file)
    raw = json.loads((ROOT / "data/raw" / f"factoriolab-{folder}.json").read_text(encoding="utf-8"))
    pos = {i["id"]: (i["x"], i["y"]) for i in raw["icons"]}
    sheet = Image.open(fetch(f"{FL}/{folder}/icons.webp", CACHE / f"{folder}-icons.webp")).convert("RGBA")
    icons = {}
    for i in ids:
        xy = pos.get(i) or pos.get(FALLBACK.get(i, ""))
        if not xy:
            print("  missing:", i)
            continue
        x, y = xy
        icons[i] = sheet.crop((x, y, x + SIZE, y + SIZE))
    return icons


def flat(img):
    """Simplified 2D version of one icon (cel-shaded look)."""
    a = np.array(img)
    mask = a[..., 3] > 110
    if mask.sum() < 20:
        return img
    rgb = Image.fromarray(a[..., :3]).filter(ImageFilter.MedianFilter(3))
    rgb = ImageEnhance.Color(rgb).enhance(1.3)
    rgb = ImageEnhance.Contrast(rgb).enhance(1.1)
    px = np.array(rgb)[mask].reshape(-1, 1, 3)
    pal = Image.fromarray(px.astype(np.uint8)).quantize(6, method=Image.Quantize.MEDIANCUT)
    idx = rgb.quantize(palette=pal, dither=Image.Dither.NONE).filter(ImageFilter.ModeFilter(3))
    q = np.array(idx.convert("RGB"))
    m = Image.fromarray((mask * 255).astype(np.uint8)).filter(ImageFilter.ModeFilter(3))
    mask = np.array(m) > 0
    out = np.zeros_like(a)
    out[..., :3] = q
    out[..., 3] = np.where(mask, 255, 0)
    ring = (np.array(m.filter(ImageFilter.MaxFilter(3))) > 0) & ~mask
    edge = (q[mask].mean(axis=0) * 0.3).astype(np.uint8)
    out[ring, :3] = edge
    out[ring, 3] = 255
    return Image.fromarray(out)


def write(game, icons):
    OUT.mkdir(parents=True, exist_ok=True)
    ids = sorted(icons)
    cols = 16
    rows = (len(ids) + cols - 1) // cols
    for style, fn in (("original", lambda im: im), ("flat", redraw), ("mono", lambda im: redraw(im, mono=True))):
        sheet = Image.new("RGBA", (cols * SIZE, rows * SIZE), (0, 0, 0, 0))
        for n, i in enumerate(ids):
            sheet.paste(fn(icons[i]), ((n % cols) * SIZE, (n // cols) * SIZE))
            if n % 40 == 0:
                print(f"  {game} {style} {n}/{len(ids)}", flush=True)
        sheet.save(OUT / f"{game}-{style}.webp", "WEBP", quality=90, method=6)
    (OUT / f"{game}.json").write_text(
        json.dumps({"size": SIZE, "cols": cols, "rows": rows, "index": {i: n for n, i in enumerate(ids)}}),
        encoding="utf-8")
    print(f"{game}: {len(ids)} icons")


def main(games):
    for g in games:
        if g == "satisfactory":
            write(g, satisfactory())
        elif g == "dsp":
            write(g, factoriolab("dsp.json", "dsp"))
        elif g == "factorio":
            write(g, factoriolab("factorio.json", "2.0"))


if __name__ == "__main__":
    main(sys.argv[1:] or ["satisfactory", "dsp", "factorio"])
