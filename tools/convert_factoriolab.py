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
  machines carry `base_productivity` and `quality_speed` (crafting speed per tier, from `qualityRecord`); modules carry per-quality values `by_quality` (from
  FactorioLab `qualityRecord`, never x1.3); top-level `qualities` lists the tier names.
- Space Age: `locations` (planets / space platform) on recipes, resources and machines (crusher on
  the platform, captive biter spawner on Nauvis), top-level `locations`;
  `beacons` (effectivity, slots, MW, distance `profile`, per-quality values); `research` families
  (technology id, recipes, productivity per level).
- Factorio energy (review 3 H4): machine `power` is electric only. Burner machines (boiler, stone /
  steel furnace, nuclear reactor, heating tower, biochamber nutrients, captive spawner, fusion reactor)
  carry `fuel` {item, rate per minute per machine, result}: the default fuel of their fuel category
  (chemical: `defaults.fuelRank`, coal; others: the first fuel item of the category), burner kW x 60 /
  (fuel MJ x 1000). The heat exchanger burns `heat` (item kept, in MJ: kW x 60 / 1000 per minute).
  Reactor and heating tower run `<machine>-reactor` recipes making heat (synthesised for the 2.0.77
  sets, which lack them); no neighbour bonus. Generators (negative power) are dropped until power is
  balanced in the model.
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


HEAT_SCALE = 1e-3  # FactorioLab heat is in kJ; ours in MJ


def energy(i, game):
    """(electric MW, burner kW, heat kW) of machine item `i`. FactorioLab 2.1 data names `burner` and
    `heat` kW; 2.0.77 data has `type: burner` with the burner kW in `usage`, and the heat exchanger's
    heat in `usage` without a type."""
    m = i["machine"]
    usage = m.get("usage", 0)
    if game != "factorio":
        return usage / 1000.0, 0.0, 0.0
    burner = m.get("burner", usage if m.get("type") == "burner" else 0)
    heat = m.get("heat", 0)
    if i["id"] == "heat-exchanger" and not heat and m.get("type") is None:
        heat, usage = usage, 0
    electric = usage if m.get("type") != "burner" and usage > 0 else 0
    return electric / 1000.0, burner, heat


