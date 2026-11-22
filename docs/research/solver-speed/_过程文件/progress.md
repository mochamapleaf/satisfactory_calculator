# solver-speed 进度记录

## 步骤 1 (2026-11-21) 读现有资料 + microlp 源码
- 完成: 读 docs/PROGRESS.md、fable-review-2/3(仅 M5 提到 MILP 时限使测试变慢,无 presolve 相关已知结论)、src/planner/lp.rs 全文、rules.rs、mod.rs。
- 完成: 读 microlp 0.8.0 源码(~/.cargo/registry/.../microlp-0.8.0/src/presolve, src/mip)。
  - presolve 只做"界"类约简: 整数界取整、singleton 行→界、forcing 行固定整数、冗余行删除、big-M 系数收紧、dual fixing。不删列、不做链式消元/doubleton 聚合、不做 dominated column、无割平面。
  - dual fixing 在给了 warm_start 时被关闭(mip/mod.rs build_state: `options.dual_fixing && options.warm_start.is_none()`),而 lp.rs 总是给 hint。
  - B&B: pseudocost 分支 + 下潜(DFS)后 best-bound;无 rounding/diving 启发式,incumbent 只来自 warm start 或节点 LP 恰好整数。
  - 支持 resume()/node_limit → 可以分段返回 best-so-far。
- 文件: 无改动(只读)。
- 下一步: 把 crate 拷到 scratchpad/bench,加基准 bin,测各游戏预设的模型规模与耗时。

## 步骤 2 (2026-11-21) 基准程序
- 完成: 把 crate 快照(= commit 058f17c 的 src/、data/)拷到 scratchpad/bench(仓库 src/ 未改动);
  用 patch_lp.py 给快照的 lp.rs 加了实验开关 Tune(gap / nosub / nohint / nopresolve / slice(resume 分段) / dominated / tighten / lattice / scont / dump .lp)和 Trace(模型规模、各阶段时间、节点数、界、轨迹)。
- 文件(会随报告拷进 _过程文件/): bench/examples/bench.rs(120 个场景: Satisfactory 42, DSP 18, Factorio 60)、patch_lp.py、run_highs.py。
- 抽查: sf_rip10 / sf_hmf7 12 个场景中仅 sf_hmf7_alt_bld(HMF 7.3/min, 全替代配方, Buildings 目标)4 s 未证明(gap 8.2%, 24858 节点);无替代配方的链 0 节点即证明(microlp presolve 已把 n 下界收紧到 ceil)。
- 进行中: 全部 120 场景 4 s 基线(out/base4s.jsonl)。
- 下一步: 汇总哪些超时 → 变体对比 → HiGHS 对照 → WASM/native 比例。

## 步骤 3 (2026-11-21) 4 s 基线结果(native release, opt-level=s, 单线程)
- 完成: 120 场景 4 s 基线 → scratchpad/bench/out/base4s.jsonl(稍后拷到 _过程文件/data/)。15 个 fx_sci7(太空科技包, 默认只选 Nauvis)本来就不可行,不计。
- 结果(105 个可行场景): 62 个 4 s 内未证明最优。
  - Resources 目标: 基本都在 <0.1 s 内于根节点证明(建筑权重只有 0.01);例外是带 beacon 的 Factorio(gap 0.011–0.023%,刚好卡在 1e-4 阈值上)和 Somersloop 预算。
  - Buildings / Power / Compute 目标: 只要有选择(替代配方、机器等级、增产剂模式、插件/beacon)几乎全部超时,gap 1–23%。
  - 多数超时场景里,最终返回的方案 = 受限 MILP 的结果,甚至 = ceil(LP) 方案(ceil%=0);后 3 s 的全量 B&B 基本没有改进 incumbent,只是在抬下界。
  - 两类"连方案都没有"的失败: (1) sf_phase4_sloop20_bld: Somersloop 预算使 ceil 提示不可行,4 s 内没有 incumbent;(2) fx_sci6_mod_b4_pow(2439 列, 14446 行): LP + 4 个重加权 LP 用了 3.8 s(LP 不受时限),受限 MILP 找到的方案(387.53)在最后 MILP 0 节点时被丢弃。
- 进行中: HiGHS(highspy native, 单线程, 60 s)对 62 个未证明模型;microlp 变体(nosub/nohint/nopresolve/dominated/tighten/lattice/scont)对 17 个代表场景。
- HiGHS 初步: DSP 无增产剂的 4 个场景 0.13–0.28 s 根节点证明(microlp 4 s/4 万节点未证明);带增产剂模式的 DSP multi 场景 HiGHS 60 s 也只到 gap 3–5%。
- 下一步: 汇总变体与 HiGHS;加 linpow 变体(power_exp=1 时去掉 p 变量和重复的分段行);WASM/native 比例与 opt-level。

