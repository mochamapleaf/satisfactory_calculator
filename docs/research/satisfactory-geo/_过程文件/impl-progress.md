# impl-progress — satisfactory-geo implementation (G1-G3)

## 2026-11-21 G1 done
- Fetched WorldResourceNodes.json @bb997b42 (HTTP 200, 169 384 bytes) -> data/satisfactory-geo/raw/.
- tools/geo_nodes.py -> data/satisfactory-geo/nodes.json (626 records: 459 node, 118 well, 17 well_core, 31 geyser, 1 deposit).
  Asserts: totals == game-data.json limits (12 items), item ids exist, iron purity 39/42/46, wells 13125 m3/min. Deterministic (md5 stable).
- SOURCE.md section appended. Next: G2 transport.json.
