# 笑笑meow 戴森球 ep18 / ep20 混带与带式布局提取

说明：BV1wxikYXEHp = ep18（优化技巧 / 万糖准备）；BV1iQrKYoEoy = ep20（万糖黄棒）。引用格式 `[BV号 mm:ss]`（mm 可超过 60）。转写为 ASR，疑点标 (ASR?)。"爪子"=分拣器（黄=Mk1，绿=Mk2，蓝=Mk3，白=Mk4/极速）这一对应由 [BV1wxikYXEHp 61:12] 的颜色说法推出，未直接说 Mk 号。

## 第一部分：ep18 BV1wxikYXEHp（优化技巧 万糖准备）

### A. 混带（混合传送带）：目的与做法
- 目的是省分拣器的逻辑运算：做微晶元件的机器原来要两个"爪子"（一边抓硅一边抓铜），改为一条带上放两种物品，只要一个爪子抓混带上的铜和硅。[BV1wxikYXEHp 21:27] [BV1wxikYXEHp 21:55]
- 他的量化：机器原本 3 个爪子（两进一出），混带后省到 2 个爪子，运算量"降低一半"（口头说"可能是从三个爪子降低到两个"）。[BV1wxikYXEHp 22:03] [BV1wxikYXEHp 22:09]
- 第二个收益：传送带本身也要算，两条带合成一条后，带的长度与计算量相应下降。[BV1wxikYXEHp 22:17] [BV1wxikYXEHp 22:28]
- 区分两种混带用途：之前"超市"用混带是为省接线压力，让一条线上有很多种物品方便生产；万糖工厂用混带是为省爪子。[BV1wxikYXEHp 21:34] [BV1wxikYXEHp 21:47]
- 结论：万糖工厂"也是要用混带的"；结尾叮嘱自己搭产线时"每一种东西都要混带"，方法参照"之前用混带生产粒子容器那种方法"。[BV1wxikYXEHp 21:43] [BV1wxikYXEHp 91:50] [BV1wxikYXEHp 91:58]
- 本期未给出混带具体的物品配比、投料位置、环形/单向、防堵细节（只在 ep20 展开，见第二部分）。

### B. 分拣器（爪子）选型与数量（CPU 优化角度）
- 逻辑：全部用白爪（极速分拣器）"不能优化 CPU"。原因：白爪运力无限，需要时只工作一帧，其余帧都"不工作"；他希望爪子"在不停地工作"。[BV1wxikYXEHp 19:21] [BV1wxikYXEHp 20:08] [BV1wxikYXEHp 20:24]
- 例：铁熔炉满速时需铁速率恒定，一个黄爪就能带动，不需要白爪；黄爪来回往返，每烧出一份就塞一份铁。[BV1wxikYXEHp 19:35] [BV1wxikYXEHp 19:42] [BV1wxikYXEHp 19:50]
- 规则：爪子要做成"恰好满足运力需求"，尽量降级到黄爪；黄爪工作时间长，CPU 运算量减小。[BV1wxikYXEHp 20:30] [BV1wxikYXEHp 20:51] [BV1wxikYXEHp 20:54]
- 限制：爪的速度不能调，只能选白/黄爪等整档，所以"恰好满足"是很高的要求。[BV1wxikYXEHp 20:37] [BV1wxikYXEHp 20:39]
- 小太阳的抓取分拣器"最好用黄分拣器"，黄的都"远远过剩"；关心帧数时所有分拣器"能用最低级就用最低级"。[BV1wxikYXEHp 70:21] [BV1wxikYXEHp 70:34] [BV1wxikYXEHp 70:37]
- 实际带的分拣器：黄绿白用得最多（"很少有机器恰好用蓝爪才能运行"）；要出货堆叠功能就用白爪，不要则黄绿爪大概率够用；"大部分进货口其实都是黄绿爪"。[BV1wxikYXEHp 61:12] [BV1wxikYXEHp 61:15] [BV1wxikYXEHp 61:20] [BV1wxikYXEHp 61:28]
  - 即：白爪（Mk4）的用途是产物输出时做堆叠；进料口黄/绿即可。
- 蓝爪只带一箱（300 多个）备用，不进塔。[BV1wxikYXEHp 61:44] [BV1wxikYXEHp 61:59]
- 传送带全用蓝带（Mk3）。[BV1wxikYXEHp 61:40]

### C. 空转机器 / 机器数量
- 绝不留不在工作的机器（堵着的机器每个逻辑帧都尝试重启：检查产物堆积、原料、电力），比在工作的机器多耗很多运算，数量大时"可能十倍甚至更多"。[BV1wxikYXEHp 17:23] [BV1wxikYXEHp 17:41] [BV1wxikYXEHp 18:16] [BV1wxikYXEHp 18:52]
- 凑小数时最后一台机器会时不时不启动，也不要留；"所有的机器都必须精准开工"（万糖目标）。[BV1wxikYXEHp 17:26] [BV1wxikYXEHp 19:02]
- 例子：翘曲器(ASR? "翘取器")产量用量约为白糖的 1/20～1/40，即使卡着也占运算少；他"为了凑整"加了 5 个绿糖研究站（937.5 凑整）。[BV1wxikYXEHp 35:14] [BV1wxikYXEHp 35:20] [BV1wxikYXEHp 35:51]
- 他"习惯做成整数倍的组装机"：黄棒 20 个组装机只有 225/分，不够，做 25 个，约 281/分。[BV1wxikYXEHp 42:11] [BV1wxikYXEHp 42:18] [BV1wxikYXEHp 42:25]

### D. 增产剂（喷涂）策略
- 增产剂选"加速"还是"增产"沿用 B 站大佬帖子，并用其中"最少卡顿"策略（另有"最少占地""最少耗电"版本）；之前的白糖矿物需求表也是按最少卡顿策略。[BV1wxikYXEHp 42:47] [BV1wxikYXEHp 43:00] [BV1wxikYXEHp 43:06] [BV1wxikYXEHp 43:17]
- 黄棒和黑棒的增产机"只能加速"，选加速以减少机器数量。[BV1wxikYXEHp 42:31] [BV1wxikYXEHp 42:35]
- 高效粒子容器配方采用了"加速"配置的假设，消耗单极磁石约 1.25 倍；他改回"增产"以省矿（矿物总量指数上升、开采速度线性上升，所以磁石消耗是更硬的约束）。[BV1wxikYXEHp 43:40] [BV1wxikYXEHp 44:12] [BV1wxikYXEHp 44:41] [BV1wxikYXEHp 44:46]
- 增产机(蓝糖，"蓝机" ASR?)就地生产：每个工业星自己产增产剂，因为外接要再做碳纳米管外接产线太麻烦。[BV1wxikYXEHp 37:53] [BV1wxikYXEHp 38:04] [BV1wxikYXEHp 38:10]
- 喷涂机用量很少，直接带走放背包左侧。[BV1wxikYXEHp 64:25] [BV1wxikYXEHp 64:31]
- 增产剂缺货的失效模式：缺货期间新产出的增产剂被别的东西抢走，没有回供自己的原料，于是持续不足，白糖减产；比黄棒问题小。[BV1wxikYXEHp 39:16] [BV1wxikYXEHp 39:22] [BV1wxikYXEHp 39:33] [BV1wxikYXEHp 39:37]

### E. 超市 / 堆叠
- 超市出货按四叠算，其他产线按三叠算；升到三叠已可跑满 450（超市）。[BV1wxikYXEHp 02:46] [BV1wxikYXEHp 02:52] [BV1wxikYXEHp 02:56]
- 750 白糖厂已开机；量化单位为每分钟。[BV1wxikYXEHp 02:37]

