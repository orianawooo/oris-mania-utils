use std::collections::BTreeMap;

use super::RawChartNote;

#[derive(Clone, Copy, Debug)]
struct XNote {
    k: usize,
    h: i32,
    t: i32,
}

#[derive(Clone, Copy, Debug)]
struct XRow {
    t: f64,
    mask: u32,
    size: i32,
}

#[derive(Clone, Copy, Debug)]
struct HandNote {
    t: f64,
    f: usize,
    tail: f64,
}

#[derive(Default)]
struct Strokes {
    t: Vec<f64>,
    width: Vec<f64>,
    size: Vec<i32>,
    mask: Vec<u32>,
    first: Vec<usize>,
}

struct WristCost {
    cost: Vec<f64>,
    cost_manip: Vec<f64>,
    able: Vec<f64>,
    manip: Vec<f64>,
    jitw: Vec<f64>,
}

struct HoldCosts {
    rel: Vec<f64>,
    weight: Vec<f64>,
    cross: Vec<f64>,
}

struct BinnedStrain {
    bin: Vec<i32>,
    dp: Vec<f64>,
}

const WINDOW_MS: f64 = 500.0;
const LN_TAP_MS: f64 = 80.0;
const TECH_WINDOW_MS: f64 = 244.92;
const TECH_SCALE: f64 = 0.001;
const MOTION_X_COEF: f64 = 0.39104;
const MOTION_W: f64 = 5.7561;
const MOTION_REPEAT_W: f64 = 0.55187;
const FINGER_MIX_P: f64 = 2.3762;
const FLAM_MS: f64 = 11.247;
const FINGER_JACK_CRIT_MS: f64 = 218.21;
const JACK_FLOOR_MS: f64 = 35.809;
const MINIJACK_MS: f64 = 72.836;
const LN_HOLD_OCC: f64 = 0.754;
const ALT_CRIT_MS: f64 = 21.197;
const WRIST_W: f64 = 3.3377;
const WRIST_CHORD_W: f64 = 0.81919;
const WRIST_GAP_POW: f64 = 2.6413;
const WRIST_GAP_PIVOT_MS: f64 = 100.0;
const STROKE_CAP_MS: f64 = 1000.0;
const STROKE_TIE_FRAC: f64 = 0.125;
const CHORD_PHASE: f64 = 0.26009;
const ANCHOR_RUN_LO: f64 = 16.0;
const ANCHOR_RUN_HI: f64 = 48.0;
const MANIP_TOL_MS: f64 = 76.009;
const MANIP_TOL_POW: f64 = 2.6;
const MANIP_WINDOW: usize = 12;
const MANIP_PERIOD_MAX: usize = 12;
const MANIP_PERIOD_COST: f64 = 0.02;
const MANIP_PACE: f64 = 0.25;
const MANIP_PACE_FLOOR_MS: f64 = 4.0;
const MANIP_FLAM_MS: f64 = 10.0;
const MANIP_CONS_LO: f64 = 0.15;
const MANIP_CONS_HI: f64 = 0.75;
const WRIST_BLEND_MS: f64 = 64.443;
const JITTER_ONSET_MS: f64 = 95.0;
const JITTER_POW: f64 = 1.5;
const JITTER_REP_LO: f64 = 3.0;
const JITTER_REP_HI: f64 = 7.0;
const STAMINA_GAIN: f64 = 2.2908;
const STAMINA_PROP: f64 = 0.90996;
const STAMINA_CAP: f64 = 1.6526;
const STAMINA_TANK: f64 = 50.918;
const STAMINA_REFILL_REST_MS: f64 = 5000.0;
const STAMINA_REFILL_LEVEL_MS: f64 = 15000.0;
const STAM_INTERVAL_MS: f64 = 500.0;
const STAMINA_DURABILITY: f64 = 0.029925;
const STAMINA_LEVEL_POW: f64 = 5.0;
const STAMINA_EASY: f64 = 0.17545;
const STAMINA_EASY_WINDOW_MS: f64 = 4000.0;
const HAND_P_SYNC: f64 = 6.1324;
const HAND_P_ALT: f64 = 2.526;
const SWITCH_MIX: f64 = 0.16864;
const SWITCH_WINDOW_MS: f64 = 500.0;
const LN_WEIGHT: f64 = 0.05;
const REL_GAIN: f64 = 0.75;
const REL_MID: f64 = 0.75;
const REL_GAP_WEIGHT: f64 = 0.5;
const LX_GAIN: f64 = 0.8;
const LX_RAMP_MS: f64 = 150.0;
const LX_STACK: f64 = 1.0;
const RATING_SCALE: f64 = 7.528655953984781;
const AGG_POWER: f64 = 3.8724;
const AGG_TARGET_LOSS: f64 = 0.02;
const SOLVE_STEPS: usize = 40;
const NOTE_DAMP_N: f64 = 60.0;
const SCALE_A: f64 = 30.0;
const SCALE_C: f64 = 290.0;
const GRID_MS: f64 = 25.0;
const DUP_MS: i32 = 10;
const LN_W: f64 = 3.997;
const LN_X: f64 = 1.3275;

const HOLD_PRESS_W: [[f64; 4]; 4] = [
    [0.00, 1.00, 0.20, 0.15],
    [1.00, 0.00, 0.20, 0.20],
    [0.20, 0.20, 0.00, 1.00],
    [0.15, 0.20, 1.00, 0.00],
];

