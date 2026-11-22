# 实施结果:报告 §6 第 0–4 步(不含 HiGHS)

日期 2026-11-22 · 分支 `refactor` · 基线 = `c4c6194`(求解器代码同 `058f17c`)。
脚本:`bench/bench_prod.rs`(只用公开的 `plan()`,基线)、`bench/bench_stream.rs`(按 Worker 的方式 `lp::start` + 每步 150 ms,记首个方案时间)、`bench/impl_summary.py`。
场景 = 报告 `bench.rs` 的 120 个(可行 105 个),4 s,原生单线程,串行运行(机器上没有其他基准在跑)。

## 1. 总表(原生,4 s)

| | 有方案 / 105 | 4 s 内证明 | 无方案 | 与已知最好解的差距 中位 | 平均 | >1 % 的个数 | 首个整栋方案 | 已证明的中位耗时 |
|---|---|---|---|---|---|---|---|---|
| 之前 `impl_before4s.jsonl` | 104 | 43 | 1 | +1.00 % | +2.67 % | 31 | 4 s 结束时才有 | 26 ms |
| 之后 `impl_after4s.jsonl` | **105** | **46** | **0** | +0.95 % | +2.64 % | 30 | **中位 6 ms,最大 279 ms** | 6 ms |

- "已知最好解" = 报告里 HiGHS 60 s 的解与两次运行里较好者。
- 之前那 1 个无方案是 `sf_phase4_sloop20_bld`(Somersloop 预算)。`fx_sci6_mod_b4_pow` 这次基线碰巧有方案(报告里两次都没有),新代码不再依赖 hint 是否被采纳。
- 之前超时的 62 个场景,首个方案中位 9.8 ms、最大 279 ms 就有了(原生)。浏览器实测见 §3。
- 平均差距基本没变:新代码多了 Somersloop Buildings 这一个有方案但差距大的场景(+8 %)计入平均。

## 2. 代表场景

| 场景 | 之前 | 之后 | 之后首个方案 (ms) |
|---|---|---|---|
| sf_hmf7_alt_bld | 42.64, 4007 ms | 42.64, 4000 ms | 3.5 |
| sf_pasta_alt_bld | 55.28, 4006 ms | 55.28, 4000 ms | 4.9 |
| sf_phase4_sloop20_bld | **无方案** | 156.72(最好已知 145.04) | 91 |
| sf_phase4_sloop20_res | 5275.20 | 5724.85(**+8.5 %,变差**) | 101 |
| fx_sci6_mod_b4_pow(2439 列) | 387.53 | 387.53 | 79 |
| dsp_um60_plain_bld | 220.12, 4 s 未证明 | 220.12 ✔ 6 ms | 1.7 |
| dsp_multi_plain_bld | 294.85, 4 s 未证明 | 294.85 ✔ 6 ms | 1.8 |
| dsp_um60_pall_cmp | 177.62 | 175.49 | 29 |
| fa_sci7_mod_b8_pow | 605.21 | 571.72(−5.5 %) | 16 |
| fa_sci6_plain_bld | 632.12 | 631.74 | 2.0 |

逐场景全表:`impl_table_full.md`。

**变差的一个**:`sf_phase4_sloop20_res` 5275(基线这次运气好;报告里同一模型多数轮次是 5443)→ 5725。原因是起点不同:新的预算保底方案(5 % 以内的 6239)比旧流程的起点好,但 B&B 之后走了另一条路,4 s 内没找到 5443。结果是确定的(重复 3 次相同),8 s 时 5487。没有找到不伤害其他场景的修法,留作已知问题。

## 3. 浏览器实测(Chromium,vite preview,WASM opt-level 3)

`bench/live_test.py`,日志 `shots/live_log.json`,截图 `shots/*.png`。

| 流程 | 之前 | 之后 |
|---|---|---|
| Satisfactory Buildings,全替代,太空电梯 5–9 + 超级计算机 + 涡轮马达 各 5/min,4 s | 4 s 后才出现方案(报超时) | 0.35–0.40 s 出现首个方案(717 栋,差距 2.6 %),0.5 s 改进到 716 栋,差距随下界上升 2.4 → 1.7 %;4.3 s 结束,蓝色提示"Best plan found in 4 s: within 1.7 %" |
| 同上,1.5 s 时点 Stop | Cancel 会丢掉进行中的求解 | 方案保留(716 栋,1.8 %);"Keep solving (+10 s)" 从同一棵树继续,12.2 s 时 1.6 % |
| Somersloop 预算 20,阶段 4 全替代,Buildings | "Time limit reached before a whole-building plan was found" | 0.32 s 出方案,147 栋、20 个 Somersloop(未超预算),4 s 时差距 27 % |
| DSP Compute 目标,宇宙矩阵 60/min | 4 s 后出现方案 | 0.24 s 出现方案,0.55 s 内改进两次,4 s 时差距 1.0 % |