## 步骤 4 (2026-11-21) 变体与 HiGHS 对照(数据已拷到 _过程文件/data/,脚本在 _过程文件/bench/)
- microlp 变体(17 个代表场景, 4 s): 
  - nohint(不给 warm start,让 dual fixing 生效): 10/17 直接"无方案" → microlp 的 B&B 自己几乎找不到 incumbent,warm start 是必须的。
  - nopresolve / tighten(自己做的下界传播) / scont(shard 连续化): 无可见差别。microlp 自带 presolve 对这些模型基本不起作用。
  - dominated(删被支配的机器等级/模式列): dsp_um60_plain_bld 85→41 列,4 s 未证明(4.3 万节点)→ 8 ms 根节点证明;fa_sci6_plain_bld gap 1.6%→0.28%;有增产剂/插件模式的场景无改善。
  - lattice(纯整数目标,界可向上取整): rip10_alt_bld 快 2 倍,其余无改善(差距 > 1 栋)。
  - linpow(power_exp=1 时去掉 p 变量和 5 条重复分段行): 行数 14446→2531,LP 357→77 ms,support 阶段 2.0 s→0.36 s,节点吞吐 3–5 倍;fx_sci6_mod_b4_pow 从"无方案"变为有方案。gap 几乎不变。
- HiGHS(highspy native 单线程, 同一模型 .lp, gap 1e-4, 60 s): 61 个 microlp 未证明的模型中 36 个证明最优,其中 20 个 ≤4 s;microlp 4 s 的方案比 HiGHS 最好解差: 中位 +0.96%,最大 +11.2%,29 个差 >1%。
  即: 超时不只是"证明不出来",返回的方案本身也常常不是最优。
- 进行中: HiGHS 2/4/10 s 限时(看 4 s 内 HiGHS 的 incumbent);microlp 60 s 轨迹。
- 下一步: opt-level=3、WASM/native 比例、resume 分段开销;写报告。

## 步骤 5 (2026-11-21) 余下测量全部完成
- HiGHS 限时 2/4/10 s(native 单线程, 无 warm start): 62 个里 ≤2 s 证明 14 个、≤4 s 20 个、60 s 36 个;4 s 时方案比 microlp 4 s 好的 36 个、差的 4 个(Somersloop 预算三个场景 HiGHS 首个 incumbent 极差,必须保留 microlp 的方案取较好者)。表: data/highs_table.md。
- microlp 60 s 轨迹(data/base60s.jsonl): hmf7_alt_bld 60 s 仍 gap 1.6%;dsp_um60_plain_bld 60 s/25 万节点仍未证明(HiGHS 0.13 s,删支配列后 8 ms)。加时间无用。
- 组合变体(62 个未证明场景, 4 s): dominated+linpow → 证明 2 个,方案质量不变;再加 h1(邻域=LP 用到配方的全部模式)无益;加 ls(“禁一列再解 LP”的局部搜索, 占 40% 预算) → 证明 4 个,相对 HiGHS 60 s 最好解的平均差距 2.54%→1.74%(中位 0.98→0.75%),但 Somersloop 预算场景变差(ceil 方案违反预算,需加预算检查)。表: data/v2_table.md, data/v3_table.md。
- microlp presolve 核对: 无替代配方链上 nopresolve 节点 24/128/285/1122 → presolve 开 0/22/16/48;自己写的下界传播(tighten)在 presolve 关时给出完全相同的节点数 → 用户设想的“确定关系”约简 microlp 已经在做。
- WASM(node 26, 058f17c 原样构建) vs native: 同节点数下慢 1.07–1.45 倍;opt-level=3 的 WASM 比 opt-level=s 快 1.23–1.49 倍,体积 1.81→3.01 MB。resume 分段(50–250 ms)节点数不变,耗时差异在噪声内。
- 结构统计(data/structure.md): 全替代配方时 85% 的配方在同一个强连通分量里;DSP 只有 4% 在环里但照样难 → 难点不在环,在“多选一 + 向上取整”。
- 下一步: 写 00-最终报告.md → critic 审 → 修。

## 步骤 6 (2026-11-21) 报告完成(已按审稿意见改完)
- 完成: docs/research/solver-speed/00-最终报告.md;审稿意见在 _过程文件/00-最终报告.review.md(中间产物),意见已全部并入正文。
- 更正前面步骤里的两处: 步骤 4 的 "nohint 10/17" 应为 9/17;步骤 3 的 fx_sci6_mod_b4_pow "无方案" 的原因不是单纯时间不够,而是 microlp 在时限打断 hint 验证时静默丢弃 hint(mip/search.rs 376 行)——首段 250 ms 时丢失,≥600 ms 或不分段时采纳(data/hint_slice_fx_b4_pow.jsonl)。
- 补存的原始数据(机器空闲时重跑): data/chain_presolve.jsonl、slice_overhead.jsonl、native_opt.jsonl、wasm_opt_s.jsonl、wasm_opt_3.jsonl、var_hintint.jsonl。
- 文件: 只新增 docs/research/solver-speed/ 下的文件;src/、react-frontend/ 未改;未提交、未推送。
- 未做: docs/PROGRESS.md 未加条目(另一个会话正在改仓库);memo 项目日志未写(由调用方记录)。
- 下一步(若采纳): 按报告 §6 的 0→1→2→3 实施;第 5 步(HiGHS)需要 Xinyu 拍板。