### F. 一屋一塔（一物一塔）与物流塔设置
- 万糖继续用"一物一塔/一屋一塔"，不用黑盒：多占一点算力但不多；好 debug（总控面板看缺什么直奔该工厂）；好设计（不用纠结挤占地，占地只通过传送带长度影响算力，影响很小）。[BV1wxikYXEHp 28:17] [BV1wxikYXEHp 28:30] [BV1wxikYXEHp 28:36] [BV1wxikYXEHp 29:02] [BV1wxikYXEHp 29:10]
- 目标风格"跟现在 750 差不多的一屋一塔"，"hopefully"能带万糖。[BV1wxikYXEHp 29:18] [BV1wxikYXEHp 29:21]
- 物流塔出货集装数量改成 1 可让小太阳更快点亮；因为要让塔里黄棒攒起来需要把很长的传送带填满。[BV1wxikYXEHp 70:44] [BV1wxikYXEHp 71:35] [BV1wxikYXEHp 71:47]
- 小太阳每台启动烧掉 1 个并缓存 10 个，共吃 11 个黄棒。[BV1wxikYXEHp 71:19] [BV1wxikYXEHp 71:24]
- 物流塔优先级：只要把矿星塔与工业星所有塔调到同一优先级组即可；黄棒、蓝糖(增产机)的星让其所有矿的优先级高于其他星，就不会出现用完的问题。[BV1wxikYXEHp 68:03] [BV1wxikYXEHp 68:09] [BV1wxikYXEHp 40:03] [BV1wxikYXEHp 40:08]
- 无优先级时的失效：矿短缺时铁被其他产线抢走，黄棒线停，黄棒越来越少，最终整颗星停电，需从外部输入黄棒重启。[BV1wxikYXEHp 38:51] [BV1wxikYXEHp 39:04] [BV1wxikYXEHp 39:42] [BV1wxikYXEHp 39:45]
- 一屋一塔下"确实不太好控制优先级"；只要保证原矿供应充足就不会死锁（自产自消）。[BV1wxikYXEHp 38:55] [BV1wxikYXEHp 40:48] [BV1wxikYXEHp 40:50]
- 每个拉矿塔放 5 种不同的矿，且所有塔的设置必须完全一样；否则会出现某塔满了、另一塔只运铜铁而光山石没人运的情况（游戏不会自动优化）。[BV1wxikYXEHp 78:35] [BV1wxikYXEHp 79:35] [BV1wxikYXEHp 79:45] [BV1wxikYXEHp 80:23] [BV1wxikYXEHp 80:35]
- 本地塔与星际塔逻辑：通常"本地供应、星际需求"；若本星仍是矿星，会出现"左手倒右手"（外星矿进塔又被本地塔送出），解决：改成"本地仓储"，用个别塔设"本地需求"去消耗本地剩余矿。[BV1wxikYXEHp 82:26] [BV1wxikYXEHp 82:55] [BV1wxikYXEHp 83:02] [BV1wxikYXEHp 83:13]
- 物流塔间距：蓝图两边传送带刚好重叠 1 格就是最小间距，重叠 2 格会报警。[BV1wxikYXEHp 81:15] [BV1wxikYXEHp 81:18] [BV1wxikYXEHp 81:23]

### G. 数字（带速/运力/塔数）
- 运力：7 光年内一个塔 = 4 条蓝带 = 4×7200（"7200 是一个四叠的蓝带"，即每分钟 7200）；12 光年 = 3 条蓝带；22 光年 = 2 条蓝带(原话仅说"再下一档是二十二光年"，条数未直说)。[BV1wxikYXEHp 76:59] [BV1wxikYXEHp 77:03] [BV1wxikYXEHp 77:06] [BV1wxikYXEHp 77:31] [BV1wxikYXEHp 77:33] [BV1wxikYXEHp 77:58]
- 算塔数的方法：把铁/硅/煤/钛/铜五种主矿每分钟需求相加，除以 (7200×3)，得拉主矿的塔"恰好需要 50 个"；最终 26+26=52 个。[BV1wxikYXEHp 78:09] [BV1wxikYXEHp 78:18] [BV1wxikYXEHp 78:20] [BV1wxikYXEHp 82:06]
- 氢(ASR? 原文"青")三塔供应：整个结构只有 5 条蓝带 = 5×7200，星总吞吐 223401(ASR?)/7200/5 ≈ 6.2 组；另一个约束为轨道采集器拉取量 16956 除以大塔运力(3×7200)，约 5.4 个；最终放 7 组。[BV1wxikYXEHp 85:33] [BV1wxikYXEHp 85:35] [BV1wxikYXEHp 85:44] [BV1wxikYXEHp 86:29] [BV1wxikYXEHp 86:45] [BV1wxikYXEHp 86:50]
- 物流塔充电功率默认 60（MW），拉满到 300MW 后充电仍是运输船瓶颈；塔运力随距离下降。[BV1wxikYXEHp 76:15] [BV1wxikYXEHp 76:17] [BV1wxikYXEHp 76:20] [BV1wxikYXEHp 65:08]
- 黄棒：1 个 = 72 GJ，1 个/分 ≈ 1 GW，300 GW ≈ 300 个/分；黄棒线 45/分的早期量。[BV1wxikYXEHp 41:16] [BV1wxikYXEHp 41:34] [BV1wxikYXEHp 41:38] [BV1wxikYXEHp 42:05] [BV1wxikYXEHp 66:17]
- 引力透镜：每个光子锅消耗 0.1/分；放透镜工作效率是不放的 4 倍（2700→675 锅）；675×0.1=67.5/分；考虑发电再翻倍 → 135(原文先说"1350"后改回，ASR?)。[BV1wxikYXEHp 36:35] [BV1wxikYXEHp 36:40] [BV1wxikYXEHp 36:54] [BV1wxikYXEHp 36:55] [BV1wxikYXEHp 37:33] [BV1wxikYXEHp 41:02]
- 规模：2 万糖/星，复制 5 份 = 10 万糖；分到多颗星以利用多核 CPU（他分 5 颗，每颗 20250(ASR?)），此说法他自称"道听途说"。[BV1wxikYXEHp 31:56] [BV1wxikYXEHp 31:58] [BV1wxikYXEHp 34:00] [BV1wxikYXEHp 34:49] [BV1wxikYXEHp 34:50]
- 设备需求：组装机 4300、熔炉 3410；每熔炉约 15 磁石(ASR?)。[BV1wxikYXEHp 45:19] [BV1wxikYXEHp 45:21] [BV1wxikYXEHp 46:02]

### H. 其他 CPU 优化提示（与带无直接关系）
- 优先级：战斗系统可关黑雾养殖场(但小太阳要多)；戴森云吸附完后关掉发射；星系内物流最耗 CPU，万糖矿别从本星系其他星拉；少放大塔；行星内物流最省算力。[BV1wxikYXEHp 14:19] [BV1wxikYXEHp 14:23] [BV1wxikYXEHp 23:18] [BV1wxikYXEHp 25:07] [BV1wxikYXEHp 25:52] [BV1wxikYXEHp 28:01]
- 小太阳阵列：每行小太阳放一行传送带（两排共用一行会出现机械臂冲突、需偏移），他选择多放传送带，"可能多占一点性能"。[BV1wxikYXEHp 72:46] [BV1wxikYXEHp 73:20] [BV1wxikYXEHp 73:27] [BV1wxikYXEHp 73:37] [BV1wxikYXEHp 73:39]
- 增产产线之外，产线按三叠、超市四叠已提；翘曲器带用"一条二层的传送带"一直拉到。[BV1wxikYXEHp 75:16]


## 第二部分：ep20 BV1iQrKYoEoy（万糖黄棒）

