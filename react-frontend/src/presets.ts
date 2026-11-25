// Ready-made plans (Xinyu 2026-11-24: 「也给做一些预设，比如说大型 factorial 的话…大型的模组产线，或者说大型的
// 传奇品质或者品质产线…戴森球的话可以给出混代结构规划…幸福工厂…最后的4种太空电梯材料按时间的规划」).
// Each preset starts from the game's defaults and sets only what it is about; rates are MINE picks of a
// "large" factory unless a source is given.
import { t } from './i18n'
import { defaultSettings, withDataset, withQ, type Settings } from './solver'

export interface Preset {
  id: string
  name: () => string
  desc: () => string
  /** Tab to open after applying ('mixed' = DSP mixed belts). */
  tab?: 'table' | 'mixed'
  settings: () => Settings
}

/** Space Elevator deliveries per phase at the default 1× cost (satisfactory.wiki.gg Project_Assembly, the
 *  per-phase delivery list, 1.0). Part ids: 1 Smart Plating, 2 Versatile Framework, 3 Automated Wiring,
 *  4 Modular Engine, 5 Adaptive Control Unit, 6 Magnetic Field Generator, 7 Assembly Director System,
 *  8 Thermal Propulsion Rocket, 9 Nuclear Pasta, 10 Biochemical Sculptor, 11 Ballistic Warp Drive,
 *  12 AI Expansion Server. Phase 5 is "the final four parts". */
export const SPACE_ELEVATOR: Record<number, [string, number][]> = {
  1: [['Desc_SpaceElevatorPart_1_C', 50]],
  2: [['Desc_SpaceElevatorPart_1_C', 1000], ['Desc_SpaceElevatorPart_2_C', 1000], ['Desc_SpaceElevatorPart_3_C', 100]],
  3: [['Desc_SpaceElevatorPart_2_C', 2500], ['Desc_SpaceElevatorPart_4_C', 500], ['Desc_SpaceElevatorPart_5_C', 100]],
  4: [['Desc_SpaceElevatorPart_7_C', 500], ['Desc_SpaceElevatorPart_6_C', 500], ['Desc_SpaceElevatorPart_8_C', 250],
    ['Desc_SpaceElevatorPart_9_C', 100]],
  5: [['Desc_SpaceElevatorPart_9_C', 1000], ['Desc_SpaceElevatorPart_10_C', 1000], ['Desc_SpaceElevatorPart_11_C', 200],
    ['Desc_SpaceElevatorPart_12_C', 256]],
}

/** Targets per minute that deliver a phase in `hours` at cost multiplier `mult`. */
export function elevatorTargets(phase: number, hours: number, mult: number) {
  const min = Math.max(1, hours * 60)
  // 6 decimals: readable in the rate field, and 256 servers in 8 h still come to 256.00 (review: 4 gave 255.98)
  return (SPACE_ELEVATOR[phase] ?? []).map(([item, n]) => ({ item, rate: +((n * mult) / min).toFixed(6) }))
}

/** Presets search longer than the 4 s default: large models (review: 4–24 % gaps at 4 s). */
const LONG = 20000

