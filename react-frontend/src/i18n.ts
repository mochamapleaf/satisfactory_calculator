// Interface language (Xinyu 2026-11-24: 「可以全中文切换」). The App keeps the choice in state and
// remounts the page under `key={lang}` when it changes, so plain calls to `t` read the current one.
export type Lang = 'zh' | 'en'
const KEY = 'lang'

function initial(): Lang {
  try {
    const v = localStorage.getItem(KEY)
    if (v === 'zh' || v === 'en') return v
  } catch { /* private mode: default */ }
  return 'zh'
}

let current: Lang = initial()

export const lang = (): Lang => current
export function setLang(l: Lang) {
  current = l
  try { localStorage.setItem(KEY, l) } catch { /* ignore */ }
  document.documentElement.lang = l === 'zh' ? 'zh-CN' : 'en'
}
document.documentElement.lang = current === 'zh' ? 'zh-CN' : 'en'

/** The text in the current language: `t('Raw resources', '原矿')`. */
export const t = (en: string, zh: string): string => (current === 'zh' ? zh : en)

// Chinese versions of the solver's English warnings (src/planner/*.rs). One rule per Rust template; numbers
// and names are kept from the English text. Unmatched text is shown as is.
type WarnRule = [RegExp, (m: RegExpMatchArray) => string]
/** Tail of the sprayed-recipes list: " and N more" in Chinese. */
const zhRecipes = (x: string) => x.replace(/ and (\d+) more$/, ' 等 $1 个')

const WARN_RULES: WarnRule[] = [
  [/^No whole-building plan fits the Somersloop \/ power shard budget\.$/, () => '没有整数建筑方案能满足能量碎片 / 索莫晶体的数量上限。'],
  [/^Time limit reached before a whole-building plan was found\.$/, () => '到了时间上限，还没找到整数建筑方案。'],
  [/^Solver error: (.*)$/s, m => `求解器出错：${m[1]}`],
  [/^Time limit reached: best whole-building plan found, not proven optimal \(within ([\d.,]+) % of the bound(?:, about ([\d.,]+) of the cheapest building)?\)\.$/,
    m => `到了时间上限：已找到当前最好的整数建筑方案，但未证明最优（与下界相差不到 ${m[1]} %${m[2] ? `，约 ${m[2]} 座最便宜的建筑` : ''}）。`],
  [/^unknown target item '(.*)'$/, m => `未知的目标物品“${m[1]}”`],
  [/^This data set has no quality: items asked for at a quality above normal are planned as normal\.$/, () => '这套数据没有品质：要求高于普通品质的物品，按普通品质规划。'],
  [/^Large model: (\d+) columns × (\d+) items \(the limit for automatic solving is (\d+)\)\.$/,
    m => `模型太大：${m[1]} 列 × ${m[2]} 种物品（自动求解的上限是 ${m[3]} 列）。`],
  [/^Solver stopped before finding a plan\.$/, () => '求解器还没找到方案就停了。'],
  [/^No feasible plan: targets exceed resource limits or need a disabled recipe\.$/, () => '无可行方案：目标超过了原矿上限，或需要已禁用的配方。'],
  [/^Not on the chosen planets: (.*)\. Add the planet or give the item as an input\.$/s,
    m => `所选星球上没有：${m[1]}。请加上这个星球，或把该物品设为输入。`],
  [/^Self-powered: generators make ([\d.,]+) MW for the plan's own ([\d.,]+) MW(?: \+ ([\d.,]+) MW asked)? \(average draw; variable-power buildings peak higher\)\.$/,
    m => `自供电：发电机发出 ${m[1]} MW，供方案自身的 ${m[2]} MW${m[3] ? ` + 另要的 ${m[3]} MW` : ''}（按平均功率算，变功率建筑的峰值更高）。`],
  [/^Nuclear waste left over \((.*)\): it cannot be sunk; store it or process it \(Plutonium \/ Ficsonium chain\)\.$/,
    m => `剩余核废料（${m[1]}）：无法销毁，请存放或继续加工（钚 / 镄链）。`],
  [/^(\d+) recipe\(s\) are sprayed with (.*) although (.*) is enabled \((.*)\)\. This is the steady-state optimum for this objective, not a build-order effect: a lower-tier spray needs far fewer upstream buildings \((.*) is made from the lower tiers plus extra ingredients\), and on these recipes that outweighs its smaller bonus\. To spray everything with (.*), untick the lower-tier proliferator modes; the plan then costs a little more by the objective \((.*)\)\.$/s,
    m => `${m[1]} 个配方喷涂的是 ${m[2]}，尽管已启用 ${m[3]}（${zhRecipes(m[4])}）。这是该目标下的稳态最优，与建造顺序无关：低一级的增产剂需要的上游建筑少得多（${m[5]} 要用低级增产剂再加额外原料制造），在这些配方上比它较小的加成更划算。想全部喷涂 ${m[6]}，请取消勾选低级增产剂模式；方案按目标算会略贵一点（${m[7].replace(/^0\.6 % on 60 universe matrix\/min, Buildings$/, '60 个/分钟宇宙矩阵时多 0.6 %，目标为建筑数')}）。`],
  [/^mixed belts are modelled for Dyson Sphere Program only$/, () => '混带只支持戴森球计划。'],
]

/** A solver warning in the current language (English text comes from Rust; unknown text stays as is). */
export function trWarn(w: string): string {
  if (current === 'en') return w
  for (const [re, f] of WARN_RULES) {
    const m = w.match(re)
    if (m) return f(m)
  }
  return w
}
