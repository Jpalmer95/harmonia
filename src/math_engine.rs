//! Symbolic Mathematical Equation & Frequency DSP Engine.
//! Parses human-friendly mathematical audio expressions into an AST and compiles them
//! into a zero-allocation stack bytecode VM (`CompiledExpr`) for sample-accurate Hi-Def synthesis.

use std::f64::consts::{E, PI, TAU};

pub const PHI: f64 = 1.618033988749895;

#[derive(Debug, Clone, Copy)]
pub struct MathContext {
    /// Time since note-on in seconds
    pub t: f64,
    /// Global composition time in seconds
    pub gt: f64,
    /// Fundamental frequency in Hz
    pub f: f64,
    /// Angular phase `2 * pi * f * t`
    pub w: f64,
    /// Note velocity (0.0..=1.0)
    pub vel: f64,
    /// Current ADSR envelope value (0.0..=1.0)
    pub env: f64,
    /// MIDI note number (can be fractional for microtonal)
    pub note: f64,
    /// Voice index within a multiline chord (0, 1, 2, ...)
    pub voice: f64,
    /// Dynamic modulation index (driven by LFO/Macro)
    pub mod_idx: f64,
    /// LFO 1 (-1.0..=1.0, slow)
    pub lfo1: f64,
    /// LFO 2 (-1.0..=1.0, medium)
    pub lfo2: f64,
    /// LFO 3 (-1.0..=1.0, fast/vibrato)
    pub lfo3: f64,
    /// Macro X (0.0..=1.0)
    pub x: f64,
    /// Macro Y (0.0..=1.0)
    pub y: f64,
    /// Macro Z (0.0..=1.0)
    pub z: f64,
}

impl Default for MathContext {
    fn default() -> Self {
        Self {
            t: 0.0,
            gt: 0.0,
            f: 220.0,
            w: 0.0,
            vel: 0.85,
            env: 1.0,
            note: 57.0,
            voice: 0.0,
            mod_idx: 2.0,
            lfo1: 0.0,
            lfo2: 0.0,
            lfo3: 0.0,
            x: 0.5,
            y: 0.5,
            z: 0.5,
        }
    }
}

