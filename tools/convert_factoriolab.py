"""Convert FactorioLab data (MIT, github.com/factoriolab/factoriolab) to our GameData JSON.

Usage: python tools/convert_factoriolab.py dsp|factorio
Reads data/raw/factoriolab-<dsp|2.0>.json (+ data/raw/factoriolab-dsp-zh.json for Chinese
names), writes data/<dsp|factorio>.json.

Conventions (same as data/game-data.json for Satisfactory):
- recipe rates are per minute for one building at speed 1;
- building power in MW (FactorioLab gives kW);
- mining recipes become `resources` (raw items), not recipes;
- recipes FactorioLab excludes by default (DSP "advanced" recipes) are marked alt.
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = {"dsp": "factoriolab-dsp.json", "factorio": "factoriolab-2.0.json"}
COMMIT = "a61e43fdf9e969e13f3a0ef7f67f49886f9fe268"
# relative cost of one unit/min of a raw resource (default 1)
RAW_WEIGHT = {"water": 0.01, "crude-oil": 0.2, "steam": 0.01}


def per_min(q, time):
    return q * 60.0 / time


def main(game):
    d = json.loads((ROOT / "data/raw" / SRC[game]).read_text(encoding="utf-8"))
    zh, zh_r = {}, {}
    if game == "dsp":
        z = json.loads((ROOT / "data/raw/factoriolab-dsp-zh.json").read_text(encoding="utf-8"))
        zh, zh_r = z.get("items", {}), z.get("recipes", {})
    items_by_id = {i["id"]: i for i in d["items"]}
    excluded = set(d.get("defaults", {}).get("excludedRecipes", []))
    prod_ok_dsp = set(d.get("limitations", {}).get("productivity", []))

    buildings, effects, belts = [], [], []
    for i in d["items"]:
        m = i.get("machine")
        if m and "speed" in m or (m and i["id"] == "fractionator"):
            buildings.append({
                "id": i["id"], "name": i["name"], "name_zh": zh.get(i["id"]),
                "power": m.get("usage", 0) / 1000.0, "power_exp": 1.0, "somersloop_slots": 0,
                # DSP fractionator: throughput-bound; speed = Mk.III belt (30 items/s), see notes
                "speed": m.get("speed", 30.0 if i["id"] == "fractionator" else 1.0),
                "modules": m.get("modules", 0),
                "disallowed_effects": m.get("disallowedEffects", []),
            })
        if "module" in i:
            mo = i["module"]
            effects.append({
                "id": i["id"], "name": i["name"], "name_zh": zh.get(i["id"]),
                "kind": "proliferator" if "proliferator" in mo else "module",
                "speed": mo.get("speed", 0.0), "productivity": mo.get("productivity", 0.0),
                "consumption": mo.get("consumption", 0.0), "quality": mo.get("quality", 0.0),
                "sprays": mo.get("sprays"), "item": mo.get("proliferator", i["id"]),
                "products_only": mo.get("limitation") == "productivity",
            })
        if "belt" in i:
            belts.append(i["belt"]["speed"] * 60.0)
    machine_ids = {b["id"] for b in buildings}

    recipes, resources, raw_items = [], [], set()
    for r in d["recipes"]:
        flags = set(r.get("flags", []))
        if "technology" in flags or "burn" in flags:  # burn = generators, modelled later
            continue
        producers = [p for p in r.get("producers", []) if p in machine_ids]
        if not producers:
            continue
        if "mining" in flags or not r.get("in"):
            m = items_by_id[producers[-1]].get("machine", {})
            for it, q in r["out"].items():
                raw_items.add(it)
                if any(x["item"] == it for x in resources):
                    continue
                resources.append({
                    "item": it, "limit": None,
                    "extractor": items_by_id[producers[-1]]["name"],
                    "extractor_rate": per_min(q, r["time"]) * m.get("speed", 1.0),
                    "extractor_power": m.get("usage", 0) / 1000.0,
                    "weight": RAW_WEIGHT.get(it, 1.0),
                })
            continue
        if game == "dsp":
            prod_ok = r["id"] in prod_ok_dsp
        else:
            prod_ok = "productivity" not in r.get("disallowedEffects", [])
        recipes.append({
            "id": r["id"], "name": r["name"], "name_zh": zh_r.get(r["id"]) or zh.get(r["id"]),
            "alt": r["id"] in excluded, "building": producers[0], "producers": producers,
            "time": r["time"],
            "in": [[k, per_min(v, r["time"])] for k, v in r["in"].items()],
            "out": [[k, per_min(v, r["time"])] for k, v in r["out"].items()],
            "power": None, "productivity_allowed": prod_ok,
        })

    # inputs nobody makes (DSP Dark Fog drops, ...) become raw with a high weight
    made = {x for r in recipes for x, _ in r["out"]} | raw_items
    for r in recipes:
        for x, _ in r["in"]:
            if x not in made:
                made.add(x)
                raw_items.add(x)
                resources.append({"item": x, "limit": None, "extractor": "drop / external",
                                  "extractor_rate": 60.0, "extractor_power": 0.0, "weight": 10.0})
    used = {x for r in recipes for k in ("in", "out") for x, _ in r[k]} | raw_items
    items = []
    for i in d["items"]:
        if i["id"] not in used:
            continue
        items.append({
            "id": i["id"], "name": i["name"], "name_zh": zh.get(i["id"]),
            "liquid": "fluid" in i.get("types", []), "sink": 0, "raw": i["id"] in raw_items,
        })
    version = " ".join(f"{k} {v}" for k, v in d["version"].items())
    out = {
        "game": game,
        "version": ("Dyson Sphere Program " if game == "dsp" else "Factorio ") + version,
        "source": f"FactorioLab public/data/{SRC[game][12:-5]}/data.json @{COMMIT[:8]} (MIT)",
        "items": items, "recipes": recipes, "buildings": buildings, "resources": resources,
        "belts": sorted(belts), "pipes": [], "effects": effects,
    }
    dst = ROOT / "data" / f"{game}.json"
    dst.write_text(json.dumps(out, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    print(f"{dst}: {len(items)} items, {len(recipes)} recipes ({sum(r['alt'] for r in recipes)} alt), "
          f"{len(buildings)} buildings, {len(resources)} resources, {len(effects)} effects")


if __name__ == "__main__":
    main(sys.argv[1])
