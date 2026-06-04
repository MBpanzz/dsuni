
```md
# dsuni 总路线图 ROADMAP.md

## 0. 项目定位

`dsuni` 是一个纯 Rust 实现的模块化求解器项目，目标逐步覆盖：

- SAT
- SMT
- LP
- MILP
- CP
- MINLP

项目采用显式求解入口，不做自动分类，不通过 FFI 调用外部求解器。
核心目标是先保证正确性、可维护性和可验证性，再逐步优化性能。

---

## 1. 总原则

### 1.1 纯 Rust

整个项目采用纯 Rust 实现。

不保留：

- Z3 FFI
- SCIP FFI
- Ipopt FFI
- HiGHS FFI
- 其他外部求解器 FFI

外部库可以用于：

- 测试对照
- benchmark 对比
- 离线验证

但不作为 `dsuni` 的运行时后端。

---

### 1.2 显式求解，不自动猜类型

不做自动分类，不根据约束内容自动选择求解器。

用户必须显式调用：

```rust
solve_sat(...)
solve_smt(...)
solve_lp(...)
solve_milp(...)
solve_cp(...)
solve_minlp(...)
```

或者通过 CLI 显式指定：

```bash
dsuni sat file.cnf
dsuni smt file.smt2
dsuni lp file.lp
dsuni milp file.mps
dsuni cp file.mzn
dsuni minlp file.xxx
```

文件后缀可以用于解析格式，但不用于自动推断求解器类别。

---

### 1.3 正确性优先

初期优先保证：

- 结果正确
- 错误可诊断
- 模型可检查
- 不确定情况明确返回 `Unknown`
- 不为了性能牺牲逻辑正确性

性能优化放在核心闭环稳定之后。

---

### 1.4 模块化推进

各求解模块相互独立，但共享基础设施：

```text
dsuni
├── core/
├── parser/
├── normalize/
├── sat/
├── linear/
├── lp/
├── smt/
├── milp/
├── cp/
├── minlp/
├── cli/
├── util/
└── tests/
```

其中：

- `core` 提供公共类型和接口
- `parser` 负责输入解析
- `normalize` 负责规范化
- `sat` 实现 SAT/CDCL
- `linear` 提供线性表达和线性约束基础
- `lp` 实现线性规划
- `smt` 实现 SAT + theory 框架
- `milp` 基于 LP relaxation 和分支定界
- `cp` 实现约束传播和回溯
- `minlp` 最后实现，避免过早引入复杂性
- `util` 提供日志、统计、资源限制、随机数、诊断工具等

---

### 1.5 可验证性预留

项目从一开始就要为后续验证预留结构。

重点关注：

- watched literal 不变式
- trail / assignment 一致性
- 线性规范化等价性
- domain pruning 正确性
- 回溯恢复正确性
- propagation 幂等性
- solver result 与 model 的一致性

后续可以针对关键模块引入 Verus 等工具，但初期先保证结构清晰、状态可检查、测试充分。

---

# 2. 第一阶段：核心地基

## 2.1 目标

建立所有模块共享的基础设施，确保后续 SAT、SMT、LP、MILP、CP、MINLP 可以在统一抽象上开发。

---

## 2.2 core 基础类型

需要建立以下基础类型：

```rust
Var
VarId
Sort / Ty
Value
Term
Constraint
Model
Assignment
SolverResult
Config
Status
Reason
Explanation
```

### Term 初始设计方向

`Term` 不要一次性做得过大，但要支持逐步扩展。

初始支持：

- Bool
- Int
- Real / Rational
- Var
- Not
- And
- Or
- Implies
- Eq
- Lt / Le / Gt / Ge
- Add
- Sub
- Mul

其中非线性 `Mul` 初期只作为 AST 表达，不要求所有模块都能求解。

---

## 2.3 结果类型

统一结果类型：

```rust
pub enum SolverResult {
    Sat(Model),
    Unsat,
    Unknown(UnknownReason),
    Timeout,
    MemoryExceeded,
    Error(SolverError),
}
```

`UnknownReason` 至少包括：

- Unsupported
- Undecidable
- NumericalIssue
- ResourceLimit
- IncompleteAlgorithm
- InternalReason

---

## 2.4 错误体系

错误需要区分：

- 语法错误
- 类型错误
- 约束非法
- 不支持的特性
- 超时
- 内存不足
- 数值错误
- 不可判定
- 内部错误

建议结构：

```rust
pub enum SolverError {
    ParseError(ParseError),
    TypeError(TypeError),
    InvalidConstraint(String),
    Unsupported(String),
    Timeout,
    MemoryExceeded,
    NumericalError(String),
    InternalError(String),
}
```

---

## 2.5 增量求解接口预留

统一预留增量求解接口：

```rust
push()
pop()
assume(...)
solve()
reset()
```

即使某些模块初期不完整支持，也要在 trait 层面保留。

示例：

```rust
pub trait Solver {
    fn push(&mut self);
    fn pop(&mut self);
    fn assume(&mut self, assumptions: &[LiteralLike]);
    fn solve(&mut self) -> SolverResult;
    fn reset(&mut self);
}
```

---

## 2.6 parser 层

初期支持简单文本格式和内部 DSL。

后续逐步支持：

- DIMACS：SAT
- SMT-LIB：SMT
- LP：LP
- MPS：MILP
- MiniZinc 子集：CP

注意：解析格式不代表自动选择求解器。
用户仍需要显式调用对应入口。

---

## 2.7 normalize 层

负责把用户输入转换为内部标准形式。

包括：

- 常量折叠
- 变量收集
- 类型检查
- 布尔表达式规范化
- NNF 转换
- CNF 转换预留
- 线性表达提取
- 非线性表达标记
- 约束合法性检查

---

## 2.8 本阶段产出

- 稳定的 `core`
- 基础 parser
- 基础 normalize 管线
- 统一错误体系
- 统一结果类型
- 最小 CLI
- 最小测试框架

---

# 3. 第二阶段：SAT 求解器

## 3.1 目标

实现一个可靠的 SAT 核心，为 SMT 提供布尔搜索底座。

---

## 3.2 范围

初始支持：

- CNF 输入
- DIMACS 解析
- CDCL 求解
- 模型输出
- UNSAT 判断

---

## 3.3 核心结构

需要实现：

```rust
Literal
Clause
ClauseId
VarActivity
ClauseActivity
Trail
DecisionLevel
Assignment
WatchList
Reason
```

---

## 3.4 CDCL 主循环

至少实现：

- decision
- unit propagation
- conflict detection
- conflict analysis
- learned clause generation
- non-chronological backtracking
- assignment rollback
- restart 预留

---

## 3.5 Two-Watched Literals

优先实现 two-watched literals。

需要维护以下不变式：

- 每个未满足子句至少有两个可监视 literal，或者已经是 unit/conflict
- watched literal 更新不能漏传播
- 回溯后 watch 结构仍然有效

这是后续形式化验证的重要对象。

---

## 3.6 启发式

第一版可先使用简单启发式：

- variable activity
- basic VSIDS-like bump
- phase saving 可后置
- restart 可先用简单几何策略
- clause deletion 可后置

后续增强：

- VSIDS
- Luby restart
- clause activity
- LBD
- learned clause database reduction

---

## 3.7 验证

测试包括：

- 手写小 CNF
- 已知 SAT / UNSAT 样例
- DIMACS 小实例
- 随机 CNF
- brute force 对照小变量实例

---

## 3.8 本阶段产出

- `SatSolver`
- DIMACS parser
- `solve_sat(...)`
- `dsuni sat file.cnf`
- 可稳定求解中小规模 CNF

---

# 4. 第三阶段：线性算术核心 linear

## 4.1 目标

建立 LP、SMT-LRA、SMT-LIA、MILP 共用的线性表达和线性约束基础。

---

## 4.2 核心结构

```rust
Rational
LinearExpr
LinearTerm
LinearConstraint
Objective
Bound
Domain
LinearSystem
```

示例：

```rust
LinearExpr = c0 + c1*x1 + c2*x2 + ...
```

约束类型：

```rust
<=
>=
=
```

目标函数：

```rust
minimize
maximize
none
```

---

## 4.3 数值策略

第一版优先精确性。

推荐：

- 使用精确有理数作为理论核心
- 浮点可作为后续性能优化分支
- 不允许浮点误差导致错误 SAT / UNSAT

如果后续引入近似数值，应明确返回：

```rust
Unknown(NumericalIssue)
```

而不是返回错误结论。

---

## 4.4 规范化

支持：

- 移项
- 合并同类项
- 常量折叠
- 系数约简
- 不等式方向统一
- 等式转双向不等式
- bound 提取
- 变量域提取

---

## 4.5 本阶段产出

- `linear` 子模块
- 线性表达构造 API
- 线性约束规范化
- 线性系统检查工具
- 与 core 的类型桥接

---

# 5. 第四阶段：LP 求解器

## 5.1 目标

实现纯 Rust 的基础 LP 求解器，为 MILP 和 SMT-LRA 提供底层支持。

---

## 5.2 初始范围

支持：

- 线性可行性
- 线性优化
- bounded / unbounded 判断
- infeasible 判断

---

## 5.3 算法路线

优先实现：

- 单纯形法
- revised simplex 或 dictionary/tableau 形式
- Phase I / Phase II
- 基础退化处理
- 基础 ratio test

内点法可以后置。

---

## 5.4 预处理

支持：

- bound tightening
- 常量传播
- 冗余约束初步检测
- 明显 infeasible 检测
- 空约束 / 零系数处理

---

## 5.5 结果类型

```rust
pub enum LpResult {
    Optimal(Model, ObjectiveValue),
    Feasible(Model),
    Infeasible,
    Unbounded,
    Unknown(UnknownReason),
}
```

---

## 5.6 验证

测试包括：

- 小型 LP 手写实例
- infeasible 实例
- unbounded 实例
- 退化实例
- 与暴力枚举小规模有界整数网格对照
- 与外部工具离线对照，但不作为运行时依赖

---

## 5.7 本阶段产出

- `LpSolver`
- `solve_lp(...)`
- `dsuni lp file.lp`
- 可被 SMT-LRA 和 MILP 复用的 LP 内核

---

# 6. 第五阶段：SMT 基础框架

## 6.1 目标

构建 SAT + theory 的 SMT 框架。
初期只支持有限、可控的理论组合，不追求全理论覆盖。

---

## 6.2 首批理论支持顺序

建议顺序：

1. `QF_UF`
2. `QF_LRA`
3. `QF_LIA`
4. `UF + LRA`
5. `UF + LIA`

`QF_NIA` 暂不作为早期重点，可放到 MINLP 或专门 nonlinear 模块之后。

---

## 6.3 SMT 架构

整体结构：

```text
输入公式
   ↓
