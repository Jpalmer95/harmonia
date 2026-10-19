//! Yamaha DX7 32-algorithm operator graphs (public domain routing tables).
//! Each algorithm is a 6×6 modulation matrix: `matrix[src][dst]` is 1.0 when
//! operator `src` (1-indexed in Yamaha docs, 0-indexed here) modulates `dst`.
//! Carriers are operators whose output goes to the mix bus.

#[derive(Debug, Clone, Copy)]
pub struct Dx7Algorithm {
    pub number: u8,
    pub name: &'static str,
    pub carriers: &'static [usize],
    pub matrix: [[f64; 6]; 6],
    pub feedback_op: usize,
}

const fn empty_matrix() -> [[f64; 6]; 6] {
    [[0.0; 6]; 6]
}

const fn edge(mut m: [[f64; 6]; 6], src: usize, dst: usize) -> [[f64; 6]; 6] {
    m[src][dst] = 1.0;
    m
}

pub fn algorithm(n: u8) -> Dx7Algorithm {
    let n = n.clamp(1, 32);
    match n {
        1 => alg(
            1,
            "Stack 6>5>4>3>2>1",
            &[0],
            edges(&[(5, 4), (4, 3), (3, 2), (2, 1), (1, 0)]),
            5,
        ),
        2 => alg(
            2,
            "6>5>4>3>2>1 (fb on 2)",
            &[0],
            edges(&[(5, 4), (4, 3), (3, 2), (2, 1), (1, 0)]),
            1,
        ),
        3 => alg(
            3,
            "Dual stack (6>5>4) + (3>2>1)",
            &[0, 3],
            edges(&[(2, 1), (1, 0), (5, 4), (4, 3)]),
            5,
        ),
        4 => alg(
            4,
            "Dual stack fb on 4",
            &[0, 3],
            edges(&[(2, 1), (1, 0), (5, 4), (4, 3)]),
            3,
        ),
        5 => alg(
            5,
            "Three pairs (2>1) (4>3) (6>5)",
            &[0, 2, 4],
            edges(&[(1, 0), (3, 2), (5, 4)]),
            5,
        ),
        6 => alg(
            6,
            "Three pairs fb on 6",
            &[0, 2, 4],
            edges(&[(1, 0), (3, 2), (5, 4)]),
            5,
        ),
        7 => alg(
            7,
            "(2>1) + ((4+5+6)>3)",
            &[0, 2],
            edges(&[(1, 0), (3, 2), (4, 2), (5, 2)]),
            5,
        ),
        8 => alg(
            8,
            "(2>1) + ((4+5+6)>3) fb 4",
            &[0, 2],
            edges(&[(1, 0), (3, 2), (4, 2), (5, 2)]),
            3,
        ),
        9 => alg(
            9,
            "(2>1) + ((4+5+6)>3) fb 2",
            &[0, 2],
            edges(&[(1, 0), (3, 2), (4, 2), (5, 2)]),
            1,
        ),
        10 => alg(
            10,
            "(3>2>1) + ((5+6)>4)",
            &[0, 3],
            edges(&[(2, 1), (1, 0), (4, 3), (5, 3)]),
            5,
        ),
        11 => alg(
            11,
            "(3>2>1) + ((5+6)>4) fb 3",
            &[0, 3],
            edges(&[(2, 1), (1, 0), (4, 3), (5, 3)]),
            2,
        ),
        12 => alg(
            12,
            "((2+3+4+5)>1) + 6",
            &[0, 5],
            edges(&[(1, 0), (2, 0), (3, 0), (4, 0)]),
            1,
        ),
        13 => alg(
            13,
            "((2+3+4+5)>1) + 6 fb 6",
            &[0, 5],
            edges(&[(1, 0), (2, 0), (3, 0), (4, 0)]),
            5,
        ),
        14 => alg(
            14,
            "((3+4)>2>1) + ((6)>5)",
            &[0, 4],
            edges(&[(2, 1), (3, 1), (1, 0), (5, 4)]),
            5,
        ),
        15 => alg(
            15,
            "((3+4)>2>1) + (6>5) fb 2",
            &[0, 4],
            edges(&[(2, 1), (3, 1), (1, 0), (5, 4)]),
            1,
        ),
        16 => alg(
            16,
            "((2)+(4>3)+(6>5))>1",
            &[0],
            edges(&[(1, 0), (3, 2), (2, 0), (5, 4), (4, 0)]),
            5,
        ),
        17 => alg(
            17,
            "((2)+(4>3)+(6>5))>1 fb 2",
            &[0],
            edges(&[(1, 0), (3, 2), (2, 0), (5, 4), (4, 0)]),
            1,
        ),
        18 => alg(
            18,
            "(2>1) + (3) + ((5+6)>4)",
            &[0, 2, 3],
            edges(&[(1, 0), (4, 3), (5, 3)]),
            5,
        ),
        19 => alg(
            19,
            "(2>1) + (4>3) + (5) + 6",
            &[0, 2, 4, 5],
            edges(&[(1, 0), (3, 2)]),
            5,
        ),
        20 => alg(
            20,
            "((2+3)>1) + ((5+6)>4)",
            &[0, 3],
            edges(&[(1, 0), (2, 0), (4, 3), (5, 3)]),
            5,
        ),
        21 => alg(
            21,
            "((2+3)>1) + (4) + ((6)>5)",
            &[0, 3, 4],
            edges(&[(1, 0), (2, 0), (5, 4)]),
            5,
        ),
        22 => alg(
            22,
            "(2>1) + (3) + (4) + ((6)>5)",
            &[0, 2, 3, 4],
            edges(&[(1, 0), (5, 4)]),
            5,
        ),
        23 => alg(
            23,
            "(2>1) + (3) + (4) + (5) + 6",
            &[0, 2, 3, 4, 5],
            edges(&[(1, 0)]),
            5,
        ),
        24 => alg(
            24,
            "((3+4+5+6)>2>1)",
            &[0],
            edges(&[(2, 1), (3, 1), (4, 1), (5, 1), (1, 0)]),
            5,
        ),
        25 => alg(
            25,
            "((2+3+4+5+6)>1)",
            &[0],
            edges(&[(1, 0), (2, 0), (3, 0), (4, 0), (5, 0)]),
            5,
        ),
        26 => alg(
            26,
            "(2>1) + ((4+5+6)>3)",
            &[0, 2],
            edges(&[(1, 0), (3, 2), (4, 2), (5, 2)]),
            4,
        ),
        27 => alg(
            27,
            "((3>2>1)) + ((5+6)>4)",
            &[0, 3],
            edges(&[(2, 1), (1, 0), (4, 3), (5, 3)]),
            4,
        ),
        28 => alg(
            28,
            "(2>1) + (4>3) + (6>5)",
            &[0, 2, 4],
            edges(&[(1, 0), (3, 2), (5, 4)]),
            4,
        ),
        29 => alg(
            29,
            "(2>1) + (3) + (5>4) + 6",
            &[0, 2, 3, 5],
            edges(&[(1, 0), (4, 3)]),
            4,
        ),
        30 => alg(
            30,
            "(2>1) + (3) + (4) + (6>5)",
            &[0, 2, 3, 4],
            edges(&[(1, 0), (5, 4)]),
            4,
        ),
        31 => alg(
            31,
            "(2>1) + 3 + 4 + 5 + 6",
            &[0, 2, 3, 4, 5],
            edges(&[(1, 0)]),
            4,
        ),
        _ => alg(32, "All carriers (organ)", &[0, 1, 2, 3, 4, 5], empty_matrix(), 5),
    }
}