这一期是混带细节最集中的一期。术语：他区分"混带控制单元"（环带 + 摇匀 + 直怼补料）和"梯口投料"（原文"剃口/梯口/提口"，ASR?，即投料点后移一格的旧式投料）；环带 = 循环传送带（回流）。"黄点"= 机器上的黄色警告点（未满速/缺料）。"滤计"= (ASR?) 某一中间产物产线名，原文反复出现，含义不确定。

### A. 带速/分拣器数值基准（他口述）
- 蓝带 = 30 物品/秒；绿带 = 12/秒；黄带 = 6/秒；蓝带是绿带的 2.5 倍。[BV1iQrKYoEoy 40:28] [BV1iQrKYoEoy 19:06] [BV1iQrKYoEoy 13:53] [BV1iQrKYoEoy 41:00]
- 绿爪 = 3/秒；黄爪 = 1.5/秒（由"黄带加黄爪就是四分之一"推出：6+1.5=7.5=30/4；原话没有直接说 1.5）；两格距离的黄爪 = 0.75/秒。[BV1iQrKYoEoy 19:08] [BV1iQrKYoEoy 20:24] [BV1iQrKYoEoy 41:03]
- 绿带+绿爪 = 12+3 = 15 = 二分之一蓝带，是常用投料比例。[BV1iQrKYoEoy 15:56] [BV1iQrKYoEoy 19:06] [BV1iQrKYoEoy 19:10]
- 游戏的基本单位是 1.5 或 0.75，所以无法精确投出 10/秒（三分之一蓝带），退而求其次每种投 9.75，三种共 29.25（不超 30）。[BV1iQrKYoEoy 40:28] [BV1iQrKYoEoy 40:34] [BV1iQrKYoEoy 40:45] [BV1iQrKYoEoy 40:49]
  - 9.75 = 黄带 6 + 绿爪 3 + 两格黄爪 0.75。[BV1iQrKYoEoy 40:57] [BV1iQrKYoEoy 41:06]
- 钛合金投料口的流量"精确的 5760"（应为每分钟，即某种物料一个投料口的精确流量）。[BV1iQrKYoEoy 14:31]

### B. 混带控制单元（环带 + 回流）的结构与原理
- 结构：一个混带控制单元能带很多组装机，其余补料只用传送带"直接往上怼"；环带把未被抓走的物品回流再循环。[BV1iQrKYoEoy 08:15] [BV1iQrKYoEoy 08:19] [BV1iQrKYoEoy 04:21]
- 直怼补料的前提约束：回料的理论流量必须 ≤ 投料出去的理论流量，否则回料物品会堵在传送带后面，把前面所有比例打乱。[BV1iQrKYoEoy 27:50] [BV1iQrKYoEoy 27:54] [BV1iQrKYoEoy 28:02] [BV1iQrKYoEoy 28:05]
- 完整周期时投料口不工作，只在时间上做摇匀；若最后一个周期做成不完整（机器被截断），必须算回流是否 ≤ 投料，否则要多加机器。[BV1iQrKYoEoy 28:08] [BV1iQrKYoEoy 28:16] [BV1iQrKYoEoy 28:23] [BV1iQrKYoEoy 28:27]
- 控制单元带机器数示例：电动机一满蓝带恰好 10 台组装机，铁板投 1/2、齿轮 1/4、线圈 1/4。[BV1iQrKYoEoy 18:32] [BV1iQrKYoEoy 18:36] [BV1iQrKYoEoy 18:37] [BV1iQrKYoEoy 18:44]
- 黑棒配方 24 秒消耗 26 个物品，一个满蓝带理论只能带 15 台；他每列造 13 台（给投料口留位置），6 列共 78 台，实际需要 75 台，多出的不开工，拆 3 台也行。[BV1iQrKYoEoy 70:51] [BV1iQrKYoEoy 70:54] [BV1iQrKYoEoy 71:28] [BV1iQrKYoEoy 71:31] [BV1iQrKYoEoy 71:38] [BV1iQrKYoEoy 71:44]
- 两个环带共用同一个投料塔的出货带，因为四个物品任何一个都占不到半条蓝带（用量最高的氢+反物质也只接近半条）。[BV1iQrKYoEoy 71:56] [BV1iQrKYoEoy 72:02] [BV1iQrKYoEoy 72:05]
- 补料口可放在侧面；但"回收铁板后一定要在同一个传送带区间内把它投回去"，否则中间的组装机吃不到铁板（他称低级错误）。[BV1iQrKYoEoy 21:00] [BV1iQrKYoEoy 21:09] [BV1iQrKYoEoy 21:13] [BV1iQrKYoEoy 21:20]
- 三个投料口在环带上位置任意（铁板投料口搬到对面也没问题，它的唯一作用是让环带上有足够铁板）。靠近放的好处：靠近投料口前面的组装机需要白爪防漏料，集中放可少用白爪（白爪性能比蓝爪差）。[BV1iQrKYoEoy 21:28] [BV1iQrKYoEoy 21:31] [BV1iQrKYoEoy 21:49] [BV1iQrKYoEoy 21:55] [BV1iQrKYoEoy 21:58] [BV1iQrKYoEoy 22:00]
- 分开放并精确计算的好处：三个投料口加起来虽是满带，但带上任何位置都不是满带（前面的机器已消耗一部分），所以投料周期可覆盖 12 台而非 10 台；条件是整个环带周期上没有任何一点流量超过蓝带。电动机线没这么做，因为十台一组刚好可放三组。[BV1iQrKYoEoy 22:19] [BV1iQrKYoEoy 22:26] [BV1iQrKYoEoy 22:43] [BV1iQrKYoEoy 22:50] [BV1iQrKYoEoy 22:59] [BV1iQrKYoEoy 23:11] [BV1iQrKYoEoy 23:30]
- 摇匀器（ASR? "摇云器"）只能摇到五分之一的倍数，所以 1:1 配方的绿马达摇成 3:2（电动机 3 : 线圈 2）。[BV1iQrKYoEoy 26:33] [BV1iQrKYoEoy 26:37] [BV1iQrKYoEoy 26:40] [BV1iQrKYoEoy 26:43]
- 绿马达周期设计：过一台补一次电动机；每过 10 台补一次线圈，再过 10 台补一次马达（原文口误后更正），周期 20 台；摆四排共 60 台 = 3 个周期。也可改成每 12 台补一次、周期 24 台，但 15 与 24 的最小公倍数太大，一个控制单元要带太多机器，可能带不动，故不采用。[BV1iQrKYoEoy 26:46] [BV1iQrKYoEoy 26:50] [BV1iQrKYoEoy 26:55] [BV1iQrKYoEoy 27:01] [BV1iQrKYoEoy 27:03] [BV1iQrKYoEoy 27:13] [BV1iQrKYoEoy 27:18] [BV1iQrKYoEoy 27:22]
- 绿马达共需 184 台：3 组 ×60 + 4 台；多出的 4 台只能放在"进投料单元前面的最后一个位置"（不影响前面比例），他最终只放 1 台，共 183 台。[BV1iQrKYoEoy 38:48] [BV1iQrKYoEoy 38:51] [BV1iQrKYoEoy 39:07] [BV1iQrKYoEoy 39:17] [BV1iQrKYoEoy 39:31] [BV1iQrKYoEoy 39:36]
- 电路板配方与磁线圈完全一样（可直接复制蓝图改材料）；补料沿用"滤计"(ASR?)那套：隔三台补铁板、隔两台补铜板、隔四台补铁板进入循环；回料带需要两条（一条带不动）。[BV1iQrKYoEoy 53:51] [BV1iQrKYoEoy 54:05] [BV1iQrKYoEoy 54:08] [BV1iQrKYoEoy 54:12] [BV1iQrKYoEoy 54:15]
- 线圈产线：每秒出 2 个（"滤计"每秒出 1 个），补料位置与"滤计"那边一致；出货侧放两条带，一半机器走一条、另一半走另一条；一个塔接 12 个口，运力压力约 6 条满带。[BV1iQrKYoEoy 25:08] [BV1iQrKYoEoy 25:15] [BV1iQrKYoEoy 25:17] [BV1iQrKYoEoy 25:20] [BV1iQrKYoEoy 25:28] [BV1iQrKYoEoy 25:34]
- 微晶元件：制造时间是"滤计"的一半，产线本质是"滤计"的产线翻倍：第一次补料隔六台，之后隔四台、隔八台；因配方一样但翻倍，两排组装机可共用一条回料带（回料放中间，进货放两侧）。[BV1iQrKYoEoy 54:41] [BV1iQrKYoEoy 54:47] [BV1iQrKYoEoy 54:54] [BV1iQrKYoEoy 54:57] [BV1iQrKYoEoy 55:10] [BV1iQrKYoEoy 55:14] [BV1iQrKYoEoy 55:17]
- 处理器（CPU，3 秒两个电路板+两个微晶元件，1:1）摇成 3:2，隔七个补一次（"七八九都无所谓"，隔十个补时恰为零），一排只放 14 台使周期好算；只有最后两个要白爪，前面都可黄爪。[BV1iQrKYoEoy 56:00] [BV1iQrKYoEoy 56:03] [BV1iQrKYoEoy 56:08] [BV1iQrKYoEoy 56:12] [BV1iQrKYoEoy 56:18] [BV1iQrKYoEoy 56:21] [BV1iQrKYoEoy 56:23] [BV1iQrKYoEoy 56:36] [BV1iQrKYoEoy 56:38]
- 湮灭约束球（1:1 配方、加工极慢）：随便一个补料口就能带很多机器，各用一个绿带投进去，后面加缓冲（对这条线加不加无所谓，因为撞住也不出问题）；一个环带转一圈又一圈，中间两条带回湮灭约束球，所有位置黄爪进黄爪出。[BV1iQrKYoEoy 58:28] [BV1iQrKYoEoy 58:33] [BV1iQrKYoEoy 58:41] [BV1iQrKYoEoy 58:43] [BV1iQrKYoEoy 58:45] [BV1iQrKYoEoy 58:59] [BV1iQrKYoEoy 59:05]
- 框架材料：投料两个（一绿两黄），占五分之四条带，五台组装机绰绰有余。[BV1iQrKYoEoy 59:31] [BV1iQrKYoEoy 59:34] [BV1iQrKYoEoy 59:37] [BV1iQrKYoEoy 59:42]