export const PRESETS: Preset[] = [
  {
    id: 'sf-phase5-final',
    name: () => t('Satisfactory · Phase 5 final four parts in 8 h', '幸福工厂 · 8 小时交齐第 5 阶段最后四种材料'),
    desc: () => t('1000 Nuclear Pasta, 1000 Biochemical Sculptors, 200 Ballistic Warp Drives, 256 AI Expansion Servers (wiki, 1× cost) spread over 8 hours; all alternates, whole buildings, 250 % clocks.',
      '1000 核意面、1000 生化塑造器、200 弹道跃迁引擎、256 AI拓展伺服器（wiki，1× 成本）分摊到 8 小时；全部替代配方、整数建筑、可超频到 250%。'),
    settings: () => ({
      ...defaultSettings('satisfactory'),
      targets: elevatorTargets(5, 8, 1),
      time_limit_ms: LONG,
      allow_all_alternates: true,
      max_clock_percent: 250,
      objective: 'buildings',
    }),
  },
  {
    id: 'sf-phase4',
    name: () => t('Satisfactory · Phase 4 in 8 h', '幸福工厂 · 8 小时交齐第 4 阶段'),
    desc: () => t('500 Assembly Director Systems, 500 Magnetic Field Generators, 250 Thermal Propulsion Rockets, 100 Nuclear Pasta over 8 hours; all alternates, whole buildings.',
      '500 组装编导系统、500 磁场发生器、250 热能推进火箭、100 核意面分摊到 8 小时；全部替代配方、整数建筑。'),
    settings: () => ({ ...defaultSettings('satisfactory'), targets: elevatorTargets(4, 8, 1), allow_all_alternates: true, objective: 'buildings', time_limit_ms: LONG }),
  },
  {
    id: 'fa-modules',
    name: () => t('Factorio Space Age · module megafactory', '异星工厂太空时代 · 大型插件工厂'),
    desc: () => t('Tier-3 productivity, quality, speed and efficiency modules, 30/min each, every planet enabled; productivity modules in the machines, speed-module beacons; fewest buildings.',
      '三级产能、品质、速度、节能插件各 30/分钟，开启所有星球；机器里放产能插件，插件塔放速度插件；建筑最少。'),
    settings: () => ({
      ...withDataset(defaultSettings('factorio'), 'spa'),
      targets: ['productivity-module-3', 'quality-module-3', 'speed-module-3', 'efficiency-module-3'].map(item => ({ item, rate: 30 })),
      planets: ['nauvis', 'vulcanus', 'gleba', 'fulgora', 'aquilo', 'space-platform'],
      // productivity modules where allowed, speed-module beacons around the machines
      effects: ['productivity-module-3'],
      beacon_module: 'speed-module-3',
      objective: 'buildings',
      time_limit_ms: LONG,
    }),
  },
  {
    id: 'fa-legendary',
    name: () => t('Factorio Space Age · legendary quality modules', '异星工厂太空时代 · 传说级品质插件'),
    desc: () => t('1 legendary quality module 3 per minute: quality modules in the quality slots, productivity modules where allowed, recycling along the chain, every planet enabled.',
      '每分钟 1 个传说级三级品质插件：品质插槽放品质插件，能放产能插件的放产能插件，沿产线回收，开启所有星球。'),
    settings: () => ({
      ...withDataset(defaultSettings('factorio'), 'spa'),
      targets: [{ item: withQ('quality-module-3', 4), rate: 1 }],
      quality_module: 'quality-module-3',
      prod_module: 'productivity-module-3',
      recycling: 'chain',
      time_limit_ms: LONG,
      planets: ['nauvis', 'vulcanus', 'gleba', 'fulgora', 'aquilo', 'space-platform'],
      objective: 'resources',
    }),
  },
  {
    id: 'dsp-em-mixed',
    name: () => t('DSP · electromagnetic matrix on one mixed belt', '戴森球 · 电磁矩阵混带'),
    desc: () => t('Electromagnetic matrix 600/min with Mk.III proliferator on mixed input belts; opens the Mixed belts tab with the station layout.',
      '电磁矩阵 600/分钟，喷 Mk.III 增产剂，混带进料；打开「混带」页看物流塔布局。'),
    tab: 'mixed',
    settings: () => ({
      ...defaultSettings('dsp'),
      targets: [{ item: 'electromagnetic-matrix', rate: 600 }],
      time_limit_ms: LONG,
      effects: ['proliferator-3-speed', 'proliferator-3-products'],
      mixed_belts: true,
      objective: 'compute',
    }),
  },
  {
    id: 'dsp-white-mixed',
    name: () => t('DSP · universe matrix base on mixed belts', '戴森球 · 白糖全产线混带'),
    desc: () => t('Universe matrix 60/min, all six matrices on mixed belts with Mk.III proliferator, fewest compute; opens the Mixed belts tab.',
      '宇宙矩阵（白糖）60/分钟，六种矩阵全部混带、喷 Mk.III，算力最少；打开「混带」页。'),
    tab: 'mixed',
    settings: () => ({
      ...defaultSettings('dsp'),
      targets: [{ item: 'universe-matrix', rate: 60 }],
      time_limit_ms: LONG,
      effects: ['proliferator-3-speed', 'proliferator-3-products'],
      allow_all_alternates: true,
      mixed_belts: true,
      objective: 'compute',
    }),
  },
]