impl MathContext {
    pub fn for_sample(t: f64, gt: f64, f: f64, vel: f64, env: f64, voice: usize, mod_idx: f64) -> Self {
        let w = TAU * f * t;
        let lfo1 = (TAU * 0.25 * gt).sin();
        let lfo2 = (TAU * 1.2 * gt).sin();
        let lfo3 = (TAU * 5.2 * gt).sin();
        let note = 69.0 + 12.0 * (f.max(1.0) / 440.0).log2();
        Self {
            t,
            gt,
            f,
            w,
            vel,
            env,
            note,
            voice: voice as f64,
            mod_idx,
            lfo1,
            lfo2,
            lfo3,
            x: 0.5 + 0.5 * lfo1,
            y: 0.5 + 0.5 * lfo2,
            z: 0.5 + 0.5 * lfo3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VarId {
    T,
    Gt,
    F,
    W,
    Vel,
    Env,
    Note,
    Voice,
    ModIdx,
    Lfo1,
    Lfo2,
    Lfo3,
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FuncId {
    // 1-arg
    Sin,
    Cos,
    Tan,
    Tanh,
    Atan,
    Sinh,
    Cosh,
    Exp,
    Ln,
    Log2,
    Sqrt,
    Cbrt,
    Abs,
    Sign,
    Floor,
    Ceil,
    Fract,
    Saw,
    Tri,
    Sq,
    // 2-arg
    Pow,
    Mod,
    Min,
    Max,
    Pulse,
    Fold,
    Cheb,
    // 3-arg
    Clamp,
    Lerp,
    Smoothstep,
    Harm,
    Pluck,
    Ks,
    // 4-arg
    Fm,
    Bell,
    // 5-arg
    Chord,
    // 6-arg
    Fm3,
}

#[derive(Debug, Clone, Copy)]
pub enum Op {
    PushConst(f64),
    PushVar(VarId),
    Neg,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Call(FuncId),
}

#[derive(Debug, Clone)]
pub struct CompiledExpr {
    pub source: String,
    pub ops: Vec<Op>,
}

impl CompiledExpr {
    pub fn compile(source: &str) -> Result<Self, String> {
        let tokens = tokenize(source)?;
        let mut parser = Parser {
            tokens: &tokens,
            pos: 0,
        };
        let mut ops = Vec::new();
        parser.parse_expr(&mut ops)?;
        if parser.pos < parser.tokens.len() {
            return Err(format!(
                "Unexpected token {:?} at position {}",
                parser.tokens[parser.pos], parser.pos
            ));
        }
        Ok(Self {
            source: source.to_string(),
            ops,
        })
    }

    #[inline]
    pub fn eval(&self, ctx: &MathContext) -> f64 {
        let mut stack = [0.0_f64; 64];
        let mut sp = 0usize;

        for &op in &self.ops {
            match op {
                Op::PushConst(c) => {
                    if sp < stack.len() {
                        stack[sp] = c;
                        sp += 1;
                    }
                }
                Op::PushVar(v) => {
                    if sp < stack.len() {
                        stack[sp] = match v {
                            VarId::T => ctx.t,
                            VarId::Gt => ctx.gt,
                            VarId::F => ctx.f,
                            VarId::W => ctx.w,
                            VarId::Vel => ctx.vel,
                            VarId::Env => ctx.env,
                            VarId::Note => ctx.note,
                            VarId::Voice => ctx.voice,
                            VarId::ModIdx => ctx.mod_idx,
                            VarId::Lfo1 => ctx.lfo1,
                            VarId::Lfo2 => ctx.lfo2,
                            VarId::Lfo3 => ctx.lfo3,
                            VarId::X => ctx.x,
                            VarId::Y => ctx.y,
                            VarId::Z => ctx.z,
                        };
                        sp += 1;
                    }
                }
                Op::Neg => {
                    if sp >= 1 {
                        stack[sp - 1] = -stack[sp - 1];
                    }
                }
                Op::Add => {
                    if sp >= 2 {
                        let b = stack[sp - 1];
                        stack[sp - 2] += b;
                        sp -= 1;
                    }
                }
                Op::Sub => {
                    if sp >= 2 {
                        let b = stack[sp - 1];
                        stack[sp - 2] -= b;
                        sp -= 1;
                    }
                }
                Op::Mul => {
                    if sp >= 2 {
                        let b = stack[sp - 1];
                        stack[sp - 2] *= b;
                        sp -= 1;
                    }
                }
                Op::Div => {
                    if sp >= 2 {
                        let b = stack[sp - 1];
                        let a = stack[sp - 2];
                        stack[sp - 2] = if b.abs() > 1e-12 { a / b } else { 0.0 };
                        sp -= 1;
                    }
                }
                Op::Rem => {
                    if sp >= 2 {
                        let b = stack[sp - 1];
                        let a = stack[sp - 2];
                        stack[sp - 2] = if b.abs() > 1e-12 { a % b } else { 0.0 };
                        sp -= 1;
                    }
                }
                Op::Pow => {
                    if sp >= 2 {
                        let b = stack[sp - 1];
                        let a = stack[sp - 2];
                        let res = a.powf(b);
                        stack[sp - 2] = if res.is_finite() { res } else { 0.0 };
                        sp -= 1;
                    }
                }
                Op::Call(func) => {
                    eval_func(func, &mut stack, &mut sp);
                }
            }
        }

        if sp > 0 && stack[sp - 1].is_finite() {
            stack[sp - 1]
        } else {
            0.0
        }
    }
}

#[inline]
fn eval_func(func: FuncId, stack: &mut [f64; 64], sp: &mut usize) {
    match func {
        FuncId::Sin if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].sin(),
        FuncId::Cos if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].cos(),
        FuncId::Tan if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].tan().clamp(-10.0, 10.0),
        FuncId::Tanh if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].tanh(),
        FuncId::Atan if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].atan(),
        FuncId::Sinh if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].sinh().clamp(-50.0, 50.0),
        FuncId::Cosh if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].cosh().clamp(-50.0, 50.0),
        FuncId::Exp if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].clamp(-40.0, 20.0).exp(),
        FuncId::Ln if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].abs().max(1e-12).ln(),
        FuncId::Log2 if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].abs().max(1e-12).log2(),
        FuncId::Sqrt if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].max(0.0).sqrt(),
        FuncId::Cbrt if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].cbrt(),
        FuncId::Abs if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].abs(),
        FuncId::Sign if *sp >= 1 => {
            let x = stack[*sp - 1];
            stack[*sp - 1] = if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                0.0
            };
        }
        FuncId::Floor if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].floor(),
        FuncId::Ceil if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].ceil(),
        FuncId::Fract if *sp >= 1 => stack[*sp - 1] = stack[*sp - 1].rem_euclid(1.0),
        FuncId::Saw if *sp >= 1 => {
            // Smooth 5-harmonic band-tempered saw + exact ramp blend over radian phase
            let p = stack[*sp - 1];
            let u = (p / TAU).rem_euclid(1.0);
            let naive = 2.0 * u - 1.0;
            let smooth = p.sin() - 0.5 * (2.0 * p).sin() + (1.0 / 3.0) * (3.0 * p).sin()
                - 0.25 * (4.0 * p).sin();
            stack[*sp - 1] = 0.5 * naive + 0.35 * smooth;
        }
        FuncId::Tri if *sp >= 1 => {
            let p = stack[*sp - 1];
            let u = (p / TAU).rem_euclid(1.0);
            stack[*sp - 1] = 1.0 - 4.0 * (u - 0.5).abs();
        }
        FuncId::Sq if *sp >= 1 => {
            let p = stack[*sp - 1];
            stack[*sp - 1] = (4.5 * p.sin()).tanh();
        }
        FuncId::Pow if *sp >= 2 => {
            let b = stack[*sp - 1];
            let a = stack[*sp - 2];
            stack[*sp - 2] = a.powf(b);
            *sp -= 1;
        }
        FuncId::Mod if *sp >= 2 => {
            let b = stack[*sp - 1];
            let a = stack[*sp - 2];
            stack[*sp - 2] = if b.abs() > 1e-12 { a.rem_euclid(b) } else { 0.0 };
            *sp -= 1;
        }
        FuncId::Min if *sp >= 2 => {
            let b = stack[*sp - 1];
            stack[*sp - 2] = stack[*sp - 2].min(b);
            *sp -= 1;
        }
        FuncId::Max if *sp >= 2 => {
            let b = stack[*sp - 1];
            stack[*sp - 2] = stack[*sp - 2].max(b);
            *sp -= 1;
        }
        FuncId::Pulse if *sp >= 2 => {
            let duty = stack[*sp - 1].clamp(0.05, 0.95);
            let p = stack[*sp - 2];
            let u = (p / TAU).rem_euclid(1.0);
            stack[*sp - 2] = if u < duty { 1.0 } else { -1.0 };
            *sp -= 1;
        }
        FuncId::Fold if *sp >= 2 => {
            let gain = stack[*sp - 1];
            let x = stack[*sp - 2];
            stack[*sp - 2] = (x * gain).sin();
            *sp -= 1;
        }
        FuncId::Cheb if *sp >= 2 => {
            let x = stack[*sp - 1].clamp(-1.0, 1.0);
            let n = (stack[*sp - 2].round() as i32).clamp(0, 16);
            stack[*sp - 2] = chebyshev_t(n as usize, x);
            *sp -= 1;
        }
        FuncId::Clamp if *sp >= 3 => {
            let hi = stack[*sp - 1];
            let lo = stack[*sp - 2];
            let x = stack[*sp - 3];
            stack[*sp - 3] = x.clamp(lo.min(hi), lo.max(hi));
            *sp -= 2;
        }
        FuncId::Lerp if *sp >= 3 => {
            let u = stack[*sp - 1];
            let b = stack[*sp - 2];
            let a = stack[*sp - 3];
            stack[*sp - 3] = a + (b - a) * u;
            *sp -= 2;
        }
        FuncId::Smoothstep if *sp >= 3 => {
            let x = stack[*sp - 1];
            let b = stack[*sp - 2];
            let a = stack[*sp - 3];
            let t = ((x - a) / (b - a).max(1e-9)).clamp(0.0, 1.0);
            stack[*sp - 3] = t * t * (3.0 - 2.0 * t);
            *sp -= 2;
        }
        FuncId::Harm if *sp >= 3 => {
            let rolloff = stack[*sp - 1].max(0.2);
            let count = (stack[*sp - 2].round() as usize).clamp(1, 24);
            let w = stack[*sp - 3];
            let mut sum = 0.0_f64;
            let mut norm = 0.0_f64;
            for k in 1..=count {
                let kf = k as f64;
                let amp = 1.0 / kf.powf(rolloff);
                sum += amp * (kf * w).sin();
                norm += amp;
            }
            stack[*sp - 3] = if norm > 0.0 { sum / norm } else { 0.0 };
            *sp -= 2;
        }
        FuncId::Pluck if *sp >= 3 => {
            let damp = stack[*sp - 1].max(0.1);
            let t = stack[*sp - 2].max(0.0);
            let f = stack[*sp - 3].max(10.0);
            let mut sum = 0.0_f64;
            for k in 1..=10 {
                let kf = k as f64;
                // Subtle string stiffness dispersion
                let fk = f * kf * (1.0 + 0.00015 * kf * kf).sqrt();
                let env = (-t * damp * (0.6 + 0.45 * kf)).exp();
                sum += (env / kf.powf(0.85)) * (TAU * fk * t).sin();
            }
            stack[*sp - 3] = sum * 0.55;
            *sp -= 2;
        }
        FuncId::Ks if *sp >= 3 => {
            let damp = stack[*sp - 1];
            let t = stack[*sp - 2];
            let f = stack[*sp - 3];
            stack[*sp - 3] = crate::karplus::ks_waveform(f, t, damp);
            *sp -= 2;
        }
        FuncId::Fm if *sp >= 4 => {
            let t = stack[*sp - 1];
            let idx = stack[*sp - 2];
            let f_m = stack[*sp - 3];
            let f_c = stack[*sp - 4];
            stack[*sp - 4] = (TAU * f_c * t + idx * (TAU * f_m * t).sin()).sin();
            *sp -= 3;
        }
        FuncId::Bell if *sp >= 4 => {
            let decay = stack[*sp - 1].max(0.05);
            let idx = stack[*sp - 2];
            let t = stack[*sp - 3].max(0.0);
            let f = stack[*sp - 4];
            let mod_env = (-t * decay * 1.8).exp();
            let amp_env = (-t * decay * 0.65).exp();
            let c1 = (TAU * f * t + idx * mod_env * (TAU * f * 1.41421356 * t).sin()).sin();
            let c2 = 0.35 * (TAU * f * 2.756 * t + 0.6 * idx * mod_env * (TAU * f * 3.89 * t).sin()).sin();
            stack[*sp - 4] = amp_env * (0.75 * c1 + 0.25 * c2);
            *sp -= 3;
        }
        FuncId::Chord if *sp >= 5 => {
            let r3 = stack[*sp - 1];
            let r2 = stack[*sp - 2];
            let r1 = stack[*sp - 3];
            let t = stack[*sp - 4];
            let f = stack[*sp - 5];
            let w0 = TAU * f * t;
            stack[*sp - 5] = 0.25 * (w0.sin() + (w0 * r1).sin() + (w0 * r2).sin() + (w0 * r3).sin());
            *sp -= 4;
        }
        FuncId::Fm3 if *sp >= 6 => {
            let t = stack[*sp - 1];
            let idx2 = stack[*sp - 2];
            let r2 = stack[*sp - 3];
            let idx1 = stack[*sp - 4];
            let r1 = stack[*sp - 5];
            let f_c = stack[*sp - 6];
            let op3 = (TAU * f_c * r2 * t).sin();
            let op2 = (TAU * f_c * r1 * t + idx2 * op3).sin();
            stack[*sp - 6] = (TAU * f_c * t + idx1 * op2).sin();
            *sp -= 5;
        }
        _ => {}
    }
}

