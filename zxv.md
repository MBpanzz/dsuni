以下是 dsuni 求解器的直接设计蓝图，聚焦于架构、模块和实现路线。

---

dsuni 求解器设计蓝图

1. 设计目标

· 覆盖范围广：SAT、SMT (QF_LIA/QF_LRA/QF_NIA)、MILP、MINLP、CP
· 纯 Rust 实现（可选 FFI 调用外部库作为后备）
· 模块化：每个理论/算法独立，便于定制
· 正确优先：优先保证结果正确性，性能后续优化
· 可验证：关键模块支持形式化验证（Verus 等）

2. 整体架构

```
dsuni
├── core/ # 核心数据结构与接口
├── sat/ # SAT 求解器 (CDCL)
├── smt/ # SMT 求解器 (理论组合)
├── lp/ # 线性规划 (单纯形/内点)
├── milp/ # 混合整数线性规划 (分支定界)
├── minlp/ # 混合整数非线性规划 (外逼近)
├── cp/ # 约束规划 (传播+回溯)
├── util/ # 通用工具 (日志、统计、内存管理)
└── ffi/ # 可选外部求解器接口 (Z3, SCIP, Ipopt)
```

3. 核心数据结构 (core)

```rust
pub enum Term {
Bool(bool),
Int(i64),
Real(f64),
Var(usize),
Not(Box<Term>),
And(Vec<Term>),
Or(Vec<Term>),
Eq(Box<Term>, Box<Term>),
Le(Box<Term>, Box<Term>),
Add(Vec<Term>),
Mul(Vec<Term>),
// 扩展: 三角函数, 指数等
}

pub struct Constraint {
pub term: Term,
pub is_hard: bool,
pub weight: Option<f64>, // 软约束权重
}

pub struct Model {
pub assignments: Vec<Value>, // 变量赋值
}

pub struct Config {
pub timeout_sec: f64,
pub max_memory_mb: usize,
pub solver_type: SolverType,
pub verbosity: u8,
}
```

4. SAT 模块 (sat/)

· 算法：CDCL (冲突驱动子句学习)
· 核心组件：
· Literal, Clause 表示
· 两字节能监视 (Two-Watched Literals)
· VSIDS 分支启发式
· 重启策略 (Luby/Geometric)
· 子句删除策略 (基于活动度)
· 输出：满足/不满足 + 赋值

5. SMT 模块 (smt/)

· 架构：分层求解 + 理论组合 (基于 Nelson-Oppen)
· 核心流程：SAT 求解器 + 理论求解器 (T-solver) 交互
· 内置理论：
· QF_LIA (线性整数算术)：基于单纯形 + 割平面
· QF_LRA (线性实数算术)：单纯形法
· QF_NIA (非线性整数算术)：区间传播 + 分支定界
· QF_UF (未解释函数)：一致性的等价类合并
· 理论组合：通过平等共享接口 (E-graph) 实现

6. 线性规划 (lp/)

· 算法：单纯形法 (修正单纯形) + 预求解
· 可选：内点法 (用于大规模问题)
· 数据结构：稀疏矩阵存储
· 数值鲁棒性：使用有理数 (ratio) 或高精度浮点

7. 混合整数线性规划 (milp/)

· 算法：LP 驱动的分支定界 + 割平面
· 分支策略：强分支 (strong branching) / 伪成本
· 割平面类型：Gomory、覆盖、混杂
· 启发式：取整启发式、松弛解邻域搜索
· 预处理：约束传播、系数化简

8. 混合整数非线性规划 (minlp/)

· 算法：外逼近 (Outer Approximation) + 分支定界
· 需要 NLP 求解器：可内部实现 Ipopt 风格或 FFI 调用
· 处理非凸：采用分段线性逼近 + 全局优化

9. 约束规划 (cp/)

· 算法：传播 + 回溯 (BT + MAC)
· 全局约束：all_different, cumulative, circuit 等
· 传播算法：基于事件驱动的固定点迭代

10. 可选 FFI 接口

当内置求解器不足时，调用外部库：

· Z3 (SMT)
· SCIP (MILP/MINLP)
· Ipopt (NLP)
· HiGHS (LP)

通过 feature flags 控制编译。

11. 统一入口 API

```rust
pub struct Dsuni {
config: Config,
constraints: Vec<Constraint>,
}

impl Dsuni {
pub fn new(config: Config) -> Self;
pub fn add_constraint(&mut self, c: Constraint);
pub fn solve(&mut self) -> SolverResult;
}

pub enum SolverResult {
Satisfiable(Model),
Unsatisfiable,
Unknown(String),
Timeout,
MemoryExceeded,
}
```

求解器自动选择策略：

· 只有布尔约束 → SAT
· 含算术但无限定 → SMT
· 线性目标 + 整数 → MILP
· 非线性 + 整数 → MINLP
· 含全局约束如 all_different → CP

12. 实现路线图 (Rust + AI 辅助)

阶段 内容 预估代码量
1 core 数据结构、Term parser 500 LoC
2 SAT CDCL 核心 1500 LoC
3 LP 单纯形法 1200 LoC
4 SMT 框架 + LIA/LRA 理论 2000 LoC
5 MILP 分支定界 1800 LoC
6 CP 基础传播 1000 LoC
7 MINLP 外逼近 2000 LoC
8 FFI 封装 800 LoC
9 统一 API + 自动策略选择 500 LoC

总计约 11k - 13k LoC，利用 AI (如 CodeLLM) 可以在数周内完成初始版本。

13. 正确性保障

· 单元测试：每个算法附 mini 实例测试
· 回归测试：使用 SMT-LIB, MIPLIB, MiniZinc 题库
· 形式化验证：关键算法 (如单纯形更新) 用 Verus 验证
· CI：每次提交运行完整测试集