### C. 梯口（缓冲格）投料规则
- 关键规则：每个投料口后面必须有至少一格缓存（一格蓝带可放 2～3 个物品），否则回流物品会撞到投料物品，被撞者被卡一个逻辑帧，流量低于理论值。"缓存空间可以很小但一定要有。" [BV1iQrKYoEoy 13:07] [BV1iQrKYoEoy 13:21] [BV1iQrKYoEoy 13:29] [BV1iQrKYoEoy 17:00] [BV1iQrKYoEoy 17:07] [BV1iQrKYoEoy 17:34] [BV1iQrKYoEoy 19:36] [BV1iQrKYoEoy 19:41] [BV1iQrKYoEoy 66:09]
- 机理：梯口就是把投料点往后移一格，黄带后多一格蓝带缓冲；这格黄带无论如何不会被卡死。[BV1iQrKYoEoy 16:51] [BV1iQrKYoEoy 16:56] [BV1iQrKYoEoy 17:00] [BV1iQrKYoEoy 17:05]
- 为什么绿带直投能正常：绿带投料间距均匀（蓝带是绿带 2.5 倍，五格一周期），一次相位矫正后永远不再撞；该投料口的流量精确、不缺料。[BV1iQrKYoEoy 13:47] [BV1iQrKYoEoy 13:57] [BV1iQrKYoEoy 14:07] [BV1iQrKYoEoy 14:18] [BV1iQrKYoEoy 14:27] [BV1iQrKYoEoy 14:33]
- 直怼失效的两种情况：(1) 某物品投多了产生持续回流：回流物品的抓取不可预测、没有周期性，持续撞新投料物品，被撞物品持续变少，最后一台机器吃不满；他说"硫酸多了没问题，钢材多了会出问题"。(2) 用"带+爪"拼出半带（如绿带 12 + 绿爪 3），统计上是二分之一但并非精确隔一格，周期很长，撞上后被往后推而达不到满速。[BV1iQrKYoEoy 14:40] [BV1iQrKYoEoy 14:44] [BV1iQrKYoEoy 14:52] [BV1iQrKYoEoy 14:54] [BV1iQrKYoEoy 15:03] [BV1iQrKYoEoy 15:19] [BV1iQrKYoEoy 15:45] [BV1iQrKYoEoy 15:50] [BV1iQrKYoEoy 15:56] [BV1iQrKYoEoy 16:26] [BV1iQrKYoEoy 16:40]
- 带+爪叠加的前提：爪必须在带前面（先爪后带限速），反过来爪贡献不了速度；绿爪与绿带两者"周期都是 30 秒"(原话，ASR? 单位存疑) 所以一旦撞过会被相位矫正，不会再撞。[BV1iQrKYoEoy 19:15] [BV1iQrKYoEoy 19:17] [BV1iQrKYoEoy 19:20] [BV1iQrKYoEoy 19:57] [BV1iQrKYoEoy 20:01] [BV1iQrKYoEoy 20:08] [BV1iQrKYoEoy 20:14]
- 反例："滤计"(ASR?)产线两格直接怼在蓝带上，撞到回流物品后无法在那一 tick 精确投出，最后一台组装机因某原料不足而不满速，即使最后两台已换白爪也无效。[BV1iQrKYoEoy 17:51] [BV1iQrKYoEoy 18:02] [BV1iQrKYoEoy 18:11] [BV1iQrKYoEoy 18:15] [BV1iQrKYoEoy 18:22]
- 不要让爪子搭在梯口上：旧版把爪放在该位置，会让优先级出问题（可能抓塔里的货物而不是回收来的货物），造成死锁；要让梯口是完整的梯口，爪子放在别处。[BV1iQrKYoEoy 50:22] [BV1iQrKYoEoy 50:27] [BV1iQrKYoEoy 50:36] [BV1iQrKYoEoy 50:39] [BV1iQrKYoEoy 50:47] [BV1iQrKYoEoy 50:50] [BV1iQrKYoEoy 50:53]
- 反例（补货口位置）：一条蓝带用白爪分成两半补两个口，若白爪放反/前面没有一大段带子，统计波动会使两个补货口同时需要整条带，前面只有一个白爪供不上，一边缺料，并向后"蝴蝶效应"传递，最终后面整个周期无法生产。解决：补货口前移，使有足够长的缓存；或干脆从塔里拉两条带。缓存越长，影响频率指数下降。[BV1iQrKYoEoy 35:33] [BV1iQrKYoEoy 35:46] [BV1iQrKYoEoy 36:09] [BV1iQrKYoEoy 36:33] [BV1iQrKYoEoy 37:00] [BV1iQrKYoEoy 37:15] [BV1iQrKYoEoy 37:17] [BV1iQrKYoEoy 37:25] [BV1iQrKYoEoy 38:16] [BV1iQrKYoEoy 38:20]

