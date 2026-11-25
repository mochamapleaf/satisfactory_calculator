"""Convert a SatisfactoryTools-format data dump (parsed from the game's Docs.json)
into the compact game-data.json the Rust solver embeds.

Usage:  python -I tools/convert_data.py <path/to/data1.0.json> data/game-data.json

Source pinned in data/SOURCE.md. Rates are converted to items (or m^3) per minute
at 100% clock with no Somersloops.
"""
import json
import sys

# Somersloop (production amplifier) slots, satisfactory.wiki.gg/wiki/Production_amplifier (2026-11-19).
SOMERSLOOP_SLOTS = {
    "Desc_SmelterMk1_C": 1,
    "Desc_ConstructorMk1_C": 1,
    "Desc_AssemblerMk1_C": 2,
    "Desc_FoundryMk1_C": 2,
    "Desc_OilRefinery_C": 2,
    "Desc_Converter_C": 2,
    "Desc_ManufacturerMk1_C": 4,
    "Desc_Blender_C": 4,
    "Desc_HadronCollider_C": 4,
    "Desc_QuantumEncoder_C": 4,
    "Desc_Packager_C": 0,
}

# World extraction maxima (per minute), satisfactory.wiki.gg/wiki/Resource_node (2026-11-19).
# Nitrogen: 12,000 (resource wells, SatisfactoryTools default). Water: unlimited (None).
RESOURCE_LIMITS = {
    "Desc_OreIron_C": 92100,
    "Desc_OreCopper_C": 36900,
    "Desc_Stone_C": 69300,
    "Desc_Coal_C": 42300,
    "Desc_OreGold_C": 15000,
    "Desc_RawQuartz_C": 13500,
    "Desc_Sulfur_C": 10800,
    "Desc_OreBauxite_C": 12300,
    "Desc_OreUranium_C": 2100,
    "Desc_SAM_C": 10200,
    "Desc_LiquidOil_C": 12600,
    "Desc_NitrogenGas_C": 12000,
    "Desc_Water_C": None,
}

# Extraction building used for the power estimate of raw inputs:
# (building name, output per minute at 100% on a normal node, MW at 100%).
EXTRACTORS = {
    "Desc_Water_C": ("Water Extractor", 120, 20),
    "Desc_LiquidOil_C": ("Oil Extractor", 120, 40),
    "Desc_NitrogenGas_C": ("Resource Well Extractor", 60, 150 / 4),  # pressurizer 150 MW shared by ~4 satellites
}
DEFAULT_EXTRACTOR = ("Miner Mk.3", 240, 45)


SF_GROUP = {
    "Desc_SmelterMk1_C": "ingots", "Desc_FoundryMk1_C": "ingots",
    "Desc_ConstructorMk1_C": "basic", "Desc_AssemblerMk1_C": "intermediate",
    "Desc_ManufacturerMk1_C": "advanced", "Desc_OilRefinery_C": "refined", "Desc_Blender_C": "refined",
    "Desc_Packager_C": "fluids", "Desc_HadronCollider_C": "exotic", "Desc_QuantumEncoder_C": "exotic",
    "Desc_Converter_C": "exotic",
}


def sf_group(k, v, is_raw, building):
    if is_raw:
        return "raw"
    if "SpaceElevatorPart" in k:
        return "project"
    if v.get("liquid"):
        return "fluids"
    # solid Refinery/Blender products (plastic, rubber, silica, ...) are "refined"; Packager output
    # (packaged fluids) stays with the fluids
    return SF_GROUP.get(building, "other")