解析 / 类型检查 / 规范化
   ↓
布尔抽象
   ↓
SAT 搜索
   ↓
Theory Solver 检查 / 传播 / 冲突解释
   ↓
学习子句
   ↓
SAT / UNSAT / UNKNOWN
```

---

## 6.4 Theory Solver trait

建议定义统一接口：

```rust
pub trait TheorySolver {
    fn push(&mut self);
    fn pop(&mut self);

    fn assert_literal(&mut self, lit: TheoryLiteral) -> TheoryResult;

    fn propagate(&mut self) -> Vec<TheoryPropagation>;

    fn check(&mut self) -> TheoryCheckResult;

    fn explain_conflict(&self) -> Option<TheoryExplanation>;

    fn explain_propagation(&self, p: TheoryPropagation) -> TheoryExplanation;
}
```

---

## 6.5 QF_UF

实现：

- union-find
- congruence closure
- 等价类维护
- 函数应用索引
- disequality 冲突检测
- explanation 预留

---

## 6.6 QF_LRA

基于 LP 求解器实现：

- 理论约束收集
- 增量检查预留
- 冲突解释预留
- 可行模型返回

---

## 6.7 QF_LIA

初期可以做基础版本：

- 线性整数约束识别
- bound reasoning
- 整数可行性基础检查
- 可复用 MILP 的部分能力，但不要强耦合
- 初版不完整时可返回 `Unknown(IncompleteAlgorithm)`

后续增强：

- branch and bound
- cutting planes
- branch and cut
- stronger integer propagation

---

## 6.8 理论冲突解释

SMT 的关键不是只发现理论冲突，而是能解释冲突。

目标：

```text
理论冲突原因
   ↓
