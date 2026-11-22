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