def main(src, dst):
    d = json.load(open(src, encoding="utf8"))
    items_used = set()
    recipes = []
    buildings_used = set()
    for r in d["recipes"].values():
        if not r["inMachine"] or r["forBuilding"]:
            continue
        machines = [b for b in r["producedIn"] if b in SOMERSLOOP_SLOTS]
        if not machines:
            continue
        b = machines[0]
        per_min = 60.0 / r["time"]
        ins = [[i["item"], round(i["amount"] * per_min, 6)] for i in r["ingredients"]]
        outs = [[i["item"], round(i["amount"] * per_min, 6)] for i in r["products"]]
        for it, _ in ins + outs:
            items_used.add(it)
        buildings_used.add(b)
        rec = {
            "id": r["className"],
            "name": r["name"].replace("Alternate: ", ""),
            "alt": bool(r["alternate"]),
            "building": b,
            "time": r["time"],
            "in": ins,
            "out": outs,
        }
        if r["isVariablePower"]:
            rec["power"] = (r["minPower"] + r["maxPower"]) / 2.0
            rec["power_range"] = [r["minPower"], r["maxPower"]]
        recipes.append(rec)

    # generators as recipes (memo step 8): fuel (+ water) → "Electricity" units/min = MW. Fuel per minute
    # = MW·60 / energy value (MJ per item or per m³); water m³/min = waterToPowerRatio·MW·60/1000
    # (coal 75 MW → 15 coal, 45 water; nuclear 2500 MW → 0.2 rods, 240 water). Waste per rod is not in
    # the dump: 50 uranium waste per uranium rod, 10 plutonium waste per plutonium rod, none for
    # ficsonium (satisfactory.wiki.gg Nuclear_Power_Plant, 1.0).
    waste = {"Desc_NuclearFuelRod_C": ("Desc_NuclearWaste_C", 50), "Desc_PlutoniumFuelRod_C": ("Desc_PlutoniumWaste_C", 10)}
    generators = {}
    for g in d["generators"].values():
        mw = g["powerProduction"]
        water = g.get("waterToPowerRatio", 0) * mw * 60 / 1000
        for f in g["fuel"]:
            e = d["items"][f]["energyValue"]
            if not e:
                continue
            rate = round(mw * 60 / e, 6)
            ins = [[f, rate]] + ([["Desc_Water_C", round(water, 6)]] if water else [])
            outs = [["Electricity", float(mw)]]
            if f in waste:
                w, k = waste[f]
                outs.append([w, round(rate * k, 6)])
            for it, _ in ins + outs[1:]:
                items_used.add(it)
            generators[g["className"]] = g
            recipes.append({
                "id": f"Generator_{f}",
                "name": f"{d['buildings'][g['className']]['name']}: {d['items'][f]['name']}",
                "alt": False,
                "building": g["className"],
                "time": 60.0,
                "in": ins,
                "out": outs,
                "generator": True,
            })

    raw = set(d["resources"].keys())
    # item picker groups: by the building of the item's own recipe, i.e. the first standard recipe that
    # has it as its main (first) product. Byproducts (Silica from Alumina Solution) and "Unpackage ..."
    # recipes (Empty Canister) only count when nothing else makes the item.
    made_in = {}
    ordered = sorted(d["recipes"].values(), key=lambda r: r.get("alternate", False))
    for main_only in (True, False):
        for r in ordered:
            if r.get("name", "").startswith("Unpackage") and main_only:
                continue
            bs = [b for b in r.get("producedIn", []) if b in SF_GROUP]
            prods = r.get("products", []) if bs else []
            for p in prods[:1] if main_only else prods:
                made_in.setdefault(p["item"], bs[0])
    items = []
    for k, v in d["items"].items():
        if k in items_used:
            items.append({
                "id": k,
                "name": v["name"],
                "liquid": bool(v["liquid"]),
                "sink": v.get("sinkPoints", 0),
                "raw": k in raw,
                "group": sf_group(k, v, k in raw, made_in.get(k, "")),
                "row": 0,
                "order": v.get("sinkPoints", 0),
            })
    if generators:
        items.append({"id": "Electricity", "name": "Electricity (MW)", "liquid": False, "sink": 0, "raw": False,
                      "group": "power", "row": 0, "order": 0})
    buildings = []
    for k in sorted(generators):
        v = d["buildings"][k]
        # generators draw no power; clocking scales fuel and output alike (no power exponent)
        buildings.append({"id": k, "name": v["name"], "power": 0, "power_exp": 1.0, "somersloop_slots": 0, "generator": True})
    for k in sorted(buildings_used):
        v = d["buildings"][k]
        buildings.append({
            "id": k,
            "name": v["name"],
            "power": v["metadata"].get("powerConsumption", 0),
            "power_exp": v["metadata"].get("powerConsumptionExponent", 1.321929),
            "somersloop_slots": SOMERSLOOP_SLOTS[k],
        })
    resources = []
    for k in sorted(raw):
        name, rate, mw = EXTRACTORS.get(k, DEFAULT_EXTRACTOR)
        resources.append({
            "item": k,
            "limit": RESOURCE_LIMITS.get(k),
            "extractor": name,
            "extractor_rate": rate,
            "extractor_power": mw,
        })
    # progression for the speedrun planner (memo step 17): tutorial unlocks, milestones (tier, cost,
    # HUB ship time, unlocked recipes), and what each building costs to place. Costs keep only
    # items the machines make (hand-crafted ones such as the Portable Miner are dropped).
    known = {i["id"] for i in items}
    tutorial = sorted({r for x in d["schematics"].values() if x["type"] == "EST_Tutorial" for r in x["unlock"]["recipes"]})
    # MAM research unlocks (non-alternate): the speedrun planner treats them as available once the MAM
    # exists, without their research cost (MINE simplification, see src/planner/speedrun.rs)
    mam = sorted({r for x in d["schematics"].values() if x["type"] == "EST_MAM" and not x.get("alternate") for r in x["unlock"]["recipes"]})
    milestones = []
    for x in d["schematics"].values():
        if x["type"] != "EST_Milestone":
            continue
        milestones.append({
            "id": x["className"], "name": x["name"], "tier": x["tier"], "time": x["time"],
            "cost": [[c["item"], c["amount"]] for c in x["cost"] if c["item"] in known],
            "recipes": sorted(x["unlock"]["recipes"]),
        })
    milestones.sort(key=lambda m: (m["tier"], m["name"]))
    # MAM research nodes (speedrun v2: their cost is paid by the first step that uses one of their recipes)
    mam_nodes = sorted(({
        "id": x["className"], "name": x["name"], "time": x["time"],
        "cost": [[c["item"], c["amount"]] for c in x["cost"] if c["item"] in known],
        "recipes": sorted(x["unlock"]["recipes"]),
    } for x in d["schematics"].values() if x["type"] == "EST_MAM"), key=lambda n: n["id"])
    building_costs = {}
    for r in d["recipes"].values():
        if not r["forBuilding"] or not r["products"]:
            continue
        p = r["products"][0]["item"]
        building_costs[p] = {
            "recipe": r["className"],
            "name": r["name"],
            "cost": [[i["item"], i["amount"]] for i in r["ingredients"] if i["item"] in known],
        }
    recipes.sort(key=lambda r: (r["alt"], r["name"]))
    items.sort(key=lambda i: i["name"])
    out = {
        "version": "Satisfactory 1.0 (unchanged for production in 1.1)",
        "source": "greeny/SatisfactoryTools data/data1.0.json @2192bd15 (parsed from Docs.json)",
        "items": items,
        "recipes": recipes,
        "buildings": buildings,
        "resources": resources,
        "progression": {"tutorial_recipes": tutorial, "mam_recipes": mam, "mam_nodes": mam_nodes, "milestones": milestones, "building_costs": building_costs},
        "belts": [60, 120, 270, 480, 780, 1200],
        "pipes": [300, 600],
    }
    with open(dst, "w", encoding="utf8", newline="\n") as f:
        json.dump(out, f, ensure_ascii=False, separators=(",", ":"))
    print(f"items={len(items)} recipes={len(recipes)} (alt={sum(r['alt'] for r in recipes)}) "
          f"buildings={len(buildings)} resources={len(resources)}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