### D. 三物品及以上混带
- 他做不出能"扛任何物品断货"的三物品控制单元（改了二三十次），所以 3 个物品的补料口都用旧版梯口投料，不用传送带直怼；评论区若有人会做，下期再介绍。[BV1iQrKYoEoy 08:21] [BV1iQrKYoEoy 08:26] [BV1iQrKYoEoy 08:44] [BV1iQrKYoEoy 09:53] [BV1iQrKYoEoy 09:55] [BV1iQrKYoEoy 10:08] [BV1iQrKYoEoy 10:12] [BV1iQrKYoEoy 10:44]
- 不能直怼的理由：若直怼了一种物品（如硫酸），另两种断货，硫酸回流后"只有一个绿口投出去"，回收缓存爆掉，最终带上全是硫酸。[BV1iQrKYoEoy 10:22] [BV1iQrKYoEoy 10:27] [BV1iQrKYoEoy 10:31] [BV1iQrKYoEoy 10:37] [BV1iQrKYoEoy 10:42]
- 他看过食仓/重装小兔/TT 的蓝图：三产物物品大多不用"控制单元+直怼补料"；需要三种以上进料的配方里最常见的是粒子容器，电动机也是（电动机可用单极磁石替代而不产）；粒子容器配方慢（8 秒一个），普通梯口控制单元也能带很多机器、省很多爪子。[BV1iQrKYoEoy 08:54] [BV1iQrKYoEoy 09:10] [BV1iQrKYoEoy 09:12] [BV1iQrKYoEoy 09:24] [BV1iQrKYoEoy 09:26] [BV1iQrKYoEoy 09:33] [BV1iQrKYoEoy 09:38] [BV1iQrKYoEoy 09:46]
- 钛合金（三物品）：投料为硫酸 2/5，另两种各 1/5，合计 4/5，未跑满带；一满带理论可带 15 台熔炉，他只带 12 台；硫酸用绿带，钢材和钛板用黄带。[BV1iQrKYoEoy 11:06] [BV1iQrKYoEoy 11:14] [BV1iQrKYoEoy 11:19] [BV1iQrKYoEoy 11:20] [BV1iQrKYoEoy 11:23] [BV1iQrKYoEoy 52:21] [BV1iQrKYoEoy 52:24] [BV1iQrKYoEoy 52:25]
- 钛合金炉的白爪：最后几台熔炉不能用绿爪进货，因为绿爪有工作时间，连续来两个想要的物品只能抓一个，另一个漏过去被回收重新投料，使该炉少拿一个而停产；白爪没有工作时间，不漏。实验得出"最后三个熔炉要白爪"，漏多少没法算，是"实验数值"；他说再多换白爪性价比不高，让它少工作一会儿无所谓。[BV1iQrKYoEoy 11:31] [BV1iQrKYoEoy 11:34] [BV1iQrKYoEoy 11:43] [BV1iQrKYoEoy 11:49] [BV1iQrKYoEoy 12:01] [BV1iQrKYoEoy 12:07] [BV1iQrKYoEoy 12:20] [BV1iQrKYoEoy 12:24] [BV1iQrKYoEoy 12:26] [BV1iQrKYoEoy 12:36]
- 钛合金线总需求小：12 个熔炉就够（他放 10 个，之后补 2 个），放在极地。[BV1iQrKYoEoy 52:00] [BV1iQrKYoEoy 52:04] [BV1iQrKYoEoy 52:41] [BV1iQrKYoEoy 52:44]
- 粒子容器（1:1:1 三物品）：只能用梯口投料，每物品恰好投 1/3 流量；24 台/组（26 台测试跑不满）；投 9 以恰好满足 24 台也不行，仍会让最后几台无法工作。[BV1iQrKYoEoy 40:12] [BV1iQrKYoEoy 40:14] [BV1iQrKYoEoy 40:23] [BV1iQrKYoEoy 41:36] [BV1iQrKYoEoy 41:39] [BV1iQrKYoEoy 43:19] [BV1iQrKYoEoy 43:24] [BV1iQrKYoEoy 43:26]
  - 只做 24 台的原因：混带后段物料分布已很乱、波动高；机器每种物料缓存有限，波动大时某物品富余、另一物品缺，机器不再抓富余的，之后短缺时漏过去的抓不回来。（10 台一组的黑马达线波动影响不到最后一台。）[BV1iQrKYoEoy 41:48] [BV1iQrKYoEoy 41:51] [BV1iQrKYoEoy 41:53] [BV1iQrKYoEoy 42:03] [BV1iQrKYoEoy 42:18] [BV1iQrKYoEoy 42:34] [BV1iQrKYoEoy 42:55] [BV1iQrKYoEoy 43:01]
  - 全黄爪进、黄爪出（配方太慢）；出货时用白爪反抓做堆叠，并到一条带。粒子容器线复制 5 份。[BV1iQrKYoEoy 43:46] [BV1iQrKYoEoy 43:48] [BV1iQrKYoEoy 44:14] [BV1iQrKYoEoy 44:17] [BV1iQrKYoEoy 44:50]
- 奇异物质（ASR? 原文"重轻"= 重氢，三物品：粒子容器 + 铁板 + 重氢）：重氢占流量最大，若三物品混一条带则带只能带少量；方案一：粒子容器+铁板混一条带、重氢单走一条（每机器多一个爪，但更美观、一带能带机器数大幅提高）；方案二（他选）：三个全混一条带。分配：重氢投 21/秒（绿带 12 + 黄带 6 + 绿爪 3），剩 9，铁板和粒子容器各 4.5（一个绿爪一个黄爪）。[BV1iQrKYoEoy 62:19] [BV1iQrKYoEoy 62:28] [BV1iQrKYoEoy 62:35] [BV1iQrKYoEoy 62:38] [BV1iQrKYoEoy 62:45] [BV1iQrKYoEoy 62:55] [BV1iQrKYoEoy 63:13] [BV1iQrKYoEoy 63:26] [BV1iQrKYoEoy 63:46] [BV1iQrKYoEoy 63:54] [BV1iQrKYoEoy 63:59] [BV1iQrKYoEoy 64:06] [BV1iQrKYoEoy 64:08]
  - 同样强调这些爪子"不能直接怼在黄带上"（原话 64:13，语序混乱）；每个投料口至少放一格带做缓冲，重氢有两个投料口，两边至少都有一格缓存。[BV1iQrKYoEoy 64:11] [BV1iQrKYoEoy 66:09] [BV1iQrKYoEoy 66:14] [BV1iQrKYoEoy 66:19] [BV1iQrKYoEoy 66:25]
  - 一个塔带 64 台对撞机，是"两条不同的混带"，一条蓝带只能带 32 台；复制 4 份共 256 台，总共需 300 多台。[BV1iQrKYoEoy 68:34] [BV1iQrKYoEoy 68:38] [BV1iQrKYoEoy 68:40] [BV1iQrKYoEoy 68:47] [BV1iQrKYoEoy 68:50] [BV1iQrKYoEoy 68:53]
  - 对撞机本身很大、制造速度为 1 而其他机器已是 3 倍，所以占地大；北极带的纬度带撞光子，南边撞奇异物质。[BV1iQrKYoEoy 60:21] [BV1iQrKYoEoy 60:23] [BV1iQrKYoEoy 60:27] [BV1iQrKYoEoy 60:40] [BV1iQrKYoEoy 60:44] [BV1iQrKYoEoy 60:46]
