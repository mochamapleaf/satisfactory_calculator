"""Convert FactorioLab data (MIT, github.com/factoriolab/factoriolab) to our GameData JSON.

Usage: python tools/convert_factoriolab.py dsp | factorio [2.0|2x1|2q1|spa]
Reads data/raw/factoriolab-<dsp|set>.json (+ data/raw/factoriolab-dsp-zh.json for Chinese
names), writes data/dsp.json, data/factorio.json (base 2.0) or data/factorio-<set>.json.
Factorio data sets: 2.0 = base 2.0.77 @a61e43fd; 2x1 = Space Age 2.1.25, 2q1 = base + quality +
recycler 2.1.25, spa = Space Age 2.0.77 (all three @50d8e977).

Conventions (same as data/game-data.json for Satisfactory):
- recipe rates are per minute for one building at speed 1;
- building power in MW (FactorioLab gives kW);
- mining recipes become `resources` (raw items), not recipes;
- recipes FactorioLab excludes by default (DSP "advanced" recipes) are marked alt;
- quality (Factorio): `recycling` marks item recycling (reverse or self recycling; scrap recycling
  is a normal recipe since nothing crafts scrap), `catalyst` (per minute), `quality_allowed`;
  machines carry `base_productivity`; modules carry per-quality values `by_quality` (from
  FactorioLab `qualityRecord`, never x1.3); top-level `qualities` lists the tier names.
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = {"dsp": "factoriolab-dsp.json", "factorio": "factoriolab-2.0.json"}
COMMIT = "a61e43fdf9e969e13f3a0ef7f67f49886f9fe268"
COMMIT_21 = "50d8e977256344ab0405e32910c26a5fe09c404a"  # 2x1 / 2q1 / spa, fetched 2026-11-21
DATASETS = {"2.0": "Factorio 2.0.77 (base)", "2x1": "Factorio 2.1.25 Space Age",
            "2q1": "Factorio 2.1.25 base + quality", "spa": "Factorio 2.0.77 Space Age"}
# relative cost of one unit/min of a raw resource (default 1)
RAW_WEIGHT = {"water": 0.01, "crude-oil": 0.2, "steam": 0.01}


def per_min(q, time):
    return q * 60.0 / time


def main(game, dataset="2.0"):
    src = SRC[game] if game == "dsp" else f"factoriolab-{dataset}.json"
    d = json.loads((ROOT / "data/raw" / src).read_text(encoding="utf-8"))
    tiers = [q["id"] for q in d.get("qualities", [])]
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
                "base_productivity": m.get("baseEffect", {}).get("productivity", 0.0),
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
            if tiers:
                rec = mo.get("qualityRecord", {})
                effects[-1]["by_quality"] = [
                    {k: rec.get(t, {}).get(k, mo.get(k, 0.0))
                     for k in ("speed", "productivity", "consumption", "quality")} for t in tiers]
        if "belt" in i:
            belts.append(i["belt"]["speed"] * 60.0)
    # Space Age plants (yumako, jellynut, trees) grow from seeds without a machine in the data:
    # an agricultural tower (100 kW, no module slots) runs them, one plant per "building"
    plants = [r for r in d["recipes"] if "plant" in r.get("flags", [])]
    if plants and "agricultural-tower" in items_by_id:
        buildings.append({"id": "agricultural-tower", "name": items_by_id["agricultural-tower"]["name"],
                          "name_zh": None, "power": 0.1, "power_exp": 1.0, "somersloop_slots": 0,
                          "speed": 1.0, "modules": 0, "disallowed_effects": [], "base_productivity": 0.0})
        for r in plants:
            r.setdefault("producers", ["agricultural-tower"])
    machine_ids = {b["id"] for b in buildings}

    # items some non-recycling recipe crafts: their "-recycling" recipe is a reverse recipe
    crafted = {x for r in d["recipes"] if r.get("in") and not {"recycling", "mining"} & set(r.get("flags", []))
               for x in r.get("out", {})}
    recipes, resources, raw_items = [], [], set()
    for r in d["recipes"]:
        flags = set(r.get("flags", []))
        if "technology" in flags or "burn" in flags:  # burn = generators, modelled later
            continue
        producers = [p for p in r.get("producers", []) if p in machine_ids]
        if not producers:
            # Space Age asteroid fields (per space location, no machine): chunks are raw
            if not r.get("in") and not r.get("producers"):
                for it in r["out"]:
                    if it not in raw_items:
                        raw_items.add(it)
                        resources.append({"item": it, "limit": None, "extractor": "asteroid collector",
                                          "extractor_rate": 60.0, "extractor_power": 0.0, "weight": 1.0})
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
        if tiers:
            ins = list(r["in"])
            recipes[-1]["recycling"] = "recycling" in flags and len(ins) == 1 and (
                ins[0] in crafted or ins[0] in r["out"])
            recipes[-1]["quality_allowed"] = "quality" not in r.get("disallowedEffects", [])
        if r.get("catalyst"):
            recipes[-1]["catalyst"] = [[k, per_min(v, r["time"])] for k, v in r["catalyst"].items()]

    # inputs nobody makes (DSP Dark Fog drops, ...) become raw with a high weight
    # (item recycling does not count: it only returns what was crafted)
    made = {x for r in recipes if not r.get("recycling") for x, _ in r["out"]} | raw_items
    for r in recipes:
        for x, _ in r["in"]:
            if x not in made:
                made.add(x)
                raw_items.add(x)
                resources.append({"item": x, "limit": None, "extractor": "drop / external",
                                  "extractor_rate": 60.0, "extractor_power": 0.0, "weight": 10.0})
    used = {x for r in recipes for k in ("in", "out") for x, _ in r[k]} | raw_items
    items = []
    for n, i in enumerate(d["items"]):
        if i["id"] not in used:
            continue
        items.append({
            "id": i["id"], "name": i["name"], "name_zh": zh.get(i["id"]),
            "liquid": "fluid" in i.get("types", []), "sink": 0, "raw": i["id"] in raw_items,
            # in-game crafting-menu layout (tab, row, position), used by the item picker
            "group": i.get("category", "other"), "row": i.get("row", 0), "order": n,
        })
    version = " ".join(f"{k} {v}" for k, v in d["version"].items())
    commit = COMMIT if game == "dsp" or dataset == "2.0" else COMMIT_21
    out = {
        "game": game,
        "version": ("Dyson Sphere Program " + version) if game == "dsp" else DATASETS[dataset],
        "source": f"FactorioLab public/data/{src[12:-5]}/data.json @{commit[:8]} (MIT): {version}",
        "items": items, "recipes": recipes, "buildings": buildings, "resources": resources,
        "belts": sorted(belts), "pipes": [], "effects": effects,
    }
    if tiers:
        out["qualities"] = [q["name"] for q in d["qualities"]]
    if game == "factorio":
        out["dataset"] = dataset
    name = game if game == "dsp" or dataset == "2.0" else f"{game}-{dataset}"
    dst = ROOT / "data" / f"{name}.json"
    dst.write_text(json.dumps(out, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    print(f"{dst}: {len(items)} items, {len(recipes)} recipes ({sum(r['alt'] for r in recipes)} alt), "
          f"{len(buildings)} buildings, {len(resources)} resources, {len(effects)} effects")


if __name__ == "__main__":
    main(sys.argv[1], *(sys.argv[2:3]))