## 4. 构建选项 opt-level(第 1 步)

Node 26,WASM,各 2 轮(`data/impl_wasm_opt_s_vs_3.txt`):

| | s | 3 | 倍率 |
|---|---|---|---|
| RIP 全替代 Buildings | 1169–1386 ms | 1128–1159 ms | 1.04–1.20 |
| RIP 全替代 Power | 793–920 ms | 592–650 ms | 1.34–1.42 |
| HMF 全替代 Power | 542–611 ms | 442–478 ms | 1.23–1.28 |
| Space Age 大模型 LP | 80 ms | 57–70 ms | 1.15–1.41 |
| `.wasm` 大小 | 1 876 002 B(gzip 441 KB) | 2 068 757 B(gzip 505 KB) | +193 KB(gzip +64 KB) |

报告的 "+1.2 MB" 没有复现(当时可能没有跑 wasm-opt);实际增量 +193 KB 未压缩。已改为 `opt-level = 3`。

## 5. 做了什么

- **第 0 步 永远返回手里最好的方案**:`lp::Run` 持有最好方案;LP 解完即有向上取整方案;每个重加权 LP 的取整方案、受限 MILP 和全量 MILP 的每个 incumbent 都参与比较(先检查预算)。MILP 首次调用用 `node_limit = 0`、不设时限,让 hint 验证跑完;hint 只给整数变量(建筑数、shard),连续变量由 microlp 补全。预算保底:shard 预算用"不超频"取整;Somersloop 预算把占预算的列上限设为 ⌊LP 栋数⌋ 再把剩余预算按小数部分分给最大的几列,重解 LP 后取整,三种上限(贪心 / 全向下取整 / 0)取最好。LP 超过预算 1/10 时跳过重加权。
- **第 2 步 建模期化简(精确)**:`power_exp = 1` 且不超频的列不建功耗变量;删同配方里被支配的列(只在整栋模式)。为避免方案总显示最高等级机器,解完后在"同栋数、同目标权重"的情况下换回耗电最少的等级(`detier`),目标值不变。注意:不同的方案本身会换等级,例如 `fa_sci6_plain_bld` 新方案 116 MW(旧 84 MW),Buildings 目标下功耗权重为 0。
- **第 3 步 分段求解 + 逐步返回**:`wasm_api.rs` 的 `solve_start / solve_step / solve_extend`;Worker 每 150 ms 一步并推送;界面在求解中显示"best plan so far, within x %",Stop 保留方案,超时 / 停止后显示蓝色提示和 "Keep solving"(续同一棵树)。没有方案时的红色超时提示保留。
- **测试**:`src/planner/solve_tests.rs` 6 个(化简前后最优值相同:9 个 MILP 都被两边证明 + 2 个大 LP;DSP Buildings 可证明;Somersloop / shard 预算有方案;2439 列模型 200 ms 也有方案;分段求解单调且 extend 续跑)。`cargo test -q` 63 通过。
- **局部搜索(可选的第 4 步)没有做**。

---

# HiGHS(WASM)做整栋 MILP 引擎(报告 §6 第 5 步,Xinyu 2026-11-22 批准)

## 做法

- npm `highs@1.15.4`(lovasoa/highs-js,MIT,2026-10-16 发布,满两周),只在整栋求解需要时动态加载:`dist/assets/highs-*.js` 101 KB + `highs-*.wasm` 3 531 401 B(gzip 1.22 MB)。不选 HiGHS 或分数建筑时不下载。
- microlp 仍做 LP、首个方案、重加权 LP 和受限 MILP;之后 Worker 把**化简后**的模型(`solve_model_lp`,CPLEX LP 文本)交给 HiGHS,用手里最好的方案做 MIP 初始解(`setSolution`),时限 = 剩余时间,`mip_rel_gap` 1e-4。HiGHS 的每个新 incumbent(`mipImprovingSolution` 回调)和下界(`mipLogging`)回到 Rust 的 `offer_values`:先逐行、逐界检查,再检查 Somersloop / shard 预算,更便宜才采用。所以 HiGHS 更差、出错或超时无解时,界面上始终是 microlp 的方案。
- 设置 "Solver engine: Auto / microlp / HiGHS"(侧栏,整栋模式下)。根据下表,**Auto = HiGHS**。
- HiGHS 运行时 Worker 收不到消息:此时点 Stop 会结束 Worker(方案已在界面上保留);"Keep solving" 发现搜索已不在,就用"时限 + 一份"重新求解,并且不会用更差的方案替换保留的方案。

## 浏览器实测(Chromium headless,真实 Worker + WASM,4 s,报告的 105 个可行场景)

驱动:`bench/browser_bench.py` + 开发用页面 `react-frontend/bench.html` / `src/bench.ts`;数据 `data/browser_microlp4s.jsonl`、`data/browser_highs4s.jsonl`;逐场景表 `browser_table.md`。