const DAN_LNR0: f64 = 6.14;
const DAN_STEP: f64 = 0.01;
const DAN_SLOPE_LO: f64 = 1.3636185681423285;
const DAN_SLOPE_HI: f64 = 3.997036617938226;
const DAN_KNOT: [f64; 263] = [
    1.695971286,
    1.710070004,
    1.726156061,
    1.74431758,
    1.764518862,
    1.786724452,
    1.810899133,
    1.837007933,
    1.865016118,
    1.894889196,
    1.926592916,
    1.960093267,
    1.995356482,
    2.032349033,
    2.071037632,
    2.111389234,
    2.153371034,
    2.196950469,
    2.242095217,
    2.288773195,
    2.336952564,
    2.386601724,
    2.437689317,
    2.490184226,
    2.544055575,
    2.599272728,
    2.655805291,
    2.713623112,
    2.772696278,
    2.83299512,
    2.894490206,
    2.957152348,
    3.0209526,
    3.085862253,
    3.151852842,
    3.218896144,
    3.286964174,
    3.35602919,
    3.426063691,
    3.497040416,
    3.568932347,
    3.641712705,
    3.715354954,
    3.789832796,
    3.865120178,
    3.941191285,
    4.018020544,
    4.095582625,
    4.173852435,
    4.252805125,
    4.332416087,
    4.412660954,
    4.493515598,
    4.574956134,
    4.656958918,
    4.739500547,
    4.822557858,
    4.90610793,
    4.990128083,
    5.074595879,
    5.159489118,
    5.244785844,
    5.330464341,
    5.416503135,
    5.502880991,
    5.589576917,
    5.676570161,
    5.763840213,
    5.851366802,
    5.939129902,
    6.027109723,
    6.11528672,
    6.203641587,
    6.292155261,
    6.380808918,
    6.469583976,
    6.558462094,
    6.647425171,
    6.73645535,
    6.825535011,
    6.914646779,
    7.003773517,
    7.092898331,
    7.182004567,
    7.271075813,
    7.360095896,
    7.449048888,
    7.537919097,
    7.626691076,
    7.715349617,
    7.803879755,
    7.892266764,
    7.98049616,
    8.068553699,
    8.156425381,
    8.244097444,
    8.331556367,
    8.418788873,
    8.505781924,
    8.592522722,
    8.678998713,
    8.765129072,
    8.847562069,
    8.92525146,
    8.99922594,
    9.070405297,
    9.139606951,
    9.207552277,
    9.274872708,
    9.342115621,
    9.409750021,
    9.47817201,
    9.547710055,
    9.618630059,
    9.691140229,
    9.765395753,
    9.841503289,
    9.919525256,
    9.999483957,
    10.08136551,
    10.16512359,
    10.25068303,
    10.33794323,
    10.42678139,
    10.51705558,
    10.60860768,
    10.7012661,
    10.79484844,
    10.88916389,
    10.98401555,
    11.07920262,
    11.17452236,
    11.26977201,
    11.36475051,
    11.45926015,
    11.55310799,
    11.64610728,
    11.73807865,
    11.82885126,
    11.91826377,
    12.00616524,
    12.09241594,
    12.17688797,
    12.25946587,
    12.34004706,
    12.41854226,
    12.49487574,
    12.56898551,
    12.64082346,
    12.71035534,
    12.77756075,
    12.84243296,
    12.90497877,
    12.96521816,
    13.02318401,
    13.07892167,
    13.13248849,
    13.18395331,
    13.23339588,
    13.28090625,
    13.32658407,
    13.37054314,
    13.41316551,
    13.45468367,
    13.49516651,
    13.53468145,
    13.5732945,
    13.61107021,
    13.6480717,
    13.68436066,
    13.71999732,
    13.75504049,
    13.78954752,
    13.82357435,
    13.85717546,
    13.89040388,
    13.92331124,
    13.9559477,
    13.98836198,
    14.02060138,
    14.05271174,
    14.08473748,
    14.11672156,
    14.14870553,
    14.18072948,
    14.21283205,
    14.24505047,
    14.27742051,
    14.30997652,
    14.34275138,
    14.37577656,
    14.40908208,
    14.44269651,
    14.47664701,
    14.51095928,
    14.54565757,
    14.58076472,
    14.6163021,
    14.65228968,
    14.68874594,
    14.72568797,
    14.76313139,
    14.80109039,
    14.83957773,
    14.87860471,
    14.91818121,
    14.95831567,
    14.99901507,
    15.04028497,
    15.0821295,
    15.12455132,
    15.16755168,
    15.21113037,
    15.25528576,
    15.30001477,
    15.34531288,
    15.39117413,
    15.43759113,
    15.48455505,
    15.5320556,
    15.58008107,
    15.62861832,
    15.67765275,
    15.72716833,
    15.77714759,
    15.82757162,
    15.87842008,
    15.92967118,
    15.98130169,
    16.03328695,
    16.08560085,
    16.13821585,
    16.19110297,
    16.24423178,
    16.29757043,
    16.35108562,
    16.40474261,
    16.45850522,
    16.51233583,
    16.56619539,
    16.6200434,
    16.67383793,
    16.72753561,
    16.78109163,
    16.83445972,
    16.88759221,
    16.94043996,
    16.9929524,
    17.04507754,
    17.09676191,
    17.14795064,
    17.19858739,
    17.24861441,
    17.2979725,
    17.346601,
    17.39443785,
    17.44141951,
    17.48748103,
    17.53255602,
    17.57657663,
    17.61947358,
    17.66117617,
    17.70162357,
];

// reform dan mean anchors mapped across tiers 4..20
const XDC1_DAN_MEANS: [f64; 17] = [
    3.74, 5.62, 6.54, 7.44, 9.00, 9.45, 10.30, 11.28, 12.04, 13.11, 13.97, 15.12, 15.71, 16.98,
    18.21, 19.00, 19.60,
];

pub fn points_to_raw(points: f64) -> f64 {
    if points > 0.0 {
        SCALE_C * (points / SCALE_A).exp_m1()
    } else {
        0.0
    }
}

pub fn raw_to_dan(raw: f64) -> f64 {
    if raw.is_nan() || raw <= 0.0 {
        return 0.0;
    }
    let x = (raw.ln() - DAN_LNR0) / DAN_STEP;
    let last = (DAN_KNOT.len() - 1) as f64;
    if x <= 0.0 {
        return (DAN_KNOT[0] + DAN_SLOPE_LO * DAN_STEP * x).max(0.0);
    }
    if x >= last {
        return DAN_KNOT[DAN_KNOT.len() - 1] + DAN_SLOPE_HI * DAN_STEP * (x - last);
    }
    let i = x as usize;
    let f = x - i as f64;
    DAN_KNOT[i] + f * (DAN_KNOT[i + 1] - DAN_KNOT[i])
}

fn hit_window_unit() -> f64 {
    let x = 0.3 * ((64.5 - (9.0_f64 * 3.0).ceil()) / 500.0).sqrt();
    x.min(0.6 * (x - 0.09) + 0.09)
}

fn first_not_before(a: &[f64], v: f64) -> usize {
    a.partition_point(|&x| x < v)
}

fn first_after(a: &[f64], v: f64) -> usize {
    a.partition_point(|&x| x <= v)
}

fn column_count(mask: u32) -> i32 {
    ((mask & 1) + ((mask >> 1) & 1) + ((mask >> 2) & 1) + ((mask >> 3) & 1)) as i32
}

fn window_integral(x: &[f64], f: &[f64], window: f64, mut span: Option<&mut Vec<f64>>) -> Vec<f64> {
    let n = x.len();
    let mut out = vec![0.0; n];
    if let Some(ref mut s) = span {
        s.clear();
        s.resize(n, 0.0);
    }
    if n < 2 {
        return out;
    }
    let mut cap_f = vec![0.0; n];
    for i in 1..n {
        cap_f[i] = cap_f[i - 1] + f[i - 1] * (x[i] - x[i - 1]);
    }
    let mut ca = 0usize;
    let mut cb = 0usize;
    let x_front = x[0];
    let x_back = x[n - 1];
    for i in 0..n {
        let a = (x[i] - window).clamp(x_front, x_back);
        let b = (x[i] + window).clamp(x_front, x_back);
        while ca < n && x[ca] < a {
            ca += 1;
        }
        while cb < n && x[cb] < b {
            cb += 1;
        }
        let ia = ca.saturating_sub(1);
        let ib = cb.saturating_sub(1);
        out[i] = (cap_f[ib] + f[ib] * (b - x[ib])) - (cap_f[ia] + f[ia] * (a - x[ia]));
        if let Some(ref mut s) = span {
            s[i] = b - a;
        }
    }
    out
}