def fuels_value(b, d):
    """MJ per item of building `b`'s fuel."""
    it = next(i for i in d["items"] if i["id"] == b["fuel"]["item"])
    return it["fuel"]["value"]


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

    # default fuel per fuel category (Factorio burner machines)
    fuel_rank = d.get("defaults", {}).get("fuelRank", [])
    fuels = {}
    for i in sorted((i for i in d["items"] if "fuel" in i), key=lambda i: i["id"] not in fuel_rank):
        for c in i["fuel"].get("types", []):
            fuels.setdefault(c, i)
    generators = []
    buildings, effects, belts, beacons = [], [], [], []
    for i in d["items"]:
        m = i.get("machine")
        if m and "speed" in m or (m and i["id"] == "fractionator"):
            if game == "factorio" and m.get("usage", 0) < 0:
                generators.append(i["id"])  # steam engine / turbine, fusion generator: not modelled yet
                continue
            power, burner, heat = energy(i, game)
            base_p = m.get("baseEffect", {}).get("productivity", 0.0)
            if game == "factorio" and i["id"] == "heating-tower" and not base_p and burner:
                # 2.0.77 data gives the tower's 40 MW heat output as its consumption and no efficiency;
                # the game burns fuel at 250 % efficiency (2.1 data: 16 MW of fuel, productivity +150 %)
                burner, base_p = burner / 2.5, 1.5
            buildings.append({
                "id": i["id"], "name": i["name"], "name_zh": zh.get(i["id"]),
                "power": power, "power_exp": 1.0, "somersloop_slots": 0,
                # DSP fractionator: throughput-bound; speed = Mk.III belt (30 items/s), see notes
                "speed": m.get("speed", 30.0 if i["id"] == "fractionator" else 1.0),
                "modules": m.get("modules", 0),
                "disallowed_effects": m.get("disallowedEffects", []),
                "base_productivity": base_p,
            })
            b = buildings[-1]
            rec = m.get("qualityRecord", {})
            if game == "factorio" and rec and tiers:
                # machine quality: crafting speed per tier (normal ... legendary) from the data
                b["quality_speed"] = [b["speed"]] + [rec.get(t, {}).get("speed", b["speed"]) for t in tiers[1:]]
            if heat:
                b["fuel"] = {"item": "heat", "rate": heat * 60.0 * HEAT_SCALE}
            elif burner:
                f = next((fuels[c] for c in m.get("fuelTypes", []) if c in fuels), None)
                if f is None:
                    raise SystemExit(f"no fuel for burner machine {i['id']}")
                b["fuel"] = {"item": f["id"], "rate": burner * 60.0 / (f["fuel"]["value"] * 1000.0)}
                if f["fuel"].get("result"):
                    b["fuel"]["result"] = f["fuel"]["result"]
            if m.get("locations") and len(d.get("locations", [])) > 1:
                b["locations"] = sorted(m["locations"])
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
        if "beacon" in i:
            be = i["beacon"]
            beacons.append({
                "id": i["id"], "name": i["name"], "effectivity": be.get("effectivity", 1.0),
                "modules": be.get("modules", 0), "power": be.get("usage", 0) / 1000.0,
                "profile": be.get("profile", []), "disallowed_effects": be.get("disallowedEffects", []),
            })
            if tiers:
                rec = be.get("qualityRecord", {})
                beacons[-1]["by_quality"] = [
                    {"effectivity": rec.get(t, {}).get("effectivity", be.get("effectivity", 1.0)),
                     "power": rec.get(t, {}).get("usage", be.get("usage", 0)) / 1000.0} for t in tiers]
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
    fueled = {b["id"] for b in buildings if b.get("fuel")}
    # heat sources: the 2.1 data has `<machine>-reactor` recipes (no inputs, heat out); the 2.0.77 sets
    # have none, so they are synthesised from the burner power (output = fuel x (1 + base productivity))
    if game == "factorio" and any(b.get("fuel", {}).get("item") == "heat" for b in buildings):
        if "heat" not in items_by_id:
            heat_item = {"id": "heat", "name": "Heat", "category": "power"}
            d["items"].append(heat_item)
            items_by_id["heat"] = heat_item
        made_heat = {p for r in d["recipes"] if "heat" in r.get("out", {}) for p in r.get("producers", [])}
        for mid in ("nuclear-reactor", "heating-tower"):
            b = next((x for x in buildings if x["id"] == mid), None)
            if b and mid not in made_heat:
                m = items_by_id[mid]["machine"]
                kw = b["fuel"]["rate"] * fuels_value(b, d) * 1000.0 / 60.0
                d["recipes"].append({"id": f"{mid}-reactor", "name": items_by_id[mid]["name"], "time": 1,
                                     "in": {}, "out": {"heat": kw}, "producers": [mid],
                                     "category": "power", "row": 0})

    # Space Age: planets / space platform (FactorioLab `locations`); recipes and resources tagged with
    # locations are only available there (planner setting `planets`). Base data (Nauvis only): none.
    locations = d.get("locations", [])
    multi_loc = len(locations) > 1

    # items some non-recycling recipe crafts: their "-recycling" recipe is a reverse recipe
    crafted = {x for r in d["recipes"] if r.get("in") and not {"recycling", "mining"} & set(r.get("flags", []))
               for x in r.get("out", {})}
    recipes, resources, raw_items = [], [], set()
    for r in d["recipes"]:
        flags = set(r.get("flags", []))
        if "technology" in flags or "burn" in flags:  # burn: fuel is on the machine (`fuel`)
            continue
        producers = [p for p in r.get("producers", []) if p in machine_ids]
        if not producers:
            # Space Age asteroid fields (per space location, no machine): chunks are raw, weight 100
            # (one chunk crushes to 20-55 ore with modules; planet ores stay the default)
            if not r.get("in") and not r.get("producers"):
                for it in r["out"]:
                    if it not in raw_items:
                        raw_items.add(it)
                        resources.append({"item": it, "limit": None, "extractor": "asteroid collector",
                                          "extractor_rate": 60.0, "extractor_power": 0.0, "weight": 100.0,
                                          "locations": ["space-platform"] if multi_loc else []})
            continue
        # input-less recipes of fuelled machines (reactor heat, biter eggs) are recipes: they burn fuel
        if "mining" in flags or (not r.get("in") and not set(producers) & fueled):
            m = items_by_id[producers[-1]].get("machine", {})
            for it, q in r["out"].items():
                raw_items.add(it)
                old = next((x for x in resources if x["item"] == it), None)
                if old is not None:
                    # a resource found on several planets: available where any of its recipes is
                    if old.get("locations") and r.get("locations"):
                        old["locations"] = sorted(set(old["locations"]) | set(r["locations"]))
                    elif not r.get("locations"):
                        old["locations"] = []
                    continue
                resources.append({
                    "item": it, "limit": None,
                    "extractor": items_by_id[producers[-1]]["name"],
                    "extractor_rate": per_min(q, r["time"]) * m.get("speed", 1.0),
                    "extractor_power": m.get("usage", 0) / 1000.0,
                    "weight": RAW_WEIGHT.get(it, 1.0),
                    "locations": sorted(r.get("locations") or []) if multi_loc else [],
                })
            continue
        if game == "dsp":
            prod_ok = r["id"] in prod_ok_dsp
        else:
            prod_ok = "productivity" not in r.get("disallowedEffects", [])
        scale = lambda k: HEAT_SCALE if k == "heat" else 1.0  # noqa: E731
        recipes.append({
            "id": r["id"], "name": r["name"], "name_zh": zh_r.get(r["id"]) or zh.get(r["id"]),
            "alt": r["id"] in excluded, "building": producers[0], "producers": producers,
            "time": r["time"],
            "in": [[k, per_min(v, r["time"]) * scale(k)] for k, v in r.get("in", {}).items()],
            "out": [[k, per_min(v, r["time"]) * scale(k)] for k, v in r["out"].items()],
            "power": None, "productivity_allowed": prod_ok,
        })
        if tiers:
            ins = list(r["in"])
            recipes[-1]["recycling"] = "recycling" in flags and len(ins) == 1 and (
                ins[0] in crafted or ins[0] in r["out"])
            recipes[-1]["quality_allowed"] = "quality" not in r.get("disallowedEffects", [])
        # where the recipe can run: its own locations and those of some machine that runs it
        if multi_loc:
            locs = set(r.get("locations") or [])
            mlocs = [set(b.get("locations", [])) for b in buildings if b["id"] in producers]
            if mlocs and all(mlocs):
                mu = set().union(*mlocs)
                locs = locs & mu if locs else mu
            if locs:
                recipes[-1]["locations"] = sorted(locs)
        if r.get("catalyst"):
            recipes[-1]["catalyst"] = [[k, per_min(v, r["time"])] for k, v in r["catalyst"].items()]

    # inputs nobody makes (DSP Dark Fog drops, ...) become raw with a high weight
    # (item recycling does not count: it only returns what was crafted)
    made = {x for r in recipes if not r.get("recycling") for x, _ in r["out"]} | raw_items
    made |= {b["fuel"]["result"] for b in buildings if b.get("fuel", {}).get("result")}  # spent fuel cells
    for r in recipes:
        for x, _ in r["in"]:
            if x not in made:
                made.add(x)
                raw_items.add(x)
                resources.append({"item": x, "limit": None, "extractor": "drop / external",
                                  "extractor_rate": 60.0, "extractor_power": 0.0, "weight": 10.0})
    # resources nothing consumes and machines that run no recipe only clutter the pickers
    fuel_items = {b["fuel"]["item"] for b in buildings if b.get("fuel")} | \
        {b["fuel"]["result"] for b in buildings if b.get("fuel", {}).get("result")}
    consumed = {x for r in recipes for x, _ in r["in"]} | fuel_items
    dropped = [x["item"] for x in resources if x["item"] not in consumed]
    resources = [x for x in resources if x["item"] in consumed]
    raw_items -= set(dropped)
    running = {p for r in recipes for p in r["producers"]}
    idle = [b["id"] for b in buildings if b["id"] not in running]
    buildings = [b for b in buildings if b["id"] in running]
    if dropped or idle or generators:
        print(f"skipped unused resources {dropped}, machines without recipes {idle}, generators {generators}")
    used = {x for r in recipes for k in ("in", "out") for x, _ in r[k]} | raw_items
    used |= {x for b in buildings if b.get("fuel") for x in (b["fuel"]["item"], b["fuel"].get("result")) if x}
    items = []
    for n, i in enumerate(d["items"]):
        if i["id"] not in used:
            continue
        items.append({
            "id": i["id"], "name": "Heat (MJ)" if i["id"] == "heat" else i["name"], "name_zh": zh.get(i["id"]),
            # heat has no quality and travels in heat pipes: treated like a fluid
            "liquid": "fluid" in i.get("types", []) or i["id"] == "heat", "sink": 0, "raw": i["id"] in raw_items,
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
    if multi_loc:
        out["locations"] = [{"id": x["id"], "name": x["name"]} for x in locations]
    if beacons:
        out["beacons"] = beacons
    # researched productivity: infinite technologies naming the recipes they boost (+10 % per level)
    rids = {r["id"] for r in recipes}
    research = []
    for i in d["items"]:
        rp = i.get("technology", {}).get("recipeProductivity")
        if rp:
            rs = [x["id"] for x in rp if x["id"] in rids]
            if rs:
                research.append({"id": i["id"], "name": i["name"], "recipes": rs, "step": rp[0]["value"]})
    if research:
        out["research"] = research
    if game == "factorio":
        out["dataset"] = dataset
    name = game if game == "dsp" or dataset == "2.0" else f"{game}-{dataset}"
    dst = ROOT / "data" / f"{name}.json"
    dst.write_text(json.dumps(out, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    print(f"{dst}: {len(items)} items, {len(recipes)} recipes ({sum(r['alt'] for r in recipes)} alt), "
          f"{len(buildings)} buildings, {len(resources)} resources, {len(effects)} effects")


if __name__ == "__main__":
    main(sys.argv[1], *(sys.argv[2:3]))