| 引擎 | 有方案 | 4 s 内证明 | 与已知最好解差距 中位 | 平均 | >1 % | 首个方案 中位 / 最大 | HiGHS 加载 |
|---|---|---|---|---|---|---|---|
| microlp | 105/105 | 45 | +0.95 % | +2.68 % | 30 | 10 / 329 ms | – |
| HiGHS(Auto) | 105/105 | **63** | **+0.06 %** | **+1.22 %** | **14** | 8 / 341 ms | 65 ms(本机,冷启动,不含网络) |

- 41 个场景 HiGHS 更好(例:Factorio 7 包 beacon 8 Buildings −7.9 %,Space Age 6 包 beacon 8 Power −8.1 %,核子面 全替代 Buildings −5.7 %,Somersloop 预算 Buildings 156.7 → 148.8),**1 个更差**:`sf_phase4_sloop20_res` 5725 → 6239(+9.0 %)。这一例 HiGHS 4 s 内没有改进保底方案,而 microlp 的全量 B&B 碰巧找到了 5725。
- 加载时间 65 ms 是本机 localhost 冷启动(浏览器新配置,无缓存)的编译 + 实例化;线上还要加 1.22 MB(gzip)的下载。加载与 microlp 的前几步并行,不推迟首个方案。

## 浏览器手动流程(Auto = HiGHS,`shots/highs/`)

| 流程 | 结果 |
|---|---|
| 太空电梯 5–9 等 7 个目标,Buildings,全替代,4 s | 0.37 s 首个方案(2.6 %),HiGHS 接手后 1.8 s 降到 1 %,4 s 结束 0.95 %(microlp 同场景 1.7 %) |
| 同上 10 s,1.5 s 时 Stop(HiGHS 正在跑) | 保留 1 % 的方案;Keep solving 重新求解(20 s),界面一直显示保留的方案,直到新运行找到更好的(0.7 %) |
| Somersloop 预算 20,Buildings | 0.31 s 首个方案;4 s 时 139 栋、20 个 Somersloop、差距 10 %(microlp:147 栋、27 %) |
| DSP Compute | 0.21 s 首个方案,0.56 s **证明最优**(microlp 4 s 仍差 1 %) |

---

# 回归修复(2026-11-22):Somersloop 预算 Resources

**原因**(原生实验,逐个开关验证):预算保底方案(6239)被当作全量 MILP 的 warm start。microlp 从它出发 4 s 只找到 5725;不给起点(058f17c 的行为:它的 hint 因超预算被拒)能找到 5275。另外保底方案的配方被加进受限 MILP 的邻域,让受限 MILP 白跑 1 s。删支配列、分段续跑都不是原因(Satisfactory 没有被删的列;不分段的 `plan()` 同样复现)。

**修法**(`lp.rs`,`fallback_version`):最好方案还是预算保底时,它只留在手里显示,不当全量 MILP 的起点,也不扩大受限 MILP 的邻域。

**Auto**:改成两个 Worker 同时跑 microlp 和 HiGHS,界面每时每刻显示两者中较好的方案,任何一方证明最优就结束两者。所以 Auto 不会比只用 microlp 差。Keep solving 时,已经停掉的一方的方案仍参与比较。

## 复测

原生 4 s,105 个场景(`data/impl_after_fix4s.jsonl`,`bench/regressions.py`):**相对 058f17c 基线,差 0.1 % 以上的场景:0 个**。

| | 有方案 | 4 s 内证明 | 差距中位 | 平均 | >1 % | 首个方案中位 |
|---|---|---|---|---|---|---|
| 基线 058f17c | 104 | 43 | +1.00 % | +2.67 % | 31 | 4 s |
| 修复后 | 105 | 47 | +0.93 % | +2.46 % | 29 | 5 ms |

Somersloop Resources 原生:基线 3 次都是 5275;修复后 5275 / 5251 / 5309(分段计时的抖动,±0.6 %)。

浏览器 4 s(`data/browser_microlp4s.jsonl`、`data/browser_auto4s.jsonl`,`browser_table.md`):**Auto 比 microlp 差 0.1 % 以上的场景:0 个**。

| 引擎 | 有方案 | 4 s 内证明 | 差距中位 | 平均 | >1 % |
|---|---|---|---|---|---|
| microlp | 105 | 45 | +0.95 % | +2.55 % | 30 |
| HiGHS 单独(修复前测) | 105 | 63 | +0.06 % | +1.22 % | 14 |
| **Auto(两者并行)** | 105 | **63** | +0.11 % | **+1.01 %** | 15 |

浏览器手动复查(`shots/regression-fix/`):Somersloop 预算 Resources,Auto 与 microlp 都是 5309.06(HiGHS 单独时是 6239);太空电梯计划 Stop 后保留 1 % 的方案,Keep solving 期间和结束时都不会换成更差的方案;Somersloop Buildings、DSP Compute 正常。