fn window_sum(x: &[f64], f: &[f64], window: f64, scale: f64) -> Vec<f64> {
    let mut out = window_integral(x, f, window, None);
    for v in &mut out {
        *v *= scale;
    }
    out
}

fn window_avg(x: &[f64], f: &[f64], window: f64) -> Vec<f64> {
    let mut span = Vec::new();
    let mut out = window_integral(x, f, window, Some(&mut span));
    for i in 0..out.len() {
        out[i] = if span[i] > 0.0 { out[i] / span[i] } else { 0.0 };
    }
    out
}

fn add_span(f: &mut [f64], grid: &[f64], t0: f64, t1: f64, val: f64) {
    if grid.len() < 2 || t1 <= t0 {
        return;
    }
    let mut q = first_after(grid, t0).saturating_sub(1);
    while q + 1 < grid.len() && grid[q] < t1 {
        let lo = t0.max(grid[q]);
        let hi = t1.min(grid[q + 1]);
        if hi > lo {
            f[q] += val * (hi - lo) / (grid[q + 1] - grid[q]);
        }
        q += 1;
    }
}

fn build_rows(notes: &[XNote]) -> Vec<XRow> {
    let mut rows: Vec<XRow> = Vec::new();
    for n in notes {
        let nh = n.h as f64;
        if rows.last().map(|r| r.t != nh).unwrap_or(true) {
            rows.push(XRow {
                t: nh,
                mask: 0,
                size: 0,
            });
        }
        if let Some(back) = rows.last_mut() {
            back.mask |= 1u32 << n.k;
            back.size = column_count(back.mask);
        }
    }
    rows
}

fn soften_gap(gap_ms: f64) -> f64 {
    (gap_ms * gap_ms + FLAM_MS * FLAM_MS).sqrt()
}

fn gap_rate(delta_ms: f64, x_term: f64) -> f64 {
    let d = 0.001 * delta_ms;
    if d > 0.0 {
        (1.0 / d) * (1.0 / (d + x_term))
    } else {
        0.0
    }
}

fn smoothstep01(u: f64) -> f64 {
    let c = u.clamp(0.0, 1.0);
    c * c * (3.0 - 2.0 * c)
}

fn minijack_gaps(t: &[f64]) -> Vec<f64> {
    if t.len() <= 1 {
        return Vec::new();
    }
    let mut g = vec![0.0; t.len() - 1];
    for i in 0..g.len() {
        g[i] = t[i + 1] - t[i];
    }
    let mut out = g.clone();
    for i in 0..g.len() {
        if g[i] >= MINIJACK_MS {
            continue;
        }
        let before = if i > 0 {
            (0.5 * (g[i - 1] - g[i])).max(0.0)
        } else {
            MINIJACK_MS
        };
        let after = if i + 1 < g.len() {
            (0.5 * (g[i + 1] - g[i])).max(0.0)
        } else {
            MINIJACK_MS
        };
        out[i] = (g[i] + before + after).min(MINIJACK_MS);
    }
    out
}

fn repeat_cost(v: &[HandNote], grid: &[f64], rice: bool) -> Vec<f64> {
    let mut ft: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    for n in v {
        ft[n.f].push(n.t);
    }
    let sg = [minijack_gaps(&ft[0]), minijack_gaps(&ft[1])];
    let mut idx = [0usize, 0usize];
    let mut rate = vec![0.0; grid.len()];
    let mut last = [-1.0_f64, -1.0_f64];
    let mut free_at = [0.0_f64, 0.0_f64];
    for n in v {
        let l = last[n.f];
        if l >= 0.0 && n.t > l {
            let raw = sg[n.f][idx[n.f] - 1];
            let gap = soften_gap(raw - (LN_HOLD_OCC * free_at[n.f]).min(raw - 1.0));
            let c = FINGER_JACK_CRIT_MS / (gap * gap + JACK_FLOOR_MS * JACK_FLOOR_MS).sqrt();
            let pen = c * c;
            add_span(
                &mut rate,
                grid,
                l,
                n.t,
                (1.0 + pen) * gap_rate(gap, 0.0) * raw / (n.t - l),
            );
        }
        idx[n.f] += 1;
        last[n.f] = n.t;
        free_at[n.f] = if !rice && n.tail >= n.t {
            (n.tail - n.t - LN_TAP_MS).max(0.0)
        } else {
            0.0
        };
    }
    window_sum(grid, &rate, TECH_WINDOW_MS, TECH_SCALE)
}

fn motion_cost(x: f64, v: &[HandNote], grid: &[f64]) -> Vec<f64> {
    let mut t: Vec<f64> = Vec::new();
    let mut m: Vec<u32> = Vec::new();
    for n in v {
        if let Some(&last_t) = t.last() {
            if last_t == n.t {
                if let Some(last_m) = m.last_mut() {
                    *last_m |= 1u32 << n.f;
                }
                continue;
            }
        }
        t.push(n.t);
        m.push(1u32 << n.f);
    }
    let xt = MOTION_X_COEF * x.powf(0.25);
    let mut rate = vec![0.0; grid.len()];
    for i in 0..t.len().saturating_sub(1) {
        let gap = soften_gap(t[i + 1] - t[i]);
        let w = if m[i + 1] == 3 {
            1.0
        } else if (m[i + 1] & m[i]) != 0 {
            MOTION_REPEAT_W
        } else {
            let ratio = ALT_CRIT_MS / gap;
            1.0 + ratio * ratio
        };
        add_span(&mut rate, grid, t[i], t[i + 1], w * gap_rate(gap, xt));
    }
    window_sum(grid, &rate, TECH_WINDOW_MS, TECH_SCALE)
}

fn finger_mix(j: f64, s: f64) -> f64 {
    let a = j;
    let b = MOTION_W * s;
    let hi = a.max(b);
    if hi <= 0.0 {
        return 0.0;
    }
    hi * (1.0 + (a.min(b) / hi).powf(FINGER_MIX_P)).powf(1.0 / FINGER_MIX_P)
}

fn group_strokes(v: &[HandNote]) -> Strokes {
    let mut s = Strokes::default();
    let mut ft: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    for i in 0..v.len() {
        let chord = (i > 0 && v[i - 1].t == v[i].t) || (i + 1 < v.len() && v[i + 1].t == v[i].t);
        if !chord {
            ft[v[i].f].push(v[i].t);
        }
    }
    let mut i = 0usize;
    while i < v.len() {
        let mut j = i + 1;
        if j < v.len()
            && v[j].f != v[i].f
            && v[j].t - v[i].t <= STROKE_CAP_MS
            && !(j + 1 < v.len()
                && v[j + 1].f == v[i].f
                && v[j + 1].t - v[j].t < (v[j].t - v[i].t) * (1.0 - STROKE_TIE_FRAC))
        {
            j += 1;
        }
        s.t.push(0.5 * (v[i].t + v[j - 1].t));
        let mut w = v[j - 1].t - v[i].t;
        if j - i == 1 {
            let o = &ft[1 - v[i].f];
            let mut d = STROKE_CAP_MS;
            let q = first_not_before(o, v[i].t);
            if q < o.len() {
                d = d.min(o[q] - v[i].t);
            }
            if q > 0 {
                d = d.min(v[i].t - o[q - 1]);
            }
            w = if d < STROKE_CAP_MS { d } else { 0.0 };
        }
        s.width.push(w);
        s.size.push((j - i) as i32);
        s.first.push(i);
        let mut m = 0u32;
        for item in &v[i..j] {
            m |= 1u32 << item.f;
        }
        s.mask.push(m);
        i = j;
    }
    s
}

