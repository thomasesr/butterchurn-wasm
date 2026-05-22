use std::collections::HashMap;

use super::parser::{Ast, BinOp, Expr, UnaryOp};

pub struct EelEnv {
    pub vars: HashMap<String, f64>,
    pub megabuf: Vec<f64>,
}

impl Default for EelEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl EelEnv {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            megabuf: vec![0.0; 1_000_000],
        }
    }

    pub fn set_audio(
        &mut self,
        bass: f64,
        mid: f64,
        treb: f64,
        vol: f64,
        bass_att: f64,
        mid_att: f64,
        treb_att: f64,
        vol_att: f64,
    ) {
        self.vars.insert("bass".into(), bass);
        self.vars.insert("mid".into(), mid);
        self.vars.insert("treb".into(), treb);
        self.vars.insert("vol".into(), vol);
        self.vars.insert("bass_att".into(), bass_att);
        self.vars.insert("mid_att".into(), mid_att);
        self.vars.insert("treb_att".into(), treb_att);
        self.vars.insert("vol_att".into(), vol_att);
    }

    pub fn set_time(&mut self, time: f64, fps: f64, frame: u64) {
        self.vars.insert("time".into(), time);
        self.vars.insert("fps".into(), fps);
        self.vars.insert("frame".into(), frame as f64);
    }

    pub fn get(&self, name: &str) -> f64 {
        *self.vars.get(name).unwrap_or(&0.0)
    }

    pub fn set(&mut self, name: &str, val: f64) {
        self.vars.insert(name.to_string(), val);
    }

    pub fn get_q_vals(&self) -> [f32; 32] {
        let mut out = [0.0f32; 32];
        for i in 1..=32usize {
            let key = format!("q{}", i);
            out[i - 1] = self.get(&key) as f32;
        }
        out
    }

    pub fn run(&mut self, ast: &Ast, regs: &mut [f64; 100]) -> f64 {
        eval_expr(ast, self, regs)
    }
}

fn truthy(v: f64) -> bool {
    v.abs() > 1e-6
}

fn get_var(name: &str, env: &EelEnv, regs: &[f64; 100]) -> f64 {
    if let Some(idx) = reg_index(name) {
        return regs[idx];
    }
    env.get(name)
}

fn set_var(name: &str, val: f64, env: &mut EelEnv, regs: &mut [f64; 100]) {
    if let Some(idx) = reg_index(name) {
        regs[idx] = val;
        return;
    }
    env.set(name, val);
}

fn reg_index(name: &str) -> Option<usize> {
    if name.len() == 5 && name.starts_with("reg") {
        name[3..].parse::<usize>().ok().filter(|&i| i < 100)
    } else {
        None
    }
}

fn apply_binop(op: BinOp, l: f64, r: f64) -> f64 {
    match op {
        BinOp::Add => l + r,
        BinOp::Sub => l - r,
        BinOp::Mul => l * r,
        BinOp::Div => {
            if r.abs() < 1e-300 {
                0.0
            } else {
                l / r
            }
        }
        BinOp::Mod => {
            if r.abs() < 1e-300 {
                0.0
            } else {
                l % r
            }
        }
        BinOp::Pow => l.powf(r),
        BinOp::Lt => if l < r { 1.0 } else { 0.0 },
        BinOp::Gt => if l > r { 1.0 } else { 0.0 },
        BinOp::BitAnd => ((l as i64) & (r as i64)) as f64,
        BinOp::BitOr => ((l as i64) | (r as i64)) as f64,
    }
}

fn eval_expr(expr: &Expr, env: &mut EelEnv, regs: &mut [f64; 100]) -> f64 {
    match expr {
        Expr::Number(n) => *n,

        Expr::Var(name) => get_var(name, env, regs),

        Expr::Assign { var, val } => {
            let v = eval_expr(val, env, regs);
            set_var(var, v, env, regs);
            v
        }

        Expr::CompoundAssign { var, op, val } => {
            let rhs = eval_expr(val, env, regs);
            let lhs = get_var(var, env, regs);
            let result = apply_binop(*op, lhs, rhs);
            set_var(var, result, env, regs);
            result
        }

        Expr::BinOp { op, lhs, rhs } => {
            let l = eval_expr(lhs, env, regs);
            let r = eval_expr(rhs, env, regs);
            apply_binop(*op, l, r)
        }

        Expr::Unary { op, expr } => {
            let v = eval_expr(expr, env, regs);
            match op {
                UnaryOp::Neg => -v,
                UnaryOp::Not => if truthy(v) { 0.0 } else { 1.0 },
            }
        }

        Expr::Call { name, args } => eval_call(name, args, env, regs),

        Expr::Sequence(stmts) => {
            let mut last = 0.0;
            for s in stmts {
                last = eval_expr(s, env, regs);
            }
            last
        }
    }
}

