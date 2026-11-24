"""Add Chinese names (name_zh) to Satisfactory and Factorio game data (Xinyu 2026-11-24: 「可以全中文切换」).

Source: FactorioLab (MIT) i18n at commit a61e43fd, copied to data/raw/i18n/:
  - factoriolab-sfy-zh.json + factoriolab-sfy-data.json (Satisfactory; matched to our Docs.json
    class names by English name, case-insensitive)
  - factoriolab-2.0-zh.json, factoriolab-spa-zh.json, factoriolab-2x1-zh.json (Factorio; same ids as
    ours; Space Age / 2.1 sets look in their own file first, then 2.0)
Names FactorioLab lacks are in MANUAL (MINE translations, marked so in data/SOURCE.md).
Run: python -I tools/add_zh_names.py
"""
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RAW = os.path.join(ROOT, "data", "raw", "i18n")

# English name -> Chinese, for names FactorioLab has no entry for (MINE)
MANUAL = {
    "Electricity (MW)": "电力 (MW)",
    "Biomass Burner": "生物质燃烧炉",
    "Coal-Powered Generator": "燃煤发电机",
    "Fuel-Powered Generator": "燃料发电机",
    "SAM": "SAM 矿石",
    "Leached Iron ingot": "浸滤铁锭",
    "Leached Iron Ingot": "浸滤铁锭",
    "Converter": "转化器",
    "Quantum Encoder": "量子编码器",
}


def load(name):
    with open(os.path.join(RAW, name), encoding="utf8") as f:
        return json.load(f)


def satisfactory():
    data = load("factoriolab-sfy-data.json")
    zh = load("factoriolab-sfy-zh.json")
    by_name = {}
    for i in data["items"]:
        z = zh["items"].get(i["id"])
        if z:
            by_name[i["name"].lower()] = z
    for r in data["recipes"]:
        z = zh["recipes"].get(r["id"]) or zh["items"].get(r["id"])
        if z:
            by_name.setdefault(r["name"].lower(), z)
    for k, v in MANUAL.items():
        by_name.setdefault(k.lower(), v)

    def name_of(en):
        z = by_name.get(en.lower())
        if z:
            return z
        # generator recipes "Building: Fuel"
        if ": " in en:
            a, b = en.split(": ", 1)
            za, zb = by_name.get(a.lower()), by_name.get(b.lower())
            if za and zb:
                return f"{za}：{zb}"
        return None

    p = os.path.join(ROOT, "data", "game-data.json")
    with open(p, encoding="utf8") as f:
        g = json.load(f)
    missing = []
    for key in ("items", "recipes", "buildings"):
        for x in g[key]:
            z = name_of(x["name"])
            if z:
                x["name_zh"] = z
            else:
                missing.append(x["name"])
    with open(p, "w", encoding="utf8", newline="\n") as f:
        json.dump(g, f, ensure_ascii=False, separators=(",", ":"))
    print(f"satisfactory: {len(missing)} without a Chinese name: {missing}")


def factorio():
    base = load("factoriolab-2.0-zh.json")
    extra = {"factorio-2x1.json": load("factoriolab-2x1-zh.json"), "factorio-spa.json": load("factoriolab-spa-zh.json"),
             "factorio-2q1.json": load("factoriolab-spa-zh.json")}
    for fn in ("factorio.json", "factorio-2q1.json", "factorio-2x1.json", "factorio-spa.json"):
        e = extra.get(fn, {"items": {}, "recipes": {}})
        zh = {k: {**base.get(k, {}), **e.get(k, {})} for k in ("items", "recipes")}
        p = os.path.join(ROOT, "data", fn)
        if not os.path.exists(p):
            continue
        with open(p, encoding="utf8") as f:
            g = json.load(f)
        n = miss = 0
        for key in ("items", "recipes", "buildings"):
            for x in g.get(key, []):
                z = zh["items"].get(x["id"]) if key != "recipes" else (zh["recipes"].get(x["id"]) or zh["items"].get(x["id"]))
                if not z and key == "recipes" and x["id"].endswith("-recycling"):
                    # "<item>-recycling" (MINE: 「X 回收」, as the game's 回收 recipes read)
                    it = zh["items"].get(x["id"][: -len("-recycling")])
                    z = f"{it}回收" if it else None
                if not z and x["id"] == "heat":
                    z = "热能"
                if z:
                    x["name_zh"] = z
                    n += 1
                else:
                    miss += 1
        # modules / sprays, beacons, planets, research productivity (Space Age)
        locs = {**base.get("locations", {}), **e.get("locations", {})}
        for x in g.get("effects", []):
            z = zh["items"].get(x.get("item") or x["id"])
            if z:
                x["name_zh"] = z
        for x in g.get("beacons", []):
            z = zh["items"].get(x["id"])
            if z:
                x["name_zh"] = z
        for x in g.get("locations", []):
            z = locs.get(x["id"]) or zh["items"].get(x["id"])
            if z:
                x["name_zh"] = z
        for x in g.get("research", []):
            stem = x["id"][: -len("-productivity")] if x["id"].endswith("-productivity") else x["id"]
            # MINE: "asteroid" productivity boosts the asteroid crushing recipes (星块 = asteroid chunk)
            z = zh["items"].get(stem) or zh["recipes"].get(stem) or {"asteroid": "星块"}.get(stem)
            if z:
                x["name_zh"] = f"{z}产能"  # MINE: 「X产能」 as the tech reads in game
        with open(p, "w", encoding="utf8", newline="\n") as f:
            json.dump(g, f, ensure_ascii=False, separators=(",", ":"))
        print(f"{fn}: {n} named, {miss} without")


if __name__ == "__main__":
    satisfactory()
    factorio()
    sys.exit(0)