fn note_consistency(nt: &[f64], nc: &[i32], i: usize) -> f64 {
    let n = nt.len();
    let a = i.saturating_sub(MANIP_WINDOW);
    let b = n.min(i + MANIP_WINDOW + 1);
    let mut best = 0.0_f64;
    for p in 1..=MANIP_PERIOD_MAX {
        if b < a + p + 4 {
            break;
        }
        let mut miss = 0.0;
        let mut cnt = 0usize;
        let mut q = a;
        while q + p < b {
            if q + p + 1 >= b {
                break;
            }
            cnt += 1;
            if nc[q] != nc[q + p] || nc[q + 1] != nc[q + p + 1] {
                miss += 1.0;
                q += 1;
                continue;
            }
            let g1 = nt[q + 1] - nt[q];
            let g2 = nt[q + p + 1] - nt[q + p];
            let med = 0.5 * (g1 + g2);
            let tol = (MANIP_PACE * med * MANIP_PACE * med
                + MANIP_PACE_FLOOR_MS * MANIP_PACE_FLOOR_MS)
                .sqrt();
            miss += smoothstep01((g1 - g2).abs() / tol);
            q += 1;
        }
        if cnt > 0 {
            best = best.max(1.0 - miss / (cnt as f64) - MANIP_PERIOD_COST * ((p - 1) as f64));
        }
    }
    best
}

fn stroke_consistency(hv: &[Vec<HandNote>; 2], s: &[Strokes; 2]) -> [Vec<f64>; 2] {
    #[derive(Clone, Copy)]
    struct Ev {
        t: f64,
        c: i32,
        h: usize,
        q: usize,
    }
    let mut all: Vec<Ev> = Vec::new();
    for (h, hand) in hv.iter().enumerate() {
        for (q, n) in hand.iter().enumerate() {
            all.push(Ev {
                t: n.t,
                c: (2 * h + n.f) as i32,
                h,
                q,
            });
        }
    }
    all.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap().then(a.c.cmp(&b.c)));
    let mut e = 0usize;
    while e < all.len() {
        let mut f = e + 1;
        let base_t = all[e].t;
        while f < all.len() && all[f].t - base_t <= MANIP_FLAM_MS {
            all[f].t = base_t;
            f += 1;
        }
        all[e..f].sort_by_key(|x| x.c);
        e = f;
    }
    let nt: Vec<f64> = all.iter().map(|x| x.t).collect();
    let nc: Vec<i32> = all.iter().map(|x| x.c).collect();
    let mut per = [vec![0.0; hv[0].len()], vec![0.0; hv[1].len()]];
    for idx in 0..all.len() {
        per[all[idx].h][all[idx].q] = note_consistency(&nt, &nc, idx);
    }
    let mut out = [Vec::new(), Vec::new()];
    for h in 0..2 {
        out[h].resize(s[h].t.len(), 0.0);
        for k in 0..out[h].len() {
            let mut sum = 0.0;
            let sz = s[h].size[k] as usize;
            for q in 0..sz {
                sum += per[h][s[h].first[k] + q];
            }
            out[h][k] = sum / (sz as f64);
        }
    }
    out
}