fn eval_call(name: &str, args: &[Expr], env: &mut EelEnv, regs: &mut [f64; 100]) -> f64 {
    // Lazy-evaluated specials
    match name {
        "loop" => {
            if args.len() < 2 {
                return 0.0;
            }
            let count = eval_expr(&args[0], env, regs).floor() as usize;
            let count = count.min(1024);
            let mut last = 0.0;
            for _ in 0..count {
                last = eval_expr(&args[1], env, regs);
            }
            return last;
        }
        "if" => {
            if args.len() < 3 {
                return 0.0;
            }
            let cond = eval_expr(&args[0], env, regs);
            return if truthy(cond) {
                eval_expr(&args[1], env, regs)
            } else {
                eval_expr(&args[2], env, regs)
            };
        }
        "exec2" => {
            if args.len() < 2 {
                return 0.0;
            }
            eval_expr(&args[0], env, regs);
            return eval_expr(&args[1], env, regs);
        }
        "exec3" => {
            if args.len() < 3 {
                return 0.0;
            }
            eval_expr(&args[0], env, regs);
            eval_expr(&args[1], env, regs);
            return eval_expr(&args[2], env, regs);
        }
        "assign" => {
            if args.len() < 2 {
                return 0.0;
            }
            if let Expr::Var(var_name) = &args[0] {
                let v = eval_expr(&args[1], env, regs);
                set_var(var_name, v, env, regs);
                return v;
            }
            return eval_expr(&args[1], env, regs);
        }
        "megabuf" => {
            if args.is_empty() {
                return 0.0;
            }
            let idx = eval_expr(&args[0], env, regs).floor() as usize;
            return if idx < env.megabuf.len() { env.megabuf[idx] } else { 0.0 };
        }
        "gmegabuf" => {
            if args.is_empty() {
                return 0.0;
            }
            let idx = eval_expr(&args[0], env, regs).floor() as usize;
            return if idx < env.megabuf.len() { env.megabuf[idx] } else { 0.0 };
        }
        _ => {}
    }

    // Eager evaluation for all other functions
    let a: Vec<f64> = args.iter().map(|e| eval_expr(e, env, regs)).collect();
    let a0 = *a.get(0).unwrap_or(&0.0);
    let a1 = *a.get(1).unwrap_or(&0.0);
    let a2 = *a.get(2).unwrap_or(&0.0);

    match name {
        "sin" => a0.sin(),
        "cos" => a0.cos(),
        "tan" => a0.tan(),
        "asin" => a0.clamp(-1.0, 1.0).asin(),
        "acos" => a0.clamp(-1.0, 1.0).acos(),
        "atan" => a0.atan(),
        "atan2" => a0.atan2(a1),
        "sqrt" => if a0 < 0.0 { 0.0 } else { a0.sqrt() },
        "sqr" => a0 * a0,
        "pow" => a0.powf(a1),
        "log" => if a0 <= 0.0 { 0.0 } else { a0.ln() },
        "log10" => if a0 <= 0.0 { 0.0 } else { a0.log10() },
        "exp" => a0.exp(),
        "abs" => a0.abs(),
        "sign" => {
            if a0 > 0.0 { 1.0 } else if a0 < 0.0 { -1.0 } else { 0.0 }
        }
        "floor" => a0.floor(),
        "ceil" => a0.ceil(),
        "int" => a0.trunc(),
        "frac" => a0.fract(),
        "min" => a0.min(a1),
        "max" => a0.max(a1),
        "clamp" => a0.clamp(a1, a2),
        "invsqrt" => if a0 <= 0.0 { 0.0 } else { 1.0 / a0.sqrt() },
        "rand" => {
            let n = a0.floor() as u64;
            if n == 0 { 0.0 } else { fastrand::u64(0..n) as f64 }
        }
        "band" => ((a0 as i64) & (a1 as i64)) as f64,
        "bor" => ((a0 as i64) | (a1 as i64)) as f64,
        "bnot" => (!(a0 as i64)) as f64,
        "equal" => if (a0 - a1).abs() < 1e-6 { 1.0 } else { 0.0 },
        "above" => if a0 > a1 { 1.0 } else { 0.0 },
        "below" => if a0 < a1 { 1.0 } else { 0.0 },
        _ => 0.0,
    }
}