fn edges(pairs: &[(usize, usize)]) -> [[f64; 6]; 6] {
    let mut m = empty_matrix();
    for &(s, d) in pairs {
        m = edge(m, s, d);
    }
    m
}

fn alg(number: u8, name: &'static str, carriers: &'static [usize], matrix: [[f64; 6]; 6], feedback_op: usize) -> Dx7Algorithm {
    Dx7Algorithm {
        number,
        name,
        carriers,
        matrix,
        feedback_op,
    }
}

pub fn all_algorithms() -> Vec<Dx7Algorithm> {
    (1..=32).map(algorithm).collect()
}

/// Apply a DX7 algorithm onto an existing Harmonia patch:
/// copy the routing graph into `mod_matrix` (scaled by `index`) and mark carriers.
pub fn apply_to_patch(patch: &mut crate::fm_synth::FmPatch, algo: u8, index: f64) {
    let a = algorithm(algo);
    patch.algorithm_name = format!("DX7 Algorithm {} — {}", a.number, a.name);
    for r in 0..6 {
        for c in 0..6 {
            patch.mod_matrix[r][c] = a.matrix[r][c] * index;
        }
    }
    for op in &mut patch.operators {
        op.enabled = true;
        op.output_level = 0.0;
    }
    let share = 0.85 / (a.carriers.len() as f64).sqrt();
    for (i, &c) in a.carriers.iter().enumerate() {
        patch.operators[c].output_level = share;
        let pan = if a.carriers.len() == 1 {
            0.0
        } else {
            (i as f64 / (a.carriers.len() - 1) as f64) * 1.2 - 0.6
        };
        patch.operators[c].pan = pan;
    }
    patch.operators[a.feedback_op].feedback = 0.35;
}