- 黑棒（四物品，氢+反物质用量高，湮灭约束球与钛合金几乎没有）：可选方案：氢和反物质用控制单元摇匀，另开一条带混湮灭约束球和钛合金，每台组装机两爪进料一爪出料；他仍选四种直接混一条带。[BV1iQrKYoEoy 69:36] [BV1iQrKYoEoy 69:38] [BV1iQrKYoEoy 70:00] [BV1iQrKYoEoy 70:16] [BV1iQrKYoEoy 70:18] [BV1iQrKYoEoy 70:26] [BV1iQrKYoEoy 70:31] [BV1iQrKYoEoy 70:44]
  - 四物品配方带来两个问题：混带很不好混；一个塔四种物品加回料会把小塔撑爆，所以用小塔、并且黑棒不进塔，直接做在旁边投料（与 750 厂同样处理）。[BV1iQrKYoEoy 69:36] [BV1iQrKYoEoy 69:38] [BV1iQrKYoEoy 69:40] [BV1iQrKYoEoy 69:45] [BV1iQrKYoEoy 69:49] [BV1iQrKYoEoy 69:54]
  - 投料：氢和反物质各用一个绿带，另两种用黄爪（"两个黄爪都是多的"）；另一侧三种用量很少的物品全用黄带，加上绿带刚好占满蓝带。[BV1iQrKYoEoy 72:09] [BV1iQrKYoEoy 72:13] [BV1iQrKYoEoy 72:16] [BV1iQrKYoEoy 74:11] [BV1iQrKYoEoy 74:18] [BV1iQrKYoEoy 74:30] [BV1iQrKYoEoy 74:36] [BV1iQrKYoEoy 74:38] [BV1iQrKYoEoy 74:40]
  - 开了加速之后黄爪带不动，进货口需绿爪（反物质和氢量"比我想象的多得多，当时我根本没算"）；黄棒线可全用黄爪。[BV1iQrKYoEoy 74:52] [BV1iQrKYoEoy 74:55] [BV1iQrKYoEoy 74:57] [BV1iQrKYoEoy 75:03] [BV1iQrKYoEoy 75:11]
  - 黑棒出货：两条出货带可合并为一条；总共六条带的黑棒一定要做集装（白爪反向堆叠），因为直接合起来超过 0.25 条蓝带；然后借增产剂涂增产。塔物流来的四个物品用白爪分流投到两边。[BV1iQrKYoEoy 72:23] [BV1iQrKYoEoy 72:26] [BV1iQrKYoEoy 73:12] [BV1iQrKYoEoy 73:16] [BV1iQrKYoEoy 73:45] [BV1iQrKYoEoy 73:53] [BV1iQrKYoEoy 73:57] [BV1iQrKYoEoy 73:58] [BV1iQrKYoEoy 74:00] [BV1iQrKYoEoy 74:03] [BV1iQrKYoEoy 74:06]
  - 一个投料组共用的两个物流塔：一个供氢+反物质，另一个供钛合金+湮灭约束球；可把氢和反物质分到两个塔减小运力压力，他认为没必要。[BV1iQrKYoEoy 72:38] [BV1iQrKYoEoy 72:44] [BV1iQrKYoEoy 72:48] [BV1iQrKYoEoy 72:51] [BV1iQrKYoEoy 72:56] [BV1iQrKYoEoy 72:58]
  - 环带布局：一条是环带，一条是回去的环带；每个环带只能带 15 台，回来的这条带上没放爪子，"有点浪费"。[BV1iQrKYoEoy 73:01] [BV1iQrKYoEoy 73:04] [BV1iQrKYoEoy 73:06] [BV1iQrKYoEoy 73:08] [BV1iQrKYoEoy 73:11]

### E. 分拣器（爪子）选型规则：何时不能用"恰好够"的爪
- 规则 1：爪子所接的带子是混带时，不管进货还是出货，爪子运力不能恰好等于理论运力，必须更高（高多少"只要大于零"，因机器自带不小的缓存；出料混带"必须严格高，不能跟它相等"）。原因：爪抓走一个后，带上下一个未必是它要的物品（可能是另一种或空），爪必然有空转，不能满速，机器就跟着少工作。[BV1iQrKYoEoy 29:44] [BV1iQrKYoEoy 29:55] [BV1iQrKYoEoy 29:57] [BV1iQrKYoEoy 30:11] [BV1iQrKYoEoy 30:14] [BV1iQrKYoEoy 30:24] [BV1iQrKYoEoy 30:32] [BV1iQrKYoEoy 30:35] [BV1iQrKYoEoy 30:42] [BV1iQrKYoEoy 30:53] [BV1iQrKYoEoy 31:29] [BV1iQrKYoEoy 31:32] [BV1iQrKYoEoy 31:33]
  - 例：电动机进料口理论上绿爪恰好够，但用绿爪会出黄点（带子是混料），所以改蓝爪。[BV1iQrKYoEoy 34:00] [BV1iQrKYoEoy 34:02] [BV1iQrKYoEoy 34:06] [BV1iQrKYoEoy 34:07] [BV1iQrKYoEoy 34:09]
  - 例：分解可燃冰产线，出料口是混带（可燃冰分解为石墨烯、氢等），出料爪也要高于理论运力。[BV1iQrKYoEoy 30:59] [BV1iQrKYoEoy 31:04] [BV1iQrKYoEoy 31:07] [BV1iQrKYoEoy 31:10]
  - 即使出料混带并不能省爪，也可能省带和占地，他说后面可能用。[BV1iQrKYoEoy 31:13] [BV1iQrKYoEoy 31:18] [BV1iQrKYoEoy 31:20]
- 规则 2：出货物品带增产（额外产出）时不能用恰好等于理论的爪，要提前升级爪。原因：增产使产出不均匀（多数周期每台只产 1 个，偶尔一个周期一次产出翻倍），短窗口内流量波动很大，带子有时被占满、有时全空；恰好运力的爪在带子被占满时卡住，且带子空闲时超出的运力又不足以把积压清空，于是黄点。[BV1iQrKYoEoy 31:37] [BV1iQrKYoEoy 31:40] [BV1iQrKYoEoy 31:44] [BV1iQrKYoEoy 31:53] [BV1iQrKYoEoy 31:55] [BV1iQrKYoEoy 32:07] [BV1iQrKYoEoy 32:14] [BV1iQrKYoEoy 32:21] [BV1iQrKYoEoy 32:34] [BV1iQrKYoEoy 32:39] [BV1iQrKYoEoy 32:44] [BV1iQrKYoEoy 33:00] [BV1iQrKYoEoy 33:02] [BV1iQrKYoEoy 33:08]
  - 计算：按"加速"算，因为额外产出的波动上限是一个生产周期内最多产出两个，正好是加速比例；懒得算就用实验法（换爪看黄点是否消失）。[BV1iQrKYoEoy 33:17] [BV1iQrKYoEoy 33:21] [BV1iQrKYoEoy 33:25] [BV1iQrKYoEoy 33:27] [BV1iQrKYoEoy 33:30] [BV1iQrKYoEoy 33:33] [BV1iQrKYoEoy 33:36]
  - 出货口要提前变白爪，而不是在"恰好该变白爪的地方"才变。[BV1iQrKYoEoy 34:12] [BV1iQrKYoEoy 34:15] [BV1iQrKYoEoy 34:18] [BV1iQrKYoEoy 34:20]