返回相关 theory literals
   ↓
生成 SAT learned clause
```

初期可以从弱解释开始，但不能只返回一个无信息的 `false`。

---

## 6.9 本阶段产出

- `SmtSolver`
- `solve_smt(...)`
- `dsuni smt file.smt2`
- QF_UF 初版
- QF_LRA 初版
- QF_LIA 初版
- SAT + theory 交互闭环

---

# 7. 第六阶段：MILP

## 7.1 目标

在线性表达和 LP 求解器稳定后，实现 MILP。

---

## 7.2 范围

支持：

- 整数变量
- 连续变量
- 线性约束
- 线性目标
- 可行性问题
- 优化问题

---

## 7.3 算法路线

初版：

- branch and bound
- LP relaxation
- incumbent 管理
- 上界 / 下界维护
- 节点队列
- 整数性检查

后续：

- branch and cut
- stronger preprocessing
- primal heuristics
- pseudo-cost branching
- strong branching

---

## 7.4 割平面

初版可选基础割：

- Gomory cut
- bound cuts
- simple cover cuts

割平面不应影响正确性。
如果 cut 生成器不确定，宁可不用，不要生成错误 cut。

---

## 7.5 节点管理

支持：

- DFS
- BFS
- best-bound
- time limit
- node limit

---

## 7.6 启发式

初始可实现：

- fractional variable branching
- most fractional
- simple rounding heuristic

后续增强：

- pseudo-cost
- diving
- local branching

---

## 7.7 本阶段产出

- `MilpSolver`
- `solve_milp(...)`
- `dsuni milp file.mps`
- 可解决基础整数线性问题

---

# 8. 第七阶段：CP

## 8.1 目标

实现约束规划求解能力，强调传播、域削减和回溯搜索。

---

## 8.2 变量域

支持：

- 离散整数域
- 区间域
- 布尔域

预留：

- 集合域
- 枚举域

---

## 8.3 传播框架

实现：

- event-driven propagation
- propagator queue
- fixed point iteration
- domain modification log
- failure detection
- reversible state

---

## 8.4 基础约束

优先支持：

- equality
- inequality
- linear
- sum
- element
- all_different

后续：

- cumulative
- circuit
- table
- regular
- no_overlap

---

## 8.5 搜索

实现：

- 回溯搜索
- 变量选择启发式
- 值选择启发式
- propagation after decision
- failure recovery

后续增强：

- conflict-directed backjumping
- nogood learning
- lazy clause generation

---

## 8.6 正确性重点

需要重点测试：

- domain pruning 是否保守正确
- 回溯后域是否完全恢复
- propagation 是否达到固定点
- propagator 是否漏传播
- all_different 是否错误删除可行值

---

## 8.7 本阶段产出

- `CpSolver`
- `solve_cp(...)`
- `dsuni cp file.mzn`
- 可解一批经典 CP 小题

---

# 9. 第八阶段：MINLP

## 9.1 目标

在 SAT、linear、LP、SMT、MILP、CP 都相对稳定之后，再实现 MINLP。

MINLP 不应过早开始。

---

## 9.2 初始范围

只支持有限子集：

- 低阶多项式
- 有界变量
- 部分双线性项
- 部分凸约束
- 中小规模问题

不追求一开始处理通用非凸大规模 MINLP。

---

## 9.3 算法路线

初始采用：

- outer approximation
- branch and bound
- interval bound propagation
- piecewise linear approximation
- McCormick relaxation for bilinear terms
- incumbent 管理

---

## 9.4 非线性处理

支持：

- 乘法项分解
- 区间界定
- 分段线性近似
- 松弛模型构造
- 局部可行性检查预留

---

## 9.5 结果策略

由于 MINLP 初期算法可能不完整，需要严格区分：

- Proven optimal
- Feasible incumbent
- Infeasible
- Unbounded
- Unknown
- Timeout

不能在没有证明时声称全局最优。

---

## 9.6 本阶段产出

- `MinlpSolver`
- `solve_minlp(...)`
- `dsuni minlp file.xxx`
- 可解决部分中小规模非线性整数问题

---

# 10. 统一入口与 CLI

## 10.1 API 入口

不提供自动求解 `solve(...)`。

提供显式入口：

```rust
pub fn solve_sat(input: SatInput, config: Config) -> SolverResult;