fn wrist_cost(v: &[HandNote], s: &Strokes, cons: &[f64], other: &[f64], grid: &[f64]) -> WristCost {
    let n = s.t.len();
    let mut chord = vec![0.0; n];
    let mut manip = vec![0.0; n];
    let mut strike = vec![0.0; n];
    for k in 0..n {
        let mut period = 0.0_f64;
        if k > 0 {
            period = period.max(s.t[k] - s.t[k - 1]);
        }
        if k + 1 < n {
            period = period.max(s.t[k + 1] - s.t[k]);
        }
        let phase = if s.width[k] <= 0.0 {
            0.0
        } else if period > 0.0 {
            s.width[k] / period
        } else {
            1.0
        };
        let near = 1.0 - smoothstep01(phase / CHORD_PHASE);
        let pair = s.size[k] == 2 && s.width[k] > 0.0;
        chord[k] = if pair { 0.0 } else { near };
        let lo_t = s.t[k] - 0.5 * s.width[k];
        let hi_t = s.t[k] + 0.5 * s.width[k];
        let q = first_not_before(other, lo_t);
        let across = q < other.len() && other[q] > lo_t && other[q] < hi_t;
        manip[k] = if pair && !across {
            1.0 - (1.0 - near)
                * (1.0 - smoothstep01((cons[k] - MANIP_CONS_LO) / (MANIP_CONS_HI - MANIP_CONS_LO)))
        } else {
            0.0
        };
        let merged = 1.0 - (1.0 - chord[k]) * (1.0 - manip[k]);
        strike[k] = if s.size[k] == 1 {
            1.0
        } else {
            smoothstep01((merged - 0.2) / 0.6)
        };
    }
    let mut run = vec![0.0_f64; n];
    for f in [1u32, 2u32] {
        let mut fw = vec![0.0; n];
        let mut bw = vec![0.0; n];
        for k in 0..n {
            if (s.mask[k] & f) != 0 {
                let prev = if k > 0 && (s.mask[k - 1] & f) != 0 {
                    fw[k - 1]
                } else {
                    0.0
                };
                fw[k] = strike[k] * (1.0 + prev);
            }
        }
        for k in (0..n).rev() {
            if (s.mask[k] & f) != 0 {
                let next = if k + 1 < n && (s.mask[k + 1] & f) != 0 {
                    bw[k + 1]
                } else {
                    0.0
                };
                bw[k] = strike[k] * (1.0 + next);
            }
        }
        for k in 0..n {
            if (s.mask[k] & f) != 0 {
                run[k] = run[k].max(fw[k] + bw[k] - strike[k]);
            }
        }
    }
    let mut rep = vec![1.0_f64; n];
    {
        let mut rt: Vec<f64> = Vec::new();
        let mut rm: Vec<u32> = Vec::new();
        let mut row_of = vec![0usize; v.len()];
        for (q, note) in v.iter().enumerate() {
            if rt.last().map(|&x| x != note.t).unwrap_or(true) {
                rt.push(note.t);
                rm.push(0);
            }
            if let Some(back) = rm.last_mut() {
                *back |= 1u32 << note.f;
            }
            row_of[q] = rt.len() - 1;
        }
        let nr = rt.len();
        let mut jacked = vec![false; nr];
        for k in 0..nr {
            jacked[k] = (k > 0 && rm[k - 1] == rm[k]) || (k + 1 < nr && rm[k + 1] == rm[k]);
        }
        let pace = |g1: f64, g2: f64| -> f64 {
            1.0 - smoothstep01(g1.max(g2) / g1.min(g2).max(1.0) - 2.0)
        };
        let mut same = vec![0.0; nr];
        for k in 1..nr {
            if rm[k] == rm[k - 1] || (jacked[k] && jacked[k - 1] && rm[k] != 3 && rm[k - 1] != 3) {
                let g1 = if k >= 2 {
                    rt[k - 1] - rt[k - 2]
                } else {
                    rt[k] - rt[k - 1]
                };
                same[k] = pace(g1, rt[k] - rt[k - 1]);
            }
        }
        let mut fw = vec![0.0; nr];
        let mut bw = vec![0.0; nr];
        for k in 1..nr {
            fw[k] = same[k] * (1.0 + fw[k - 1]);
        }
        for k in (1..nr).rev() {
            bw[k - 1] = same[k] * (1.0 + bw[k]);
        }
        let mut sm = vec![0.0; n];
        let mut sf = vec![0.0; n];
        let mut sb = vec![0.0; n];
        for (k, slot) in sm.iter_mut().enumerate().skip(1) {
            if s.mask[k] == s.mask[k - 1] {
                let g1 = if k >= 2 {
                    s.t[k - 1] - s.t[k - 2]
                } else {
                    s.t[k] - s.t[k - 1]
                };
                *slot = pace(g1, s.t[k] - s.t[k - 1]);
            }
        }
        for k in 1..n {
            sf[k] = sm[k] * (1.0 + sf[k - 1]);
        }
        for k in (1..n).rev() {
            sb[k - 1] = sm[k] * (1.0 + sb[k]);
        }
        for k in 0..n {
            let r = row_of[s.first[k]];
            rep[k] = 1.0 + (fw[r] + bw[r]).max(sf[k] + sb[k]);
        }
    }
    let mut rate = vec![0.0; grid.len()];
    let mut rate_m = vec![0.0; grid.len()];
    let mut able = vec![0.0; grid.len()];
    let mut mcurve = vec![0.0; grid.len()];
    let mut jitw = vec![0.0; grid.len()];
    for i in 0..n.saturating_sub(1) {
        let gap = s.t[i + 1] - s.t[i];
        let r = if (s.mask[i + 1] & s.mask[i]) != 0 {
            smoothstep01((run[i] - ANCHOR_RUN_LO) / (ANCHOR_RUN_HI - ANCHOR_RUN_LO))
        } else {
            0.0
        };
        let a = 1.0 - (1.0 - chord[i]) * (1.0 - r);
        add_span(&mut able, grid, s.t[i], s.t[i + 1], a);
        add_span(&mut mcurve, grid, s.t[i], s.t[i + 1], manip[i]);
        let j = smoothstep01((rep[i] - JITTER_REP_LO) / (JITTER_REP_HI - JITTER_REP_LO));
        let share = if gap > 0.0 && gap < JITTER_ONSET_MS {
            (gap / JITTER_ONSET_MS).powf(JITTER_POW)
        } else {
            1.0
        };
        add_span(
            &mut jitw,
            grid,
            s.t[i],
            s.t[i + 1],
            j * ((1.0 - share) / 0.1).min(1.0),
        );
        let load = 1.0 + WRIST_CHORD_W * ((s.size[i + 1] - 1) as f64);
        let off = if s.size[i + 1] == 2 {
            s.width[i + 1] / MANIP_TOL_MS
        } else {
            0.0
        };
        let c = load
            * gap_rate(gap, 0.0)
            * if gap > 0.0 {
                (WRIST_GAP_PIVOT_MS / gap).powf(WRIST_GAP_POW - 2.0)
            } else {
                1.0
            }
            * (1.0 - j * (1.0 - share));
        add_span(&mut rate, grid, s.t[i], s.t[i + 1], c);
        add_span(
            &mut rate_m,
            grid,
            s.t[i],
            s.t[i + 1],
            c * (1.0 + off.powf(MANIP_TOL_POW)),
        );
    }
    let mut cost = window_sum(grid, &rate, TECH_WINDOW_MS, TECH_SCALE);
    let mut cost_manip = window_sum(grid, &rate_m, TECH_WINDOW_MS, TECH_SCALE);
    for c in &mut cost {
        *c *= WRIST_W;
    }
    for c in &mut cost_manip {
        *c *= WRIST_W;
    }
    WristCost {
        cost,
        cost_manip,
        able: window_avg(grid, &able, WRIST_BLEND_MS),
        manip: window_avg(grid, &mcurve, WRIST_BLEND_MS),
        jitw: window_avg(grid, &jitw, WRIST_BLEND_MS),
    }
}

fn hand_cost(f: f64, w: &WristCost, i: usize) -> f64 {
    let wa = w.able[i].clamp(0.0, 1.0);
    let wm = w.manip[i].clamp(0.0, 1.0);
    let h = (1.0 - wa) * ((1.0 - wm) * f + wm * f.min(w.cost_manip[i])) + wa * w.cost[i];
    let j = w.jitw[i].clamp(0.0, 1.0);
    (1.0 - j) * h + j * h.min(w.cost_manip[i])
}

type HandCurves = ([Vec<f64>; 2], [Vec<f64>; 2], [Vec<f64>; 2]);

fn technique_strain(x: f64, by_col: &[Vec<XNote>; 4], grid: &[f64]) -> HandCurves {
    let mut hv: [Vec<HandNote>; 2] = [Vec::new(), Vec::new()];
    let mut strokes: [Strokes; 2] = [Strokes::default(), Strokes::default()];
    for (h, hand) in hv.iter_mut().enumerate() {
        for (f, col) in by_col[(2 * h)..(2 * h + 2)].iter().enumerate() {
            for n in col {
                hand.push(HandNote {
                    t: n.h as f64,
                    f,
                    tail: n.t as f64,
                });
            }
        }
        hand.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap().then(a.f.cmp(&b.f)));
        strokes[h] = group_strokes(hand);
    }
    let cons = stroke_consistency(&hv, &strokes);
    let mut out = [vec![0.0; grid.len()], vec![0.0; grid.len()]];
    let mut rice = [vec![0.0; grid.len()], vec![0.0; grid.len()]];
    let mut able = [vec![0.0; grid.len()], vec![0.0; grid.len()]];
    for h in 0..2 {
        let v = &hv[h];
        let j_full = repeat_cost(v, grid, false);
        let s_cost = motion_cost(x, v, grid);
        let other: Vec<f64> = hv[1 - h].iter().map(|n| n.t).collect();
        let w = wrist_cost(v, &strokes[h], &cons[h], &other, grid);
        let j_rice = repeat_cost(v, grid, true);
        for i in 0..grid.len() {
            out[h][i] = hand_cost(finger_mix(j_full[i], s_cost[i]), &w, i);
            rice[h][i] = hand_cost(finger_mix(j_rice[i], s_cost[i]), &w, i);
            let wa = w.able[i].clamp(0.0, 1.0);
            let wm = w.manip[i].clamp(0.0, 1.0);
            able[h][i] = wa + (1.0 - wa) * wm;
        }
    }
    (out, able, rice)
}