/// Evaluate Chebyshev polynomial of the first kind $T_n(x)$ iteratively.
#[inline]
pub fn chebyshev_t(n: usize, x: f64) -> f64 {
    match n {
        0 => 1.0,
        1 => x,
        _ => {
            let mut t_prev = 1.0;
            let mut t_curr = x;
            for _ in 2..=n {
                let t_next = 2.0 * x * t_curr - t_prev;
                t_prev = t_curr;
                t_curr = t_next;
            }
            t_curr
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    LParen,
    RParen,
    Comma,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || c == '.' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '_') {
                i += 1;
            }
            let s: String = chars[start..i].iter().filter(|&&ch| ch != '_').collect();
            let val = s
                .parse::<f64>()
                .map_err(|_| format!("Invalid number literal '{s}'"))?;
            tokens.push(Token::Num(val));
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            tokens.push(Token::Ident(ident.to_ascii_lowercase()));
            continue;
        }
        match c {
            '+' => tokens.push(Token::Plus),
            '-' => tokens.push(Token::Minus),
            '*' => tokens.push(Token::Star),
            '/' => tokens.push(Token::Slash),
            '%' => tokens.push(Token::Percent),
            '^' => tokens.push(Token::Caret),
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            ',' => tokens.push(Token::Comma),
            _ => return Err(format!("Unexpected character '{c}' in math expression")),
        }
        i += 1;
    }

    Ok(tokens)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<&'a Token> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn parse_expr(&mut self, ops: &mut Vec<Op>) -> Result<(), String> {
        self.parse_term(ops)?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.next();
                    self.parse_term(ops)?;
                    ops.push(Op::Add);
                }
                Token::Minus => {
                    self.next();
                    self.parse_term(ops)?;
                    ops.push(Op::Sub);
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn parse_term(&mut self, ops: &mut Vec<Op>) -> Result<(), String> {
        self.parse_power(ops)?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Star => {
                    self.next();
                    self.parse_power(ops)?;
                    ops.push(Op::Mul);
                }
                Token::Slash => {
                    self.next();
                    self.parse_power(ops)?;
                    ops.push(Op::Div);
                }
                Token::Percent => {
                    self.next();
                    self.parse_power(ops)?;
                    ops.push(Op::Rem);
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn parse_power(&mut self, ops: &mut Vec<Op>) -> Result<(), String> {
        self.parse_unary(ops)?;
        if let Some(Token::Caret) = self.peek() {
            self.next();
            self.parse_power(ops)?;
            ops.push(Op::Pow);
        }
        Ok(())
    }

    fn parse_unary(&mut self, ops: &mut Vec<Op>) -> Result<(), String> {
        match self.peek() {
            Some(Token::Minus) => {
                self.next();
                self.parse_unary(ops)?;
                ops.push(Op::Neg);
                Ok(())
            }
            Some(Token::Plus) => {
                self.next();
                self.parse_unary(ops)
            }
            _ => self.parse_primary(ops),
        }
    }

    fn parse_primary(&mut self, ops: &mut Vec<Op>) -> Result<(), String> {
        let Some(tok) = self.next() else {
            return Err("Unexpected end of math expression".to_string());
        };
        match tok {
            Token::Num(n) => {
                ops.push(Op::PushConst(*n));
                Ok(())
            }
            Token::LParen => {
                self.parse_expr(ops)?;
                match self.next() {
                    Some(Token::RParen) => Ok(()),
                    _ => Err("Expected closing ')'".to_string()),
                }
            }
            Token::Ident(name) => {
                if let Some(Token::LParen) = self.peek() {
                    self.next(); // consume '('
                    let mut arg_count = 0usize;
                    if !matches!(self.peek(), Some(Token::RParen)) {
                        loop {
                            self.parse_expr(ops)?;
                            arg_count += 1;
                            match self.peek() {
                                Some(Token::Comma) => {
                                    self.next();
                                }
                                Some(Token::RParen) => break,
                                _ => {
                                    return Err(format!(
                                        "Expected ',' or ')' in call to '{name}'"
                                    ))
                                }
                            }
                        }
                    }
                    self.next(); // consume ')'
                    let (func_id, expected_args) = resolve_function(name)?;
                    if arg_count != expected_args {
                        return Err(format!(
                            "Function '{name}' expects {expected_args} argument(s), got {arg_count}"
                        ));
                    }
                    ops.push(Op::Call(func_id));
                    Ok(())
                } else {
                    match name.as_str() {
                        "pi" => ops.push(Op::PushConst(PI)),
                        "tau" => ops.push(Op::PushConst(TAU)),
                        "e" => ops.push(Op::PushConst(E)),
                        "phi" => ops.push(Op::PushConst(PHI)),
                        "t" => ops.push(Op::PushVar(VarId::T)),
                        "gt" => ops.push(Op::PushVar(VarId::Gt)),
                        "f" | "freq" => ops.push(Op::PushVar(VarId::F)),
                        "w" | "phase" => ops.push(Op::PushVar(VarId::W)),
                        "vel" | "v" => ops.push(Op::PushVar(VarId::Vel)),
                        "env" => ops.push(Op::PushVar(VarId::Env)),
                        "note" => ops.push(Op::PushVar(VarId::Note)),
                        "voice" => ops.push(Op::PushVar(VarId::Voice)),
                        "mod_idx" | "idx" | "m" => ops.push(Op::PushVar(VarId::ModIdx)),
                        "lfo1" | "lfo" => ops.push(Op::PushVar(VarId::Lfo1)),
                        "lfo2" => ops.push(Op::PushVar(VarId::Lfo2)),
                        "lfo3" => ops.push(Op::PushVar(VarId::Lfo3)),
                        "x" => ops.push(Op::PushVar(VarId::X)),
                        "y" => ops.push(Op::PushVar(VarId::Y)),
                        "z" => ops.push(Op::PushVar(VarId::Z)),
                        _ => return Err(format!("Unknown variable or constant '{name}'")),
                    }
                    Ok(())
                }
            }
            other => Err(format!("Unexpected token {other:?}")),
        }
    }
}

fn resolve_function(name: &str) -> Result<(FuncId, usize), String> {
    let res = match name {
        "sin" => (FuncId::Sin, 1),
        "cos" => (FuncId::Cos, 1),
        "tan" => (FuncId::Tan, 1),
        "tanh" => (FuncId::Tanh, 1),
        "atan" => (FuncId::Atan, 1),
        "sinh" => (FuncId::Sinh, 1),
        "cosh" => (FuncId::Cosh, 1),
        "exp" => (FuncId::Exp, 1),
        "ln" | "log" => (FuncId::Ln, 1),
        "log2" => (FuncId::Log2, 1),
        "sqrt" => (FuncId::Sqrt, 1),
        "cbrt" => (FuncId::Cbrt, 1),
        "abs" => (FuncId::Abs, 1),
        "sign" | "sgn" => (FuncId::Sign, 1),
        "floor" => (FuncId::Floor, 1),
        "ceil" => (FuncId::Ceil, 1),
        "fract" => (FuncId::Fract, 1),
        "saw" => (FuncId::Saw, 1),
        "tri" => (FuncId::Tri, 1),
        "sq" | "square" => (FuncId::Sq, 1),
        "pow" => (FuncId::Pow, 2),
        "mod" => (FuncId::Mod, 2),
        "min" => (FuncId::Min, 2),
        "max" => (FuncId::Max, 2),
        "pulse" => (FuncId::Pulse, 2),
        "fold" => (FuncId::Fold, 2),
        "cheb" | "chebyshev" => (FuncId::Cheb, 2),
        "clamp" => (FuncId::Clamp, 3),
        "lerp" | "mix" => (FuncId::Lerp, 3),
        "smoothstep" => (FuncId::Smoothstep, 3),
        "harm" | "harmonics" => (FuncId::Harm, 3),
        "pluck" | "string" => (FuncId::Pluck, 3),
        "ks" | "karplus" => (FuncId::Ks, 3),
        "fm" => (FuncId::Fm, 4),
        "bell" => (FuncId::Bell, 4),
        "chord" => (FuncId::Chord, 5),
        "fm3" => (FuncId::Fm3, 6),
        _ => return Err(format!("Unknown function '{name}()'")),
    };
    Ok(res)
}

#[derive(Debug, Clone)]
pub struct MathFormulaPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub formula: &'static str,
    pub description: &'static str,
}

pub fn builtin_math_formulas() -> Vec<MathFormulaPreset> {
    vec![
        MathFormulaPreset {
            id: "chowning_fm_brass",
            name: "Chowning Modulated FM Brass",
            category: "FM & Classical-Modern",
            formula: "env * fm3(f, 1.0, mod_idx * (0.4 + 0.6 * env), 2.0, 0.85 * env, t)",
            description: "3-operator cascaded FM where modulation depth blooms with the note envelope and slow LFO mod_idx.",
        },
        MathFormulaPreset {
            id: "glass_cathedral_morph",
            name: "Glass Cathedral Harmonic Morph",
            category: "FM & Classical-Modern",
            formula: "env * (0.65 * fm(f, f * 2.0, mod_idx * (0.6 + 0.4 * lfo1), t) + 0.35 * fm(f * 1.5, f * 3.0, 1.2 * exp(-t * 1.8), t))",
            description: "Dual-branch crystalline FM with octave + fifth partials and slow LFO modulation of harmonic depth.",
        },
        MathFormulaPreset {
            id: "golden_phi_cosmos",
            name: "Golden Ratio Phi Spectral Choir",
            category: "Mathematical & Microtonal",
            formula: "env * (0.5 * sin(w + 1.2 * (0.5 + 0.5 * lfo1) * sin(w * phi)) + 0.3 * sin(w * 2.0 + 0.8 * sin(w / phi)) + 0.2 * harm(w, 7, 1.4))",
            description: "Inharmonic golden-ratio (phi = 1.618034) sidebands blended with a 7-partial harmonic series.",
        },
        MathFormulaPreset {
            id: "chebyshev_warm_organ",
            name: "Chebyshev Polynomial Harmonic Organ",
            category: "Polynomial Waveshaping",
            formula: "env * (0.45 * cheb(1, sin(w)) + 0.25 * (0.5 + 0.5 * lfo1) * cheb(3, sin(w)) + 0.18 * (0.5 + 0.5 * lfo2) * cheb(5, sin(w)) + 0.12 * cheb(7, sin(w)))",
            description: "Exact odd-order harmonic excitation via Chebyshev polynomials T_1, T_3, T_5, T_7 with LFO crossfading.",
        },
        MathFormulaPreset {
            id: "buchla_wavefolder_lead",
            name: "West-Coast Buchla Wavefolder + Sub",
            category: "Wavefolding & Chaos",
            formula: "env * (0.7 * fold(sin(w) + 0.35 * sin(w * 2.002), 1.5 + mod_idx * (0.6 + 0.4 * lfo1)) + 0.3 * sin(w * 0.5))",
            description: "West-coast sine wavefolder where fold gain is driven by mod_idx and LFO1, anchored by a warm sub-octave.",
        },
        MathFormulaPreset {
            id: "risset_spectral_bell",
            name: "Risset-Chowning Spectral Bell",
            category: "Physical & Spectral",
            formula: "bell(f, t, mod_idx * 1.4, 0.9) + 0.3 * pluck(f * 2.0, t, 1.5)",
            description: "Classic CCRMA inharmonic FM bell combined with a modal plucked string overtone layer.",
        },
        MathFormulaPreset {
            id: "karplus_lute",
            name: "Karplus–Strong Lute / Harp",
            category: "Physical Models",
            formula: "env * (0.72 * ks(f, t, 0.55) + 0.28 * sin(w * 0.5))",
            description: "Plucked-string physical model with a warm sub-octave sine for lute, harp, and guitar transients.",
        },
    ]
}
