# dsuni

**纯 Rust 模块化约束求解器 / Modular Constraint Solver in Pure Rust**

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange)](https://rust-lang.org)
[![Python](https://img.shields.io/badge/Python-3.8%2B-blue)](https://python.org)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)
[![Tests](https://img.shields.io/badge/Tests-150%20passed-brightgreen)]()

覆盖 8 个求解器模块，全部纯 Rust 实现，无外部求解器依赖。提供 Rust 库、CLI 命令行和 Python wheel 三种使用方式。

### 求解器模块

| 模块 | 算法 | 输入格式 |
|------|------|----------|
| **SAT** | CDCL + Two-Watched Literals + VSIDS | DIMACS CNF |
| **LP** | 两阶段单纯形法（精确有理数） | 自定义文本格式 |
| **SMT** | CDCL(T) + Congruence Closure（QF_UF）| SMT-LIB |
| **MILP** | 分支定界（LP Relaxation + McCormick）| 自定义文本格式 |
| **CP** | 传播 + 回溯（First-Fail 搜索）| 自定义文本格式 |
| **MINLP** | 区间传播 + McCormick 松弛 | 自定义文本格式 |

### 安装

#### Rust
```bash
git clone https://github.com/MBpanzz/dsuni.git
cd dsuni
cargo build --release
cargo test  # 150 tests
```

#### Python（从源码）
```bash
pip install maturin
maturin build --release
pip install target/wheels/dsuni-*.whl
```

#### Python（从 PyPI）
```bash
pip install dsuni
```

### 快速开始

#### CLI
```bash
# SAT
dsuni sat test_data/unsat_unit.cnf
# → UNSAT

# LP
dsuni lp test_data/simple.lp
# → OPTIMAL (obj=-5) / Model: [x0 -> 5]

# MILP
dsuni milp test_data/simple_milp.lp
# → OPTIMAL (obj=-5) / Model: [x0 -> 5]
```

#### Python
```python
import dsuni

# SAT (DIMACS CNF)
r = dsuni.solve_sat("p cnf 1 2\n1 0\n-1 0\n")
print(r)  # {'status': 'UNSAT'}

# LP
r = dsuni.solve_lp_text("min -1*x0\nx0 <= 5\nx0 >= 0\n")
print(r)  # {'status': 'OPTIMAL', 'model': {'x0': 5}, 'objective': -5.0}

# MILP
r = dsuni.solve_milp_text("int x0\nmin -1*x0\nx0 <= 5\nx0 >= 0\n")
print(r)  # {'status': 'OPTIMAL', 'model': {'x0': 5}, 'objective': -5.0}

# CP
r = dsuni.solve_cp_text("var x 0 10\nvar y 0 10\neq x y\n")
print(r)  # {'status': 'SAT', 'model': {'x0': 0, 'x1': 0}}
```

### 项目结构

```
src/
├── core/     → 基础类型（Var / Term / Config / SolverResult）
├── sat/      → CDCL SAT 求解器
├── linear/   → Rational / 线性表达式 / 线性系统
├── lp/       → 两阶段单纯形法
├── milp/     → 分支定界 MILP
├── smt/      → CDCL(T) + QF_UF / QF_LRA
├── cp/       → 传播 + 回溯 CP
├── minlp/    → 区间传播 + McCormick 松弛
├── parser/   → DIMACS / LP / SMT-LIB / CP 格式解析器
├── cli/      → CLI 命令定义
└── python.rs → PyO3 Python 绑定
test_data/    → 14 个测试输入文件
```

### 许可证

MIT License © 2025 MBpanzz