fn map_level(grid: &[f64], strain: &[Vec<f64>; 2], w: &[Vec<f64>; 2]) -> f64 {
    let n = strain[0].len();
    let both: Vec<f64> = (0..n)
        .map(|i| 0.5 * (strain[0][i] + strain[1][i]))
        .collect();
    let g = window_avg(grid, &both, STAMINA_EASY_WINDOW_MS);
    let mut acc = 0.0;
    let mut tw = 0.0;
    for wh in w {
        for i in 0..n {
            if g[i].is_nan() || g[i] <= 0.0 {
                continue;
            }
            let wd = g[i].powf(STAMINA_LEVEL_POW) * wh[i];
            acc += wd * g[i];
            tw += wd;
        }
    }
    if tw <= 0.0 || acc <= 0.0 {
        return 0.0;
    }
    let easy = STAMINA_EASY * acc / tw;
    let mut acc5 = 0.0;
    let mut tw5 = 0.0;
    for h in 0..2 {
        for i in 0..n {
            if g[i] < easy {
                continue;
            }
            let d = strain[h][i];
            let d2 = d * d;
            acc5 += d2 * d2 * d * w[h][i];
            tw5 += w[h][i];
        }
    }
    if tw5 <= 0.0 || acc5 <= 0.0 {
        return 0.0;
    }
    (acc5 / tw5).powf(0.2)
}

fn run_tank(grid: &[f64], strain: &[f64], level: f64) -> Vec<f64> {
    let mut mod_curve = vec![1.0; strain.len()];
    let mut tank = 1.0_f64;
    let mut prev = 0.0_f64;
    let mut work = 0.0_f64;
    for i in 0..strain.len() {
        let step = if i == 0 {
            0.0
        } else {
            (grid[i] - grid[i - 1]) / STAM_INTERVAL_MS
        };
        let avs = 0.5 * (prev + strain[i]);
        prev = strain[i];
        let rel = avs / level;
        if rel > 1.0 {
            tank -= (rel - 1.0) * step / STAMINA_TANK;
        } else {
            let tau = STAMINA_REFILL_REST_MS + STAMINA_REFILL_LEVEL_MS * rel * rel;
            tank += (1.0 - tank) * (1.0 - (-step * STAM_INTERVAL_MS / tau).exp());
        }
        tank = tank.max(0.0);
        work += rel * step * STAM_INTERVAL_MS / 60000.0;
        mod_curve[i] =
            (1.0 + (STAMINA_CAP - 1.0) * (1.0 - tank)) * (1.0 + STAMINA_DURABILITY * work.sqrt());
    }
    mod_curve
}

fn apply_stamina(grid: &[f64], strain: &mut [Vec<f64>; 2], w: &[Vec<f64>; 2]) {
    let level = STAMINA_PROP * map_level(grid, strain, w);
    if level.is_nan() || level <= 0.0 {
        return;
    }
    for sh in strain.iter_mut() {
        let m = run_tank(grid, sh, level);
        for (val, &mv) in sh.iter_mut().zip(&m) {
            *val *= (1.0 + STAMINA_GAIN * (mv - 1.0)).max(0.0);
        }
    }
}

fn hand_sync(able: &[Vec<f64>; 2], rows: &[XRow], grid: &[f64]) -> Vec<f64> {
    let mut s = vec![0.0; able[0].len()];
    let mut sw = vec![0.0; grid.len()];
    let mut pt: Vec<f64> = Vec::new();
    let mut ch: Vec<f64> = Vec::new();
    for r in 1..rows.len() {
        let a = rows[r - 1].mask;
        let b = rows[r].mask;
        if rows[r - 1].size != 1 || rows[r].size != 1 {
            continue;
        }
        pt.push(rows[r].t);
        ch.push(if ((a & 3) != 0) != ((b & 3) != 0) {
            1.0
        } else {
            0.0
        });
    }
    let mut cum = vec![0.0; ch.len() + 1];
    for k in 0..ch.len() {
        cum[k + 1] = cum[k] + ch[k];
    }
    let mut lo = 0usize;
    let mut hi = 0usize;
    for i in 0..grid.len() {
        while lo < pt.len() && pt[lo] < grid[i] - SWITCH_WINDOW_MS {
            lo += 1;
        }
        while hi < pt.len() && pt[hi] <= grid[i] + SWITCH_WINDOW_MS {
            hi += 1;
        }
        sw[i] = if hi > lo {
            (cum[hi] - cum[lo]) / ((hi - lo) as f64)
        } else {
            0.0
        };
    }
    for i in 0..s.len() {
        let a = (0.5 * (able[0][i] + able[1][i])).clamp(0.0, 1.0);
        s[i] = ((1.0 - SWITCH_MIX) * a + SWITCH_MIX * (1.0 - sw[i])).clamp(0.0, 1.0);
    }
    s
}

fn combine_hands(strain: &[Vec<f64>; 2], sync: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; strain[0].len()];
    for i in 0..out.len() {
        let a = strain[0][i].max(0.0);
        let b = strain[1][i].max(0.0);
        let hi = a.max(b);
        if hi <= 0.0 {
            continue;
        }
        if a.min(b) <= 0.0 {
            out[i] = hi;
            continue;
        }
        let s = sync[i].clamp(0.0, 1.0);
        let p = HAND_P_ALT + (HAND_P_SYNC - HAND_P_ALT) * s;
        let lo = a.min(b) / hi;
        out[i] = hi * (1.0 + lo.powf(p)).powf(1.0 / p);
    }
    out
}