- 绿马达出料理论上 16 台组装机刚好带满一条蓝带；他只过了 12 台就把后面两台换白爪（"我上一期提过的问题"）。[BV1iQrKYoEoy 29:15] [BV1iQrKYoEoy 29:17] [BV1iQrKYoEoy 29:20] [BV1iQrKYoEoy 29:26] [BV1iQrKYoEoy 29:27] [BV1iQrKYoEoy 29:29]
- 两条规则都不成立时，可用恰好运力的爪，或在回料带"恰好满带的位置"才改白爪。[BV1iQrKYoEoy 34:25] [BV1iQrKYoEoy 34:28] [BV1iQrKYoEoy 34:30] [BV1iQrKYoEoy 34:32]
  - 冶炼区示例：铁板进货黄爪刚好够；出货刚好五个蓝爪堆满一条蓝带，所以恰好在那里变蓝爪；若是增产产线则要提前换。[BV1iQrKYoEoy 34:37] [BV1iQrKYoEoy 34:40] [BV1iQrKYoEoy 34:46] [BV1iQrKYoEoy 34:48] [BV1iQrKYoEoy 34:50] [BV1iQrKYoEoy 34:55] [BV1iQrKYoEoy 34:57]
  - 钛矿冶炼：两进一出，前 10 个熔炉绿爪、后面全白爪；铁矿冶炼（一进一出）爪子高一级：蓝爪，只有 5 个熔炉就要换白爪。[BV1iQrKYoEoy 51:17] [BV1iQrKYoEoy 51:21] [BV1iQrKYoEoy 51:25] [BV1iQrKYoEoy 51:26] [BV1iQrKYoEoy 51:28] [BV1iQrKYoEoy 51:32] [BV1iQrKYoEoy 51:34] [BV1iQrKYoEoy 51:36] [BV1iQrKYoEoy 51:37]
  - 铜矿冶炼结构与铁矿相同，硅矿冶炼与钛矿相同。[BV1iQrKYoEoy 53:36] [BV1iQrKYoEoy 53:39]
- 光子对撞机（一个反物质 → 一个光子 + 一个氢）：进货黄爪完全够（机器慢），出货一个物品变两个，需两条满带的反物质+氢，混带，"提前变白爪"并"装成四堆叠"，否则物品会堵住；一满带光子可带 60 台粒子对撞机；一个塔可左右各带 60 台，运力压力小，他一个塔只带一条蓝带，放了四个塔，第五个塔因空间不足。[BV1iQrKYoEoy 61:06] [BV1iQrKYoEoy 61:08] [BV1iQrKYoEoy 61:20] [BV1iQrKYoEoy 61:24] [BV1iQrKYoEoy 61:30] [BV1iQrKYoEoy 61:32] [BV1iQrKYoEoy 61:36] [BV1iQrKYoEoy 61:39] [BV1iQrKYoEoy 61:42] [BV1iQrKYoEoy 61:45] [BV1iQrKYoEoy 61:47] [BV1iQrKYoEoy 61:55] [BV1iQrKYoEoy 62:01] [BV1iQrKYoEoy 62:04]
- 光子对撞机喷涂：该产线只能"加速"，光子接出后过一个喷涂机再上带即可。[BV1iQrKYoEoy 60:52] [BV1iQrKYoEoy 61:00] [BV1iQrKYoEoy 61:02]
- 增产剂喷涂位置：黑棒出货集装后，借"增产剂"位置涂增产，"直接怼到黄棒生产单元上面"；黄棒回塔前也要记得涂增产剂。[BV1iQrKYoEoy 74:03] [BV1iQrKYoEoy 74:06] [BV1iQrKYoEoy 74:08] [BV1iQrKYoEoy 75:37] [BV1iQrKYoEoy 75:39]
- 白爪（最高级）被他用于：出货反向集装/四堆叠、分流、防漏料的末端机器；"白爪性能比蓝爪差"（CPU）。[BV1iQrKYoEoy 21:58] [BV1iQrKYoEoy 22:00] [BV1iQrKYoEoy 44:14] [BV1iQrKYoEoy 72:26] [BV1iQrKYoEoy 73:45]

### F. 放组装机 / 带的位置规则
- 不能随便往混带线中间加机器（改变带上流量，后面全错）；只能在"进投料单元前面的位置"放。[BV1iQrKYoEoy 39:07] [BV1iQrKYoEoy 39:09] [BV1iQrKYoEoy 39:12] [BV1iQrKYoEoy 39:17] [BV1iQrKYoEoy 39:22]
- 传送带过爪：其他九格都放了爪，带子跨不过去，所以他多加一格带；也可升到二层绕过去，但他不喜欢升到高度二。[BV1iQrKYoEoy 44:21] [BV1iQrKYoEoy 44:25] [BV1iQrKYoEoy 44:29] [BV1iQrKYoEoy 44:35]
- 钛合金出线：回流直接绕一下进塔；在一层"跨越"所有传送带。[BV1iQrKYoEoy 52:29] [BV1iQrKYoEoy 52:31] [BV1iQrKYoEoy 52:36] [BV1iQrKYoEoy 52:38]
- 投料口、爪子、带子都可以放高空压榨空间，但"不好造不好修"，他习惯放地面。[BV1iQrKYoEoy 71:07] [BV1iQrKYoEoy 71:13] [BV1iQrKYoEoy 71:17] [BV1iQrKYoEoy 71:20] [BV1iQrKYoEoy 71:25]
- 微晶元件产线的带子"可以走高空，中间还能再挤下一排组装机"，他没做，导致球放不下两万糖。[BV1iQrKYoEoy 57:46] [BV1iQrKYoEoy 57:50] [BV1iQrKYoEoy 57:53] [BV1iQrKYoEoy 57:57]

### G. 断料 / 重启 / 堵塞测试
- 正常产线必须能在这些情况后重启：两种产物分别断货（最常见，必测）；产物堆积；断电。[BV1iQrKYoEoy 03:52] [BV1iQrKYoEoy 03:56] [BV1iQrKYoEoy 04:03] [BV1iQrKYoEoy 04:10] [BV1iQrKYoEoy 04:21]
- 产物堆积：环带整圈停转（投料口锁死），但不是卡死；需求恢复后爪子继续抓、环带出现缺口，自然重启。上一期所有产线都扛产物堆积。[BV1iQrKYoEoy 04:21] [BV1iQrKYoEoy 04:37] [BV1iQrKYoEoy 04:51]
- 断电失效：爪不工作 → 两种货物不能分离 → 环带卡死；复电后一般还能恢复（带上两种物品都有）；会真正卡死的情形：反复断电/复电且中间只生产极短时间；或先某原料（原文"金刚石"，ASR? 实为一种原料）断货、带上充满另一种物品，再断电，复电后断货物品进不去 → 停产（先断货再断电才会；先断电再断货则不会）。[BV1iQrKYoEoy 05:21] [BV1iQrKYoEoy 05:51] [BV1iQrKYoEoy 06:06] [BV1iQrKYoEoy 06:21] [BV1iQrKYoEoy 06:33]
- 抗停产的优化：在进控制单元前"越远越好"的位置，把一格蓝带拆掉，改成"用白爪接过去"；断电时爪不工作，那段带子上就没有物品，复电后有缓存空间能让断货物品进入，机器恢复；他因少见断电而没做。[BV1iQrKYoEoy 06:51] [BV1iQrKYoEoy 07:03] [BV1iQrKYoEoy 07:15] [BV1iQrKYoEoy 07:22] [BV1iQrKYoEoy 07:30] [BV1iQrKYoEoy 07:36] [BV1iQrKYoEoy 07:56] [BV1iQrKYoEoy 08:00]
- 开机观察：看黄点是否逐渐消失，持续没有黄点才算没问题；"剃口混带的断料测试，足够有信心可以不做，只需看比例、爪子有没有用错"。[BV1iQrKYoEoy 43:36] [BV1iQrKYoEoy 43:38] [BV1iQrKYoEoy 43:41] [BV1iQrKYoEoy 43:44] [BV1iQrKYoEoy 44:04] [BV1iQrKYoEoy 44:08] [BV1iQrKYoEoy 44:11]
- 混带启动速度是"硬伤"，梯口投料启动速度并不比直怼好多少。[BV1iQrKYoEoy 43:55] [BV1iQrKYoEoy 44:00]
- 私下开机调试了五六次，每次都能找到一个小 bug，多为低级错误（接错东西、爪子用错、带子流量不对）。[BV1iQrKYoEoy 28:57] [BV1iQrKYoEoy 28:59] [BV1iQrKYoEoy 38:35] [BV1iQrKYoEoy 38:40]
- 混带产线任何地方都可能发生波动，只要发生频率小就不用管；"根据概率论……受缓存空间影响应是指数级"。[BV1iQrKYoEoy 37:59] [BV1iQrKYoEoy 38:01] [BV1iQrKYoEoy 38:09] [BV1iQrKYoEoy 38:16] [BV1iQrKYoEoy 38:20]
- 机器停工与跑不满对 CPU 压力一样（都是机器停工）；有溢出产量的可不管，但"恰好 144 台"那类（钛晶石）少一台就少糖，必须测试跑满。[BV1iQrKYoEoy 03:13] [BV1iQrKYoEoy 03:30] [BV1iQrKYoEoy 03:33] [BV1iQrKYoEoy 03:52]
- 黑马达/混带控制单元与直接产线比：空闲时更占电和 CPU，但万糖开机后一直工作所以无所谓。[BV1iQrKYoEoy 26:04] [BV1iQrKYoEoy 26:07] [BV1iQrKYoEoy 26:11] [BV1iQrKYoEoy 26:13]

