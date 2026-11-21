# impl-progress — satisfactory-geo implementation (G1-G3)

## 2026-11-21 G1 done
- Fetched WorldResourceNodes.json @bb997b42 (HTTP 200, 169 384 bytes) -> data/satisfactory-geo/raw/.
- tools/geo_nodes.py -> data/satisfactory-geo/nodes.json (626 records: 459 node, 118 well, 17 well_core, 31 geyser, 1 deposit).
  Asserts: totals == game-data.json limits (12 items), item ids exist, iron purity 39/42/46, wells 13125 m3/min. Deterministic (md5 stable).
- SOURCE.md section appended. Next: G2 transport.json.

## 2026-11-21 G2 done
- tools/geo_transport.py -> data/satisfactory-geo/transport.json (belts, pipes, rail, train, drone, truck, power; each value has source + confidence wiki/derived/estimate; 1.0 / 1.2 variants for fluid car and truck dock).
- Asserts reproduce report tables: 1793.1, 2247.8, fluid 979.1, under-filled 2027, section 2.2 table points (1370/480/2181/1084), truck 1470/1260, drone 210/1050/130. Output deterministic (md5 stable).
- Next: G3 tools/geo_sites.py.