fn hold_costs(x: f64, by_col: &[Vec<XNote>; 4], note_seq: &[XNote], grid: &[f64]) -> HoldCosts {
    let mut tail_seq: Vec<XNote> = note_seq.iter().copied().filter(|n| n.t >= 0).collect();
    tail_seq.sort_by_key(|n| n.t);
    let mut r_step = vec![0.0; grid.len()];
    let mut w_step = vec![0.0; grid.len()];
    let mut x_step = vec![0.0; grid.len()];
    let inv_x = 1.0 / x;
    let sigmoid = |v: f64| 1.0 / (1.0 + (-5.0 * (v - REL_MID)).exp());
    let col_times: [Vec<f64>; 4] = [
        by_col[0].iter().map(|n| n.h as f64).collect(),
        by_col[1].iter().map(|n| n.h as f64).collect(),
        by_col[2].iter().map(|n| n.h as f64).collect(),
        by_col[3].iter().map(|n| n.h as f64).collect(),
    ];
    let mut rel: Vec<XNote> = Vec::new();
    let mut i_list: Vec<f64> = Vec::new();
    for n in &tail_seq {
        let len = (n.t - n.h) as f64;
        if len <= LN_TAP_MS {
            continue;
        }
        let mut h_j = 1e9_f64;
        let times = &col_times[n.k];
        let idx = first_not_before(times, n.h as f64);
        if idx + 1 < times.len() {
            h_j = times[idx + 1];
        }
        let s_h = sigmoid(0.001 * (len - LN_TAP_MS) * inv_x);
        let s_t = sigmoid(0.001 * (h_j - (n.t as f64)).max(0.0) * inv_x);
        rel.push(*n);
        i_list.push((s_h + REL_GAP_WEIGHT * s_t) / (1.0 + REL_GAP_WEIGHT));
    }
    for i in 0..rel.len().saturating_sub(1) {
        let t_start = rel[i].t as f64;
        let t_end = rel[i + 1].t as f64;
        if t_end <= t_start {
            continue;
        }
        let delta_r = 0.001 * (t_end - t_start);
        let val =
            REL_GAIN * 0.08 / delta_r.sqrt() * inv_x * (1.0 + 0.8 * (i_list[i] + i_list[i + 1]));
        add_span(&mut r_step, grid, t_start, t_end, val);
    }
    for n in note_seq {
        if n.t < 0 {
            continue;
        }
        let th = n.h as f64;
        add_span(&mut w_step, grid, th, th + 1.0, 1000.0 * LN_WEIGHT);
    }
    let holds: [Vec<XNote>; 4] = [
        by_col[0].iter().copied().filter(|n| n.t >= 0).collect(),
        by_col[1].iter().copied().filter(|n| n.t >= 0).collect(),
        by_col[2].iter().copied().filter(|n| n.t >= 0).collect(),
        by_col[3].iter().copied().filter(|n| n.t >= 0).collect(),
    ];
    let mut cur = [0usize; 4];
    for press in note_seq {
        let t = press.h as f64;
        let mut sum = 0.0_f64;
        let mut worst = 0.0_f64;
        for k in 0..4 {
            if k == press.k {
                continue;
            }
            let hk = &holds[k];
            while cur[k] < hk.len() && (hk[cur[k]].t as f64) < t {
                cur[k] += 1;
            }
            if cur[k] >= hk.len() {
                continue;
            }
            let h = hk[cur[k]];
            let d = (t - (h.h as f64)).min((h.t as f64) - t);
            if d <= 0.0 {
                continue;
            }
            let pair = HOLD_PRESS_W[k][press.k] * (d / LX_RAMP_MS).min(1.0);
            sum += pair;
            worst = worst.max(pair);
        }
        if sum <= 0.0 {
            continue;
        }
        let acc = worst + LX_STACK * (sum - worst);
        add_span(&mut x_step, grid, t, t + 1.0, 1000.0 * LX_GAIN * acc);
    }
    HoldCosts {
        rel: window_sum(grid, &r_step, WINDOW_MS, 0.001),
        weight: window_sum(grid, &w_step, WINDOW_MS, 0.001),
        cross: window_sum(grid, &x_step, WINDOW_MS, 0.001),
    }
}

fn bin_strain(strain: &[f64]) -> BinnedStrain {
    let mut bin = vec![-1i32; strain.len()];
    let mut dp = Vec::new();
    let lo = (1e-4_f64).ln();
    let step = (1.01_f64).ln();
    let mut groups: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for (i, &s) in strain.iter().enumerate() {
        if s > 1e-4 {
            let key = ((s.ln() - lo) / step) as i32;
            groups.entry(key).or_default().push(i);
        }
    }
    for (_key, idx) in groups {
        let sum: f64 = idx.iter().map(|&i| strain[i]).sum();
        let b_idx = dp.len() as i32;
        for &i in &idx {
            bin[i] = b_idx;
        }
        dp.push((sum / (idx.len() as f64)).powf(AGG_POWER));
    }
    BinnedStrain { bin, dp }
}

