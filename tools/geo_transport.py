"""G2: write data/satisfactory-geo/transport.json (capacities and parameters of the transport modes) and check the
throughput formulas against the numbers in docs/research/satisfactory-geo/00-最终报告.md section 2.
Every parameter has `source` and `confidence` (wiki = stated on the wiki page, derived = computed from wiki values,
estimate = assumption that needs an in-game measurement). Usage: python -I tools/geo_transport.py
"""
import json
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'data', 'satisfactory-geo', 'transport.json')
WIKI = 'https://satisfactory.wiki.gg/wiki/'
SFT = 'greeny/SatisfactoryTools data1.0.json @2192bd15'


def p(value, source, confidence, note=None, unit=None):
    d = {'value': value, 'source': source, 'confidence': confidence}
    if unit:
        d['unit'] = unit
    if note:
        d['note'] = note
    return d


DOC = {
    'meta': {
        'opened': '2026-11-21',
        'report': 'docs/research/satisfactory-geo/00-最终报告.md section 2',
        'versions': 'Parameters that differ between game versions carry v1_0 and v1_2 keys; data/game-data.json is 1.0.',
        'confidence': {'wiki': 'stated on the wiki page', 'derived': 'computed from wiki values',
                       'estimate': 'assumption, needs in-game measurement'},
    },
    'belts': {
        'tiers': p([60, 120, 270, 480, 780, 1200], 'data/game-data.json belts; ' + WIKI + 'Conveyor_Belts', 'wiki',
                   unit='items/min, Mk.1..Mk.6'),
        'unlock_tier': p([0, 2, 4, 5, 7, 9], 'schematics in ' + SFT, 'wiki', unit='milestone tier, Mk.1..Mk.6'),
        'detour_factor': p(1.3, 'report section 7.1', 'estimate', 'straight-line distance x factor, user-adjustable'),
    },
    'pipes': {
        'tiers': p([300, 600], 'data/game-data.json pipes; ' + WIKI + 'Pipelines', 'wiki', unit='m3/min, Mk.1..Mk.2'),
        'unlock_tier': p([3, 6], 'schematics in ' + SFT, 'wiki'),
        'pumps_and_head_lift': p('not modelled', 'report section 2', 'estimate'),
    },
    'rail': {
        'track_cost_per_12m': p({'steel_pipe': 1, 'steel_beam': 1}, WIKI + 'Railway', 'wiki',
                                'first 18 m, then each further 12 m'),
        'track_cost_per_km': p({'steel_pipe': 83, 'steel_beam': 83}, WIKI + 'Railway', 'derived'),
        'double_track_factor': p(2, 'report section 7.1: single-ended trains cannot reverse, count the track twice',
                                 'estimate'),
        'signals_unlock_tier': p(6, 'Railway Signalling milestone, ' + SFT, 'wiki'),
        'block_headway_capacity': p(None, 'report section 11', 'estimate', 'not quantified'),
    },
    'train': {
        'speed_mps': p(round(120 / 3.6, 3), WIKI + 'Electric_Locomotive (120 km/h own power)', 'wiki', unit='m/s'),
        'car_slots': p(32, WIKI + 'Freight_Car', 'wiki', 'solid freight car'),
        'car_fluid_m3': {'v1_0': p(1600, WIKI + 'Freight_Car', 'wiki', 'old value'),
                         'v1_2': p(2400, WIKI + 'Freight_Car', 'wiki')},
        'car_mixed_fluid_solid': p(False, WIKI + 'Freight_Car', 'wiki', 'one car cannot carry both'),
        'platform_slots_buffer': p(48, WIKI + 'Freight_Platform', 'wiki'),
        'platform_fluid_buffer_m3': p(3200, WIKI + 'Freight_Platform', 'wiki', '1.2'),
        'platform_dock_lock_s': p(27.08, WIKI + 'Freight_Platform', 'wiki', 'ports locked while loading or unloading'),
        'platform_ports': p({'in': 2, 'out': 2}, WIKI + 'Freight_Platform', 'wiki'),
        'platform_direction': p('load or unload, not both', WIKI + 'Freight_Platform', 'wiki'),
        'platform_feed_per_min': p({'solid': 2400, 'fluid': 1200}, 'two Mk.6 belts / two Mk.2 pipes', 'derived',
                                   unit='items or m3 per minute'),
        'platform_power_mw': p({'active': 50, 'idle': 0.1}, WIKI + 'Freight_Platform', 'wiki'),
        'station_power_mw': p(50, WIKI + 'Train_Station', 'wiki'),
        'loco_power_mw': p([25, 110], WIKI + 'Electric_Locomotive', 'wiki'),
        'loco_tractive_kn': p(2000, WIKI + 'Electric_Locomotive', 'wiki'),
        'mass_t': p({'loco': 300, 'car_empty': 30, 'car_full': 100}, WIKI + 'Electric_Locomotive', 'wiki'),
        'accel_loss_s': p({'short_consist': 13, 'heavy_consist': 30},
                          'report section 2.2: v/(2a)+v/(2b) per stop from traction force and mass', 'estimate',
                          'short = 1 loco + 4 full cars; heavy = about 2000 t per loco; station approach limits and signal waits not counted'),
        'ttf_formula': p('TtF = stack*32/belt_per_min + 0.4513 min', WIKI + 'Tutorial:Train_throughput', 'wiki'),
        'build_cost': p({'station': '10 reinforced beam, 50 plastic, 50 concrete, 200 wire',
                         'platform': '5 motor, 10 reinforced beam, 25 plastic, 50 concrete, 100 wire',
                         'car': '5 modular frame, 10 steel pipe, 10 steel beam'}, SFT, 'wiki'),
        'unlock_tier': p(6, 'schematics in ' + SFT, 'wiki'),
    },
    'drone': {
        'slots': p(9, WIKI + 'Drone', 'wiki'),
        'speed_mps': p({'min': 50, 'battery': 75, 'max': 100}, WIKI + 'Drone', 'wiki', 'depends on fuel'),
        'takeoff_s': p(51, WIKI + 'Drone', 'wiki'),
        'landing_s': p(51, WIKI + 'Drone', 'wiki'),
        'fixed_round_trip_s': p(204, 'two take-offs and two landings', 'estimate',
                                'report section 2.3; the wiki gives no round-trip formula'),
        'port_power_mw': p(100, WIKI + 'Drone_Port', 'wiki'),
        'port': p({'drones': 1, 'outputs': 1, 'buffer_in_slots': 18, 'buffer_out_slots': 18}, WIKI + 'Drone_Port', 'wiki'),
        'build_cost': p('Drone Port: 20 heavy modular frame, 20 high-speed connector, 50 aluminum sheet, '
                        '50 aluminum casing, 10 radio control unit', SFT, 'wiki'),
        'unlock_tier': p(8, 'schematics in ' + SFT, 'wiki'),
        'fuel_use': p(None, WIKI + 'Drone', 'estimate', 'shown only in the UI; no formula'),
    },
    'truck': {
        'slots': p(48, WIKI + 'Truck', 'wiki'),
        'power_mw': p(75, WIKI + 'Truck', 'wiki'),
        'average_speed_kmh': p(80, 'report section 2.3', 'estimate',
                               'wiki speed unit is wrong ("89 m/h"); route and terrain matter'),
        'station': p({'power_mw': 20, 'slots': 48, 'inputs': 3, 'outputs': 2}, WIKI + 'Truck_Station', 'wiki',
                     'one input is the fuel port'),
        'dock_rule': {
            'v1_0': p({'transfer_stacks_per_min': 120}, WIKI + 'Truck_Station', 'wiki', '48 slots take 24 s per end'),
            'v1_2': p({'dock_s': 8, 'transfer': 'whole truck at the end of docking'}, WIKI + 'Truck_Station', 'wiki'),
        },
        'unlock_tier': {'tractor': p(3, 'schematics in ' + SFT, 'wiki'), 'truck': p(5, 'schematics in ' + SFT, 'wiki')},
    },
    'power': {
        'line': p('no capacity limit, no loss', WIKI + 'Power_Line', 'wiki'),
        'pressurizer_mw': p({'100%': 150, '250%': 503.66}, WIKI + 'Resource_Well_Pressurizer', 'wiki'),
        'water_extractor': p({'m3_per_min': 120, 'mw': 20, 'deep_water_only': True}, WIKI + 'Water_Extractor', 'wiki'),
    },
}


