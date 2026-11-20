# game-data.json provenance

- **Game version:** Satisfactory 1.0. Patch 1.1 (1.1.0.0 / 1.1.1.0) changed no production recipes, rates,
  power or overclock formulas (satisfactory.wiki.gg/wiki/Patch_1.1.0.0, checked 2026-11-19), so the data
  is current for 1.1.
- **Source dump:** `greeny/SatisfactoryTools` `data/data1.0.json` at commit `2192bd15` (2024-09-10, MIT),
  which is parsed from the game's `CommunityResources/Docs/Docs.json`.
  URL: https://raw.githubusercontent.com/greeny/SatisfactoryTools/2192bd15/data/data1.0.json
  (The local Steam install on G: is only a 132 MB stub without CommunityResources, so the game's own
  Docs.json was not available. To refresh from a real install: point `tools/convert_data.py` at a
  SatisfactoryTools-format JSON generated from Docs.json.)
- **Converted by:** `python -I tools/convert_data.py data1.0.json data/game-data.json`
  → 151 items, 275 machine recipes (111 alternates), 11 production buildings, 13 raw resources.
- **Hand-entered tables (in convert_data.py), from satisfactory.wiki.gg on 2026-11-19:**
  - Somersloop slots: Production_amplifier (Smelter/Constructor 1, Assembler/Foundry/Refinery/Converter 2,
    Manufacturer/Blender/Particle Accelerator/Quantum Encoder 4, Packager 0).
    Output ×(1+f), power ×(1+f)², f = filled/total slots; clock power exponent 1.321928.
  - World extraction maxima per minute: Resource_node (iron 92,100; copper 36,900; limestone 69,300;
    coal 42,300; caterium 15,000; quartz 13,500; sulfur 10,800; bauxite 12,300; uranium 2,100;
    SAM 10,200; crude oil 12,600). Nitrogen 12,000 (SatisfactoryTools default); water unlimited.
  - Belts 60/120/270/480/780/1200, pipes 300/600 per minute.
- Variable-power recipes (Particle Accelerator, Converter, Quantum Encoder) use the average of min/max MW.

## Dyson Sphere Program and Factorio (2026-11-20)
- Source: FactorioLab (MIT, https://github.com/factoriolab/factoriolab) at commit
  `a61e43fd`: `public/data/dsp/data.json` (+ `i18n/zh.json` for Chinese names) and
  `public/data/2.0/data.json`. Raw copies in `data/raw/`.
- Versions: DSP 0.10.29.21950; Factorio base 2.0.77 (no Space Age yet).
- Converted by `tools/convert_factoriolab.py dsp|factorio` → `data/dsp.json`, `data/factorio.json`.
  Mining recipes become raw resources (weight 1; water 0.01, crude oil 0.2); inputs no recipe
  makes (DSP Dark Fog drops) become raw "drops" with weight 10; technology and burn
  (generator) recipes are skipped for now.
- DSP fractionator speed is set to 30 items/s (Mk.III belt, unstacked) since its rate is
  belt-bound.
- Game data © Youthcat Studio / Wube Software; only derived numbers are shipped.

## Icons (2026-11-20)
- `tools/build_icons.py` builds `react-frontend/public/icons/<game>-{original,flat}.webp` sprite sheets
  (64 px cells) and `<game>.json` (id → cell). 162 Satisfactory, 176 DSP, 203 Factorio icons (items + buildings).
- Satisfactory: `greeny/SatisfactoryTools` `www/assets/images/items/<slug>_64.png` @2192bd15.
  DSP/Factorio: FactorioLab `public/data/{dsp,2.0}/icons.webp` @a61e43fd. The art belongs to the games'
  developers (Coffee Stain, Youthcat, Wube), as in those fan tools.
- `flat` is generated from the original (palette reduction + outline), not hand-drawn.