pub fn solve_smt(input: SmtInput, config: Config) -> SolverResult;

pub fn solve_lp(input: LpInput, config: Config) -> SolverResult;

pub fn solve_milp(input: MilpInput, config: Config) -> SolverResult;

pub fn solve_cp(input: CpInput, config: Config) -> SolverResult;

pub fn solve_minlp(input: MinlpInput, config: Config) -> SolverResult;
```

---

## 10.2 CLI 入口

```bash
dsuni sat file.cnf
dsuni smt file.smt2
dsuni lp file.lp
dsuni milp file.mps
dsuni cp file.mzn
dsuni minlp file.xxx
```

允许显式指定格式：

```bash
dsuni sat --format dimacs file.txt
dsuni smt --format smtlib file.txt
dsuni lp --format lp file.txt
```

但不根据内容自动切换求解器。

---

## 10.3 统一配置

```rust
pub struct Config {
    pub timeout_ms: Option<u64>,
    pub max_memory_mb: Option<usize>,
    pub verbosity: u8,
    pub random_seed: Option<u64>,
    pub check_model: bool,
    pub produce_proof: bool,
    pub produce_unsat_core: bool,
}
```

其中：

- `produce_proof` 初期可以不实现，但接口预留
- `produce_unsat_core` 初期可以部分支持
- `check_model` 应优先实现，用于验证模型正确性

---

## 10.4 统一统计信息

```rust
pub struct SolverStats {
    pub elapsed_ms: u64,
    pub decisions: u64,
    pub propagations: u64,
    pub conflicts: u64,
    pub restarts: u64,
    pub nodes: u64,
    pub simplex_iterations: u64,
    pub memory_mb: Option<usize>,
}
```

不同模块可以只填充相关字段。

---

## 10.5 本阶段产出

- 统一 API
- 统一 CLI
- 统一配置
- 统一结果格式
- 统一日志系统
- 统一统计信息
- 统一超时 / 中断处理

---

# 11. 测试、验证与回归体系

## 11.1 单元测试

每个模块都要有：

- 最小样例
- 正常样例
- 边界样例
- 退化样例
- 错误输入样例
- 随机样例

---

## 11.2 回归测试

建议题库分类：

- SAT：DIMACS
- SMT：SMT-LIB 小规模 QF_UF/QF_LRA/QF_LIA
- LP：自造 LP + Netlib 小型实例
- MILP：自造小例子 + MIPLIB 小型实例
- CP：MiniZinc 风格经典小题
- MINLP：精选低阶非线性小题

---

## 11.3 属性测试

适合使用 property-based testing 的内容：

- normalize 前后等价性
- 线性表达规范化等价性
- propagation 幂等性
- push/pop 后状态一致性
- 回溯后 assignment/trail 一致性
- SAT 模型满足所有子句
- LP/MILP 模型满足所有约束
- CP 域削减不删除真实解

---

## 11.4 模型检查

所有返回 `Sat`、`Feasible`、`Optimal` 的结果，都应支持独立模型检查：

```rust
check_model(input, model) -> bool
```

这是正确性优先路线中的关键工具。

---

## 11.5 UNSAT / Infeasible 验证预留

初期可以不完整实现 proof，但应预留：

- UNSAT core
- proof trace
- conflict explanation
- infeasibility certificate

SAT 可优先考虑 DRAT/LRAT 输出预留。
LP 可预留 Farkas certificate。
SMT 可预留 theory lemma trace。

---

## 11.6 CI

CI 至少包括：

- cargo test
- cargo clippy
- cargo fmt
- 小型回归集
- 随机种子固定测试
- 模型检查测试

---

# 12. 推荐实际开发顺序

最现实的顺序：

1. `core`
2. `parser`
3. `normalize`
4. `sat`
5. `linear`
6. `lp`
7. `smt`
8. `milp`
9. `cp`
10. `minlp`
11. 统一入口完善
12. 测试 / 验证 / 文档强化

原因：

- SAT 最容易形成完整闭环
- `linear + lp` 是 SMT-LRA、MILP 的公共底座
- SMT 需要 SAT 和 theory solver 框架
- MILP 依赖 LP
- CP 可以在 MILP 之后独立推进
- MINLP 最复杂，应最后实现

---

# 13. 版本里程碑

## v0.1

目标：最小可运行 SAT 项目。

包含：

- core
- parser 初版
- normalize 初版
- SAT CDCL 初版
- DIMACS parser
- 基础 CLI
- 基础测试

---

## v0.2

目标：线性系统和 LP 初版。

包含：

- linear 模块
- Rational / LinearExpr / LinearConstraint
- LP 单纯形初版
- LP parser 初版
- LP 测试集
- 模型检查器初版

---

## v0.3

目标：SMT 基础框架。

包含：

- SAT + theory 框架
- QF_UF
- QF_LRA
- QF_LIA 初版
- SMT-LIB 子集 parser
- theory conflict explanation 初版

---

## v0.4

目标：MILP 初版。

包含：

- branch and bound
- LP relaxation
- integer feasibility check
- incumbent 管理
- 基础 MILP parser
- MILP 小型回归集

---

## v0.5

目标：CP 初版。

包含：

- domain representation
- propagation queue
- reversible state
- backtracking search
- all_different / linear / sum / element
- CP 小型题库

---

## v0.6

目标：MINLP 初版。

包含：

- interval bound propagation
- outer approximation
- piecewise linear relaxation
- branch and bound
- bilinear term relaxation
- 中小规模 MINLP 样例

---

## v0.7

目标：统一入口和工程化。

包含：

- 完整显式 API
- 完整 CLI
- 统一 Config
- 统一 SolverResult
- 统一日志
- 统一统计信息
- 超时 / 中断 / 内存限制

---

## v1.0

目标：可公开使用的稳定版本。

包含：

- 核心模块稳定
- 回归测试完整
- 文档完善
- 示例完善
- 错误诊断清晰
- 结果可检查
- 关键模块具备验证预留
- 不依赖 FFI
- 不做自动求解器选择

---

# 14. 建议代码结构

```text
dsuni/
├── Cargo.toml
├── crates/
│   ├── dsuni-core/
│   ├── dsuni-parser/
│   ├── dsuni-normalize/
│   ├── dsuni-sat/
│   ├── dsuni-linear/
│   ├── dsuni-lp/
│   ├── dsuni-smt/
│   ├── dsuni-milp/
│   ├── dsuni-cp/
│   ├── dsuni-minlp/
│   ├── dsuni-cli/
│   └── dsuni-util/
├── tests/
│   ├── sat/
│   ├── smt/
│   ├── lp/
│   ├── milp/
│   ├── cp/
│   └── minlp/
├── benches/
├── docs/
└── examples/
```

初期也可以先用单 crate，等模块稳定后再拆 workspace。

建议顺序：

1. 单 crate 快速打通
2. 模块边界稳定后拆成 workspace
3. 每个 solver 独立 crate
4. 顶层 `dsuni` crate 只暴露统一 API

---

# 15. 不做事项

为了避免范围失控，明确初期不做：

- 不做 FFI 后端
- 不做自动求解器选择
- 不做通用 NLP
- 不做完整 SMT-LIB 全理论
- 不做完整 MiniZinc
- 不追求一开始性能领先
- 不做复杂 proof system
- 不做大型工业级 presolve
- 不做并行求解

这些可以作为远期研究方向，但不进入早期主路线。

---

# 16. 一句话总结

`dsuni` 的路线是：先用纯 Rust 打牢 `core + SAT + linear + LP`，再构建 SMT 和 MILP，随后补齐 CP，最后谨慎推进 MINLP；所有求解入口显式调用，不自动分类，不依赖 FFI，以正确性、模块化和可验证性为长期核心。
```