### H. 开头对上一期问题的更正
- 增产策略帖来自时桑(ASR?)大佬，是黑雾版本前的；黑雾版本后组装机/熔炉/研究站升到三级而化工厂未升，机器比例变化使增产选取变化；TT 大佬重发新帖；两版差别主要在临界点物品，他不改产线。[BV1iQrKYoEoy 00:00] [BV1iQrKYoEoy 00:29] [BV1iQrKYoEoy 00:57] [BV1iQrKYoEoy 01:14]
- 他的公式曾算错（中间插行，后行仍引用前行），导致一个物品为 -1.5、一个为 0，说明比例不可行；所以"滤计"线只能采用 2:4 的比例；评论指出 2:4 的"黄积"(ASR?)刚好只能带 6 台组装机，可能跑不满，可改 1:2（白爪分流要增多），或让一个控制单元少带点机器，或接受跑不满。[BV1iQrKYoEoy 01:44] [BV1iQrKYoEoy 02:00] [BV1iQrKYoEoy 02:13] [BV1iQrKYoEoy 02:33] [BV1iQrKYoEoy 02:43] [BV1iQrKYoEoy 02:56] [BV1iQrKYoEoy 03:13]

### I. 物流塔 / 站 / 优先级
- 出口大塔数量需算大飞机运力 + 容量：一个大塔的有效转运约等于"20 个大飞机"（一来一回只有一半时间会占用；20000 个物品恰好是 10 个大飞机满载，每架 2000 个碳管）；原因是飞机起飞即"预定"整塔货物，小飞机补货被卡；需要 40 架大飞机就至少放 2 个塔。[BV1iQrKYoEoy 47:33] [BV1iQrKYoEoy 47:36] [BV1iQrKYoEoy 47:52] [BV1iQrKYoEoy 48:15] [BV1iQrKYoEoy 48:21] [BV1iQrKYoEoy 48:24] [BV1iQrKYoEoy 48:36] [BV1iQrKYoEoy 49:00] [BV1iQrKYoEoy 49:04] [BV1iQrKYoEoy 49:17] [BV1iQrKYoEoy 49:21] [BV1iQrKYoEoy 49:25]
- 一个塔接 12 个口，总运力压力约 6 条满带；能这样做是因为取货的塔离得很近（取货塔近才能做到 4 条满带以上）；否则让一个塔只带一排组装机。[BV1iQrKYoEoy 25:28] [BV1iQrKYoEoy 25:34] [BV1iQrKYoEoy 25:38] [BV1iQrKYoEoy 25:43] [BV1iQrKYoEoy 25:45] [BV1iQrKYoEoy 25:51]
- 齿轮一个塔带三条袋子是为了"总共六条"。[BV1iQrKYoEoy 24:32] [BV1iQrKYoEoy 24:36]
- 黄棒出口塔设"限制 400"（存量限制）。[BV1iQrKYoEoy 49:57] [BV1iQrKYoEoy 50:07] [BV1iQrKYoEoy 50:11]
- 电池堵塞事故：忘了给物流塔设置优先级分组，工业星补电池的塔先回收电池，充电星没电；要把相关塔放同一分组。[BV1iQrKYoEoy 64:37] [BV1iQrKYoEoy 64:52] [BV1iQrKYoEoy 64:55] [BV1iQrKYoEoy 65:00] [BV1iQrKYoEoy 65:30] [BV1iQrKYoEoy 65:37] [BV1iQrKYoEoy 65:39]
- 黄棒回塔设"本地需求星际供应"；也可暂改星际仓储保证本星黄棒；黄棒运过来后会优先从传送带出去，而不是优先被大飞机预定。[BV1iQrKYoEoy 75:44] [BV1iQrKYoEoy 75:46] [BV1iQrKYoEoy 75:54] [BV1iQrKYoEoy 76:04] [BV1iQrKYoEoy 76:09] [BV1iQrKYoEoy 76:14]
- 增产剂(ASR? 原文"蓝记/篮记")需超过一条蓝带的 7 成多(原文"712"，ASR?)运力，要在某个塔处需求并用白爪接到总环线上。[BV1iQrKYoEoy 46:40] [BV1iQrKYoEoy 46:46] [BV1iQrKYoEoy 46:51]
- 重氢产线两种方案：做进两万糖蓝图，或直接在气态巨星采集；他算出两万糖需要矿物循环约 80 级才够，先在外接产线转重氢。[BV1iQrKYoEoy 66:36] [BV1iQrKYoEoy 66:42] [BV1iQrKYoEoy 66:46] [BV1iQrKYoEoy 67:04] [BV1iQrKYoEoy 67:08] [BV1iQrKYoEoy 67:25] [BV1iQrKYoEoy 67:44]

### J. 其他数字与规模
- 黑棒 75 台组装机（需要）；黄棒线可全用黄爪；对撞机 64/塔；光子对撞机 60 台/塔-带。[BV1iQrKYoEoy 71:31] [BV1iQrKYoEoy 75:11] [BV1iQrKYoEoy 68:34] [BV1iQrKYoEoy 61:32]
- 目标缩减：放弃十万糖，回退为两万糖；到十万糖需等矿物循环"三四百级"；磁石/碳管高级产线是否开有待考虑。[BV1iQrKYoEoy 77:14] [BV1iQrKYoEoy 77:19] [BV1iQrKYoEoy 78:22] [BV1iQrKYoEoy 78:24] [BV1iQrKYoEoy 77:26]
- 熔炉不足：单极磁石不够，家里超市无法产熔炉。[BV1iQrKYoEoy 24:44] [BV1iQrKYoEoy 24:48]
- 铁矿不足/氢卡死：铺大产线后铁不够；氢无法消耗是因为卡氢产线没开工。他说"最阴间的三个东西"是粒子容器、CPU（处理器）、化工（碳纳米管）：单物品消耗矿太多，一铺就暴露矿没开够。[BV1iQrKYoEoy 45:07] [BV1iQrKYoEoy 45:10] [BV1iQrKYoEoy 45:21] [BV1iQrKYoEoy 45:23] [BV1iQrKYoEoy 45:30] [BV1iQrKYoEoy 45:32] [BV1iQrKYoEoy 46:01] [BV1iQrKYoEoy 46:03]