fn solve_skill(bw: &[f64], dp: &[f64], mean_w: f64) -> f64 {
    let target = AGG_TARGET_LOSS * mean_w;
    let mut cap = 0.0_f64;
    let mut dmax = 0.0_f64;
    for b in 0..bw.len() {
        if bw[b] > 0.0 {
            cap += bw[b];
            dmax = dmax.max(dp[b]);
        }
    }
    if cap <= target || dmax <= 0.0 {
        return 0.0;
    }
    let mut lo = dmax.ln() - 200.0;
    let mut hi = dmax.ln() + 60.0;
    for _ in 0..SOLVE_STEPS {
        let mid = 0.5 * (lo + hi);
        let inv = (-mid).exp();
        let mut loss = 0.0;
        for b in 0..bw.len() {
            if bw[b] > 0.0 {
                loss += bw[b] * (1.0 - (-0.05 * dp[b] * inv).exp());
            }
        }
        if loss > target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (0.5 * (lo + hi) / AGG_POWER).exp()
}

fn rating_from_bins(pd: &BinnedStrain, w: &[f64], total_notes: f64) -> f64 {
    let total_w: f64 = w.iter().sum();
    if total_w <= 0.0 {
        return 0.0;
    }
    let mut bw = vec![0.0; pd.dp.len()];
    for i in 0..w.len() {
        if pd.bin[i] >= 0 {
            bw[pd.bin[i] as usize] += w[i];
        }
    }
    let mut raw = solve_skill(&bw, &pd.dp, total_w);
    raw *= total_notes / (total_notes + NOTE_DAMP_N);
    raw * 0.975 * RATING_SCALE
}

fn note_weights(grid: &[f64], notes: &[XNote]) -> Vec<f64> {
    let ht: Vec<f64> = notes.iter().map(|n| n.h as f64).collect();
    let n = grid.len();
    let mut w = vec![0.0; n];
    if n < 2 {
        return w;
    }
    let mut lo = 0usize;
    let mut hi = 0usize;
    for i in 0..n {
        while lo < ht.len() && ht[lo] < grid[i] - WINDOW_MS {
            lo += 1;
        }
        while hi < ht.len() && ht[hi] < grid[i] + WINDOW_MS {
            hi += 1;
        }
        let gap = if i == 0 {
            (grid[1] - grid[0]) / 2.0
        } else if i == n - 1 {
            (grid[n - 1] - grid[n - 2]) / 2.0
        } else {
            (grid[i + 1] - grid[i - 1]) / 2.0
        };
        w[i] = ((hi - lo) as f64) * gap;
    }
    w
}

fn rating_points(raw: f64) -> f64 {
    if raw > 0.0 {
        SCALE_A * (raw / SCALE_C).ln_1p()
    } else {
        0.0
    }
}

pub fn analyse_xdc1(raw_notes: &[RawChartNote], rate: f32) -> (f64, f64, f64) {
    let (dan, pts, pts_rice, _, _) = analyse_xdc1_with_curve(raw_notes, rate);
    (dan, pts, pts_rice)
}

fn downsample_strain_curve(rested_all: &[f64], bins: usize) -> Vec<f32> {
    if rested_all.is_empty() || bins == 0 {
        return Vec::new();
    }
    let pts: Vec<f64> = rested_all
        .iter()
        .map(|&s| rating_points(s * 0.975 * RATING_SCALE))
        .collect();
    let max_p = pts.iter().copied().fold(0.0_f64, f64::max).max(1.0);
    let mut out = vec![0.0_f32; bins];
    let n = pts.len();
    for (b, slot) in out.iter_mut().enumerate() {
        let i0 = (b * n) / bins;
        let i1 = (((b + 1) * n) / bins).max(i0 + 1).min(n);
        let sum: f64 = pts[i0..i1].iter().sum();
        let avg = sum / ((i1 - i0) as f64);
        *slot = (avg / max_p).clamp(0.0, 1.0) as f32;
    }
    let mut smooth = vec![0.0_f32; bins];
    for i in 0..bins {
        let prev = if i > 0 { out[i - 1] } else { out[i] };
        let next = if i + 1 < bins { out[i + 1] } else { out[i] };
        smooth[i] = 0.25 * prev + 0.50 * out[i] + 0.25 * next;
    }
    smooth
}

pub fn analyse_xdc1_with_curve(
    raw_notes: &[RawChartNote],
    rate: f32,
) -> (f64, f64, f64, Vec<f32>, i64) {
    let r = (rate as f64).max(0.1);
    let mut notes: Vec<XNote> = raw_notes
        .iter()
        .filter(|n| n.col < 4)
        .map(|n| {
            let h = if r != 1.0 {
                (n.head_ms as f64 / r).floor() as i32
            } else {
                n.head_ms as i32
            };
            let mut t = if n.tail_ms >= 0 {
                if r != 1.0 {
                    (n.tail_ms as f64 / r).floor() as i32
                } else {
                    n.tail_ms as i32
                }
            } else {
                -1
            };
            if t >= 0 && t < h {
                t = -1;
            }
            XNote {
                k: n.col,
                h: h.max(0),
                t,
            }
        })
        .collect();
    notes.sort_by(|a, b| a.h.cmp(&b.h).then(a.k.cmp(&b.k)));

    let mut seq: Vec<XNote> = Vec::with_capacity(notes.len());
    let mut last = [-1i32; 4];
    for n in notes {
        let j = last[n.k];
        if j >= 0 && n.h - seq[j as usize].h <= DUP_MS {
            let idx = j as usize;
            seq[idx].t = seq[idx].t.max(n.t);
            continue;
        }
        last[n.k] = seq.len() as i32;
        seq.push(n);
    }
    if seq.len() < 2 {
        return (0.0, 0.0, 0.0, Vec::new(), 0);
    }

    let mut by_col: [Vec<XNote>; 4] = Default::default();
    let mut by_hand: [Vec<XNote>; 2] = Default::default();
    let mut end = 0i32;
    let mut any_ln = false;
    for &n in &seq {
        by_col[n.k].push(n);
        by_hand[n.k / 2].push(n);
        end = end.max(n.h).max(n.t);
        any_ln = any_ln || n.t >= 0;
    }
    let mut grid = Vec::new();
    let mut t = 0.0;
    let max_t = (end as f64) + 1000.0;
    while t <= max_t {
        grid.push(t);
        t += GRID_MS;
    }
    let x = hit_window_unit();
    let (mut strain, able, mut strain_rice) = technique_strain(x, &by_col, &grid);

    for (h, hand_col) in by_hand.iter().enumerate() {
        let mut cols: [Vec<XNote>; 4] = Default::default();
        cols[(2 * h)..(2 * h + 2)].clone_from_slice(&by_col[(2 * h)..(2 * h + 2)]);
        let hc = hold_costs(x, &cols, hand_col, &grid);
        for (i, dst) in strain[h].iter_mut().enumerate() {
            let l = hc.rel[i] + hc.weight[i] + hc.cross[i];
            if l > 0.0 {
                *dst += LN_W * l.powf(LN_X);
            }
        }
    }

    let w = [
        note_weights(&grid, &by_hand[0]),
        note_weights(&grid, &by_hand[1]),
    ];
    let rows = build_rows(&seq);
    let sync = hand_sync(&able, &rows, &grid);
    let rested_all = combine_hands(&strain, &sync);
    let curve = downsample_strain_curve(&rested_all, 96);
    let duration_ms = ((end as f64) * r).round() as i64;

    apply_stamina(&grid, &mut strain, &w);
    if any_ln {
        apply_stamina(&grid, &mut strain_rice, &w);
    }

    let all = combine_hands(&strain, &sync);
    let all_rice = if any_ln {
        combine_hands(&strain_rice, &sync)
    } else {
        all.clone()
    };
    let weights = note_weights(&grid, &seq);
    let tn = seq.len() as f64;
    let pd = bin_strain(&all);
    let pd_rice = if any_ln {
        bin_strain(&all_rice)
    } else {
        bin_strain(&all)
    };
    let raw = rating_from_bins(&pd, &weights, tn);
    let raw_rice = if any_ln {
        rating_from_bins(&pd_rice, &weights, tn)
    } else {
        raw
    };
    let pts = rating_points(raw);
    let pts_rice = rating_points(raw_rice);
    let dan = raw_to_dan(points_to_raw(pts));
    (dan, pts, pts_rice, curve, duration_ms)
}

pub fn xdc1_diff_to_numeric(diff: f64) -> f64 {
    if diff <= 0.01 {
        return 0.0;
    }
    let n = XDC1_DAN_MEANS.len();
    for (i, &mean) in XDC1_DAN_MEANS.iter().enumerate() {
        let lower = if i > 0 {
            0.5 * (XDC1_DAN_MEANS[i - 1] + mean)
        } else {
            mean - (0.5 * (XDC1_DAN_MEANS[1] + mean) - mean)
        };
        let upper = if i + 1 < n {
            0.5 * (mean + XDC1_DAN_MEANS[i + 1])
        } else {
            mean + (mean - 0.5 * (mean + XDC1_DAN_MEANS[n - 2]))
        };
        if diff >= lower && diff < upper {
            let t = if diff < mean {
                0.5 * (diff - lower) / (mean - lower).max(1e-6)
            } else {
                0.5 + 0.5 * (diff - mean) / (upper - mean).max(1e-6)
            };
            return 4.0 + (i as f64) + t.clamp(0.0, 0.99);
        }
    }
    let first_lower =
        XDC1_DAN_MEANS[0] - (0.5 * (XDC1_DAN_MEANS[1] + XDC1_DAN_MEANS[0]) - XDC1_DAN_MEANS[0]);
    if diff < first_lower {
        return (((diff - 0.5) / (first_lower - 0.5).max(0.5)) * 3.0 + 1.0).clamp(1.0, 3.99);
    }
    20.5
}