# --- formulas (the Rust port in G2 follows these) ---------------------------------------------------------------
def train_car_throughput(stack, rtd_s, car_slots=32, dock_s=27.08, belt_per_min=2400.0, fluid_m3=None):
    """Per-car throughput (items or m3 per minute), one direction loaded, return trip empty:
    the smaller of car capacity per round trip and the platform feed (belt rate outside the dock lock)."""
    cap = (fluid_m3 if fluid_m3 else car_slots * stack) * 60.0 / rtd_s
    feed = belt_per_min * (rtd_s - dock_s) / rtd_s
    return min(cap, feed)


def rtd(length_m, t_acc_s, v=120 / 3.6, dock_s=27.08):
    return 2 * length_m / v + 2 * dock_s + 2 * t_acc_s


def drone_items_per_min(dist_m, stack, speed=75.0, fixed=204.0, slots=9):
    return slots * stack * 60.0 / (2 * dist_m / speed + fixed)


def truck_items_per_min(dist_m, stack, version='v1_2', speed_kmh=80.0):
    drive = 2 * dist_m / (speed_kmh / 3.6)
    dock = 2 * 8.0 if version == 'v1_2' else 2 * 48 / 120 * 60
    return 48 * stack * 60.0 / (drive + dock)


def check():
    near = lambda a, b, tol: abs(a - b) <= tol
    # report 2.1: wiki throughput tutorial (RtD 107.08 s for stack 100, not the wiki's 102.08)
    assert near(train_car_throughput(100, 107.08), 1793.1, 0.1)
    assert near(train_car_throughput(500, 427.08), 2247.8, 0.1)
    assert near(train_car_throughput(0, 147.08, fluid_m3=2400, belt_per_min=1200), 979.1, 0.1)
    assert near(train_car_throughput(500, 174.16), 2027, 1)  # under-filled platform
    # report 2.2 table: RtD and per-car throughput for t_acc 13 / 30 s
    assert near(rtd(1000, 13), 140.2, 0.2) and near(rtd(1000, 30), 174.2, 0.2)
    assert round(train_car_throughput(100, rtd(1000, 13))) == 1370
    assert round(train_car_throughput(50, rtd(2000, 13))) == 480
    assert round(train_car_throughput(500, rtd(6000, 13))) == 2181
    assert round(train_car_throughput(200, rtd(4000, 30))) == 1084
    # report 2.3: truck 2 km stack 100 (1.2 ~ 1470, 1.0 ~ 1260); drone battery 2 km (~210, ~1050) and 8 km (~130)
    assert near(truck_items_per_min(2000, 100, 'v1_2'), 1470, 15) and near(truck_items_per_min(2000, 100, 'v1_0'), 1260, 15)
    assert near(drone_items_per_min(2000, 100), 210, 5) and near(drone_items_per_min(2000, 500), 1050, 20)
    assert near(drone_items_per_min(8000, 100), 130, 5)


def main():
    check()
    with open(OUT, 'w', encoding='utf8', newline='\n') as f:
        json.dump(DOC, f, ensure_ascii=False, indent=1)
        f.write('\n')
    print('transport.json written; throughput formulas reproduce the report tables')


if __name__ == '__main__':
    main()
