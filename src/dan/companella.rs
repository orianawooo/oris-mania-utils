use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use super::RawChartNote;

#[derive(Clone, Copy)]
struct SNote {
    k: usize,
    h: i32,
    t: i32,
}

#[derive(Clone, Copy)]
struct OnnxTreeNode {
    is_leaf: bool,
    feature_id: usize,
    threshold: f32,
    true_id: usize,
    false_id: usize,
    weight: f32,
}

struct OnnxEnsemble {
    base_value: f32,
    trees: Vec<Vec<OnnxTreeNode>>,
}

static ONNX_MODEL: OnceLock<Option<OnnxEnsemble>> = OnceLock::new();

pub fn calculate_sunny(raw_notes: &[RawChartNote], key_count: usize, rate: f32) -> f64 {
    calculate_sunny_with_od(raw_notes, key_count, rate, 8.0)
}

fn np_search_left(arr: &[f64], val: f64) -> usize {
    arr.partition_point(|&v| v < val)
}

fn np_search_right(arr: &[f64], val: f64) -> usize {
    arr.partition_point(|&v| v <= val)
}

fn bisect_left_i32(arr: &[i32], val: f64) -> usize {
    arr.partition_point(|&v| (v as f64) < val)
}

fn cumulative_sum(x: &[f64], f: &[f64]) -> Vec<f64> {
    let mut big_f = vec![0.0_f64; x.len()];
    for i in 1..x.len() {
        big_f[i] = big_f[i - 1] + f[i - 1] * (x[i] - x[i - 1]);
    }
    big_f
}

fn query_cumsum(q: f64, x: &[f64], big_f: &[f64], f: &[f64]) -> f64 {
    if q <= x[0] {
        return 0.0;
    }
    if q >= x[x.len() - 1] {
        return big_f[big_f.len() - 1];
    }
    let i = np_search_left(x, q).saturating_sub(1);
    big_f[i] + f[i] * (q - x[i])
}

fn smooth_on_corners(x: &[f64], f: &[f64], window: f64, scale: f64, avg_mode: bool) -> Vec<f64> {
    if x.is_empty() {
        return Vec::new();
    }
    let big_f = cumulative_sum(x, f);
    let x_first = x[0];
    let x_last = x[x.len() - 1];
    let mut g = vec![0.0_f64; f.len()];
    for i in 0..x.len() {
        let s = x[i];
        let a = (s - window).max(x_first);
        let b = (s + window).min(x_last);
        let val = query_cumsum(b, x, &big_f, f) - query_cumsum(a, x, &big_f, f);
        g[i] = if avg_mode {
            if b > a {
                val / (b - a)
            } else {
                0.0
            }
        } else {
            scale * val
        };
    }
    g
}

fn interp_values(new_x: &[f64], old_x: &[f64], old_vals: &[f64]) -> Vec<f64> {
    let mut new_vals = vec![0.0_f64; new_x.len()];
    if old_x.is_empty() || old_vals.is_empty() {
        return new_vals;
    }
    let last = old_x.len() - 1;
    for (i, &x_val) in new_x.iter().enumerate() {
        if x_val <= old_x[0] {
            new_vals[i] = old_vals[0];
        } else if x_val >= old_x[last] {
            new_vals[i] = old_vals[last];
        } else {
            let pos = np_search_left(old_x, x_val);
            if pos == 0 {
                new_vals[i] = old_vals[0];
            } else if pos >= old_x.len() {
                new_vals[i] = old_vals[last];
            } else {
                let x0 = old_x[pos - 1];
                let x1 = old_x[pos];
                let y0 = old_vals[pos - 1];
                let y1 = old_vals[pos];
                let denom = x1 - x0;
                new_vals[i] = if denom.abs() > 1e-12 {
                    y0 + ((x_val - x0) / denom) * (y1 - y0)
                } else {
                    y0
                };
            }
        }
    }
    new_vals
}

fn step_interp(new_x: &[f64], old_x: &[f64], old_vals: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0_f64; new_x.len()];
    if old_vals.is_empty() {
        return out;
    }
    let max_idx = old_vals.len() - 1;
    for (i, &nx) in new_x.iter().enumerate() {
        let idx = np_search_right(old_x, nx).saturating_sub(1).min(max_idx);
        out[i] = old_vals[idx];
    }
    out
}

fn sunny_ln_bodies_sparse(ln_seq: &[SNote], t_max: i32) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut diff: BTreeMap<i64, f64> = BTreeMap::new();
    for n in ln_seq {
        let h = n.h as i64;
        let t = n.t as i64;
        let t0 = (h + 60).min(t);
        let t1 = (h + 120).min(t);
        *diff.entry(t0).or_insert(0.0) += 1.3;
        *diff.entry(t1).or_insert(0.0) += -0.3;
        *diff.entry(t).or_insert(0.0) -= 1.0;
    }
    let mut pts_set: BTreeSet<i64> = BTreeSet::new();
    pts_set.insert(0);
    pts_set.insert(t_max as i64);
    for &k in diff.keys() {
        pts_set.insert(k);
    }
    let points_i: Vec<i64> = pts_set.into_iter().collect();
    let points: Vec<f64> = points_i.iter().map(|&v| v as f64).collect();
    let mut values = Vec::with_capacity(points.len().saturating_sub(1));
    let mut cumsum = Vec::with_capacity(points.len());
    cumsum.push(0.0);
    let mut curr = 0.0_f64;
    for i in 0..points_i.len().saturating_sub(1) {
        if let Some(&chg) = diff.get(&points_i[i]) {
            curr += chg;
        }
        let v = curr.min(2.5 + 0.5 * curr);
        values.push(v);
        let seg_len = (points_i[i + 1] - points_i[i]) as f64;
        let prev = *cumsum.last().unwrap();
        cumsum.push(prev + seg_len * v);
    }
    (points, cumsum, values)
}

fn sunny_ln_sum(a: f64, b: f64, points: &[f64], cumsum: &[f64], values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let i = np_search_right(points, a)
        .saturating_sub(1)
        .min(values.len() - 1);
    let j = np_search_right(points, b)
        .saturating_sub(1)
        .min(values.len() - 1);
    if i == j {
        return (b - a) * values[i];
    }
    let mut total = (points[i + 1] - a) * values[i];
    total += cumsum[j] - cumsum[i + 1];
    total += (b - points[j]) * values[j];
    total
}

fn sunny_calculate_notes(notes: &[SNote], key_count: usize, od: f64) -> f64 {
    if notes.is_empty() || key_count == 0 {
        return 0.0;
    }
    let k_cnt = key_count;
    let mut x = 0.3 * ((64.5 - (od * 3.0).ceil()).max(0.1) / 500.0).sqrt();
    x = x.min(0.6 * (x - 0.09) + 0.09);

    let mut note_seq = notes.to_vec();
    note_seq.sort_by(|a, b| a.h.cmp(&b.h).then(a.k.cmp(&b.k)));

    let t_max = note_seq.iter().map(|n| n.h.max(n.t)).max().unwrap_or(0) + 1;
    let mut note_by_col: Vec<Vec<SNote>> = vec![Vec::new(); k_cnt];
    let mut ln_seq: Vec<SNote> = Vec::new();
    for &n in &note_seq {
        note_by_col[n.k].push(n);
        if n.t >= 0 {
            ln_seq.push(n);
        }
    }
    let mut tail_seq = ln_seq.clone();
    tail_seq.sort_by(|a, b| a.t.cmp(&b.t).then(a.h.cmp(&b.h)));

    let mut corners_base: BTreeSet<i64> = BTreeSet::new();
    for n in &note_seq {
        let h = n.h as i64;
        for s in [Some(h), (n.t >= 0).then_some(n.t as i64)]
            .into_iter()
            .flatten()
        {
            corners_base.insert(s);
            corners_base.insert(s + 501);
            corners_base.insert(s - 499);
            corners_base.insert(s + 1);
        }
    }
    corners_base.insert(0);
    corners_base.insert(t_max as i64);
    let base_corners: Vec<f64> = corners_base
        .iter()
        .copied()
        .filter(|&s| s >= 0 && s <= t_max as i64)
        .map(|s| s as f64)
        .collect();

    let mut corners_a: BTreeSet<i64> = BTreeSet::new();
    for n in &note_seq {
        let h = n.h as i64;
        for s in [Some(h), (n.t >= 0).then_some(n.t as i64)]
            .into_iter()
            .flatten()
        {
            corners_a.insert(s);
            corners_a.insert(s + 1000);
            corners_a.insert(s - 1000);
        }
    }
    corners_a.insert(0);
    corners_a.insert(t_max as i64);
    let a_corners: Vec<f64> = corners_a
        .iter()
        .copied()
        .filter(|&s| s >= 0 && s <= t_max as i64)
        .map(|s| s as f64)
        .collect();

    let mut all_set: BTreeSet<i64> = BTreeSet::new();
    for &s in &corners_base {
        if s >= 0 && s <= t_max as i64 {
            all_set.insert(s);
        }
    }
    for &s in &corners_a {
        if s >= 0 && s <= t_max as i64 {
            all_set.insert(s);
        }
    }
    let all_corners: Vec<f64> = all_set.into_iter().map(|s| s as f64).collect();
    if all_corners.len() < 2 || base_corners.len() < 2 || a_corners.len() < 2 {
        return 0.0;
    }

    let bc_len = base_corners.len();
    let mut key_usage = vec![vec![false; bc_len]; k_cnt];
    for n in &note_seq {
        let start_t = (n.h - 150).max(0) as f64;
        let end_t = if n.t < 0 {
            (n.h + 150) as f64
        } else {
            (n.t + 150).min(t_max - 1) as f64
        };
        let l = np_search_left(&base_corners, start_t);
        let r = np_search_left(&base_corners, end_t);
        key_usage[n.k][l..r].fill(true);
    }

    let active_columns: Vec<Vec<usize>> = (0..bc_len)
        .map(|i| (0..k_cnt).filter(|&col| key_usage[col][i]).collect())
        .collect();

    let mut key_usage_400 = vec![vec![0.0_f64; bc_len]; k_cnt];
    for n in &note_seq {
        let start_t = n.h.max(0) as f64;
        let end_t = if n.t < 0 {
            n.h as f64
        } else {
            n.t.min(t_max - 1) as f64
        };
        let l400 = np_search_left(&base_corners, start_t - 400.0);
        let l = np_search_left(&base_corners, start_t);
        let r = np_search_left(&base_corners, end_t);
        let r400 = np_search_left(&base_corners, end_t + 400.0);
        let body_val = 3.75 + (end_t - start_t).min(1500.0) / 150.0;
        for slot in &mut key_usage_400[n.k][l..r] {
            *slot += body_val;
        }
        for idx in l400..l {
            let d = base_corners[idx] - start_t;
            key_usage_400[n.k][idx] += 3.75 - (3.75 / 160000.0) * (d * d);
        }
        for idx in r..r400 {
            let d = (base_corners[idx] - end_t).abs();
            key_usage_400[n.k][idx] += 3.75 - (3.75 / 160000.0) * (d * d);
        }
    }

    let mut anchor = vec![0.0_f64; bc_len];
    let mut counts = vec![0.0_f64; k_cnt];
    for idx in 0..bc_len {
        for col in 0..k_cnt {
            counts[col] = key_usage_400[col][idx];
        }
        counts.sort_by(|a, b| b.total_cmp(a));
        let nonzero: Vec<f64> = counts.iter().copied().filter(|&c| c != 0.0).collect();
        if nonzero.len() > 1 {
            let mut walk = 0.0;
            let mut max_walk = 0.0;
            for i in 0..nonzero.len() - 1 {
                let ratio = nonzero[i + 1] / nonzero[i];
                walk += nonzero[i] * (1.0 - 4.0 * (0.5 - ratio).powi(2));
                max_walk += nonzero[i];
            }
            anchor[idx] = walk / max_walk;
        }
    }
    for a in &mut anchor {
        *a = 1.0 + (*a - 0.18).min(5.0 * (*a - 0.22).powi(3));
    }

    let mut j_ks = vec![vec![0.0_f64; bc_len]; k_cnt];
    let mut delta_ks = vec![vec![1e9_f64; bc_len]; k_cnt];
    for col in 0..k_cnt {
        let col_notes = &note_by_col[col];
        for i in 0..col_notes.len().saturating_sub(1) {
            let start = col_notes[i].h as f64;
            let end = col_notes[i + 1].h as f64;
            let l = np_search_left(&base_corners, start);
            let r = np_search_left(&base_corners, end);
            if l >= r {
                continue;
            }
            let delta = 0.001 * (end - start);
            let val = delta.powi(-1) * (delta + 0.11 * x.powf(0.25)).powi(-1);
            let jack_nerf = 1.0 - 7e-5 * (0.15 + (delta - 0.08).abs()).powi(-4);
            let j_val = val * jack_nerf;
            j_ks[col][l..r].fill(j_val);
            delta_ks[col][l..r].fill(delta);
        }
    }
    let jbar_ks: Vec<Vec<f64>> = (0..k_cnt)
        .map(|col| smooth_on_corners(&base_corners, &j_ks[col], 500.0, 0.001, false))
        .collect();
    let mut jbar = vec![0.0_f64; bc_len];
    for i in 0..bc_len {
        let mut num = 0.0;
        let mut den = 0.0;
        for col in 0..k_cnt {
            let v = jbar_ks[col][i].max(0.0);
            let w = 1.0 / delta_ks[col][i];
            num += v.powi(5) * w;
            den += w;
        }
        jbar[i] = (num / den.max(1e-9)).powf(0.2);
    }
    let jbar = interp_values(&all_corners, &base_corners, &jbar);

    let cross_matrix: [&[f64]; 11] = [
        &[-1.0],
        &[0.075, 0.075],
        &[0.125, 0.05, 0.125],
        &[0.125, 0.125, 0.125, 0.125],
        &[0.175, 0.25, 0.05, 0.25, 0.175],
        &[0.175, 0.25, 0.175, 0.175, 0.25, 0.175],
        &[0.225, 0.35, 0.25, 0.05, 0.25, 0.35, 0.225],
        &[0.225, 0.35, 0.25, 0.225, 0.225, 0.25, 0.35, 0.225],
        &[0.275, 0.45, 0.35, 0.25, 0.05, 0.25, 0.35, 0.45, 0.275],
        &[
            0.275, 0.45, 0.35, 0.25, 0.275, 0.275, 0.25, 0.35, 0.45, 0.275,
        ],
        &[
            0.325, 0.55, 0.45, 0.35, 0.25, 0.05, 0.25, 0.35, 0.45, 0.55, 0.325,
        ],
    ];
    let cross_coeff = cross_matrix[k_cnt.min(10)];
    let mut x_ks = vec![vec![0.0_f64; bc_len]; k_cnt + 1];
    let mut fast_cross = vec![vec![0.0_f64; bc_len]; k_cnt + 1];
    for col in 0..=k_cnt {
        let notes_in_pair: Vec<SNote> = if col == 0 {
            note_by_col[0].clone()
        } else if col == k_cnt {
            note_by_col[k_cnt - 1].clone()
        } else {
            let mut merged =
                Vec::with_capacity(note_by_col[col - 1].len() + note_by_col[col].len());
            merged.extend_from_slice(&note_by_col[col - 1]);
            merged.extend_from_slice(&note_by_col[col]);
            merged.sort_by_key(|n| n.h);
            merged
        };
        for i in 1..notes_in_pair.len() {
            let start = notes_in_pair[i - 1].h as f64;
            let end = notes_in_pair[i].h as f64;
            let idx_start = np_search_left(&base_corners, start);
            let idx_end = np_search_left(&base_corners, end);
            if idx_start >= idx_end {
                continue;
            }
            let delta = 0.001 * (end - start);
            let mut val = 0.16 * x.max(delta).powi(-2);
            let end_clamped = idx_end.min(active_columns.len() - 1);
            let prev_not_start = col == 0 || !active_columns[idx_start].contains(&(col - 1));
            let prev_not_end = col == 0 || !active_columns[end_clamped].contains(&(col - 1));
            let curr_not_start = !active_columns[idx_start].contains(&col);
            let curr_not_end = !active_columns[end_clamped].contains(&col);
            if (prev_not_start && prev_not_end) || (curr_not_start && curr_not_end) {
                val *= 1.0 - cross_coeff[col];
            }
            let fc = (0.4 * delta.max(0.06).max(0.75 * x).powi(-2) - 80.0).max(0.0);
            x_ks[col][idx_start..idx_end].fill(val);
            fast_cross[col][idx_start..idx_end].fill(fc);
        }
    }
    let mut x_base = vec![0.0_f64; bc_len];
    for i in 0..bc_len {
        let mut s1 = 0.0;
        for col in 0..=k_cnt {
            s1 += x_ks[col][i] * cross_coeff[col];
        }
        let mut s2 = 0.0;
        for col in 0..k_cnt {
            s2 += (fast_cross[col][i]
                * cross_coeff[col]
                * fast_cross[col + 1][i]
                * cross_coeff[col + 1])
                .sqrt();
        }
        x_base[i] = s1 + s2;
    }
    let xbar = interp_values(
        &all_corners,
        &base_corners,
        &smooth_on_corners(&base_corners, &x_base, 500.0, 0.001, false),
    );

    let (ln_pts, ln_cumsum, ln_vals) = sunny_ln_bodies_sparse(&ln_seq, t_max);
    let mut p_step = vec![0.0_f64; bc_len];
    for i in 0..note_seq.len().saturating_sub(1) {
        let h_l = note_seq[i].h as f64;
        let h_r = note_seq[i + 1].h as f64;
        let delta_time = h_r - h_l;
        if delta_time < 1e-9 {
            let spike = 1000.0 * (0.02 * (4.0 / x - 24.0)).max(0.0).powf(0.25);
            let l = np_search_left(&base_corners, h_l);
            let r = np_search_right(&base_corners, h_l).min(bc_len);
            if l < r {
                for slot in &mut p_step[l..r] {
                    *slot += spike;
                }
            }
            continue;
        }
        let start = np_search_left(&base_corners, h_l);
        let end = np_search_left(&base_corners, h_r);
        if start >= end {
            continue;
        }
        let delta = 0.001 * delta_time;
        let v = 1.0 + 6.0 * 0.001 * sunny_ln_sum(h_l, h_r, &ln_pts, &ln_cumsum, &ln_vals);
        let nps = 7.5 / delta;
        let b_val = if nps > 160.0 && nps < 360.0 {
            1.0 + 1.7e-7 * (nps - 160.0) * (nps - 360.0).powi(2)
        } else {
            1.0
        };
        let inner = if delta < 2.0 * x / 3.0 {
            1.0 - (24.0 / x) * (delta - x / 2.0).powi(2)
        } else {
            1.0 - (24.0 / x) * (x / 6.0).powi(2)
        };
        let inc = delta.powi(-1) * ((0.08 / x) * inner).max(0.0).powf(0.25) * b_val.max(v);
        for idx in start..end {
            p_step[idx] += (inc * anchor[idx]).min(inc.max(inc * 2.0 - 10.0));
        }
    }
    let pbar = interp_values(
        &all_corners,
        &base_corners,
        &smooth_on_corners(&base_corners, &p_step, 500.0, 0.001, false),
    );

    let mut dks = vec![vec![0.0_f64; bc_len]; k_cnt.saturating_sub(1)];
    for i in 0..bc_len {
        let cols = &active_columns[i];
        for j in 0..cols.len().saturating_sub(1) {
            let k0 = cols[j];
            let k1 = cols[j + 1];
            if k0 < k_cnt.saturating_sub(1) {
                dks[k0][i] = (delta_ks[k0][i] - delta_ks[k1][i]).abs()
                    + 0.4 * ((delta_ks[k0][i].max(delta_ks[k1][i])) - 0.11).max(0.0);
            }
        }
    }
    let mut a_step = vec![1.0_f64; a_corners.len()];
    for (i, &s) in a_corners.iter().enumerate() {
        let idx = np_search_left(&base_corners, s).min(bc_len - 1);
        let cols = &active_columns[idx];
        for j in 0..cols.len().saturating_sub(1) {
            let k0 = cols[j];
            let k1 = cols[j + 1];
            if k0 >= dks.len() {
                continue;
            }
            let d_val = dks[k0][idx];
            if d_val < 0.02 {
                a_step[i] *= (0.75 + 0.5 * delta_ks[k0][idx].max(delta_ks[k1][idx])).min(1.0);
            } else if d_val < 0.07 {
                a_step[i] *=
                    (0.65 + 5.0 * d_val + 0.5 * delta_ks[k0][idx].max(delta_ks[k1][idx])).min(1.0);
            }
        }
    }
    let abar = interp_values(
        &all_corners,
        &a_corners,
        &smooth_on_corners(&a_corners, &a_step, 250.0, 1.0, true),
    );

    let mut r_step = vec![0.0_f64; bc_len];
    if !tail_seq.is_empty() {
        let times_by_col: Vec<Vec<i32>> = note_by_col
            .iter()
            .map(|c| c.iter().map(|n| n.h).collect())
            .collect();
        let mut i_list = vec![0.0_f64; tail_seq.len()];
        for (i, n) in tail_seq.iter().enumerate() {
            let col_notes = &note_by_col[n.k];
            let pos = times_by_col[n.k].partition_point(|&h| h < n.h);
            let h_j = if pos + 1 < col_notes.len() {
                col_notes[pos + 1].h as f64
            } else {
                1_000_000_000.0
            };
            let i_h = 0.001 * ((n.t - n.h - 80) as f64).abs() / x;
            let i_t = 0.001 * (h_j - (n.t as f64) - 80.0).abs() / x;
            i_list[i] = 2.0 / (2.0 + (-5.0 * (i_h - 0.75)).exp() + (-5.0 * (i_t - 0.75)).exp());
        }
        for i in 0..tail_seq.len() - 1 {
            let t_start = tail_seq[i].t as f64;
            let t_end = tail_seq[i + 1].t as f64;
            let l = np_search_left(&base_corners, t_start);
            let r = np_search_left(&base_corners, t_end);
            if l >= r {
                continue;
            }
            let delta_r = 0.001 * (t_end - t_start);
            let r_val =
                0.08 * delta_r.powf(-0.5) * x.powi(-1) * (1.0 + 0.8 * (i_list[i] + i_list[i + 1]));
            r_step[l..r].fill(r_val);
        }
    }
    let rbar = interp_values(
        &all_corners,
        &base_corners,
        &smooth_on_corners(&base_corners, &r_step, 500.0, 0.001, false),
    );

    let hit_times: Vec<i32> = note_seq.iter().map(|n| n.h).collect();
    let mut c_step = vec![0.0_f64; bc_len];
    let mut ks_step = vec![1.0_f64; bc_len];
    for i in 0..bc_len {
        let s = base_corners[i];
        let hi_i = bisect_left_i32(&hit_times, s + 500.0);
        let lo_i = bisect_left_i32(&hit_times, s - 500.0);
        c_step[i] = (hi_i.saturating_sub(lo_i)) as f64;
        let kc = (0..k_cnt).filter(|&col| key_usage[col][i]).count();
        ks_step[i] = (kc as f64).max(1.0);
    }
    let c_arr = step_interp(&all_corners, &base_corners, &c_step);
    let ks_arr = step_interp(&all_corners, &base_corners, &ks_step);

    let ac_len = all_corners.len();
    let mut d_all = vec![0.0_f64; ac_len];
    for i in 0..ac_len {
        let term1 = abar[i].powf(3.0 / ks_arr[i]) * jbar[i].min(8.0 + 0.85 * jbar[i]);
        let term2 = abar[i].powf(2.0 / 3.0) * (0.8 * pbar[i] + rbar[i] * 35.0 / (c_arr[i] + 8.0));
        let s_v = (0.4 * term1.powf(1.5) + 0.6 * term2.powf(1.5)).powf(2.0 / 3.0);
        let t_v = abar[i].powf(3.0 / ks_arr[i]) * xbar[i] / (xbar[i] + s_v + 1.0);
        d_all[i] = 2.7 * s_v.powf(0.5) * t_v.powf(1.5) + s_v * 0.27;
    }

    let mut gaps = vec![0.0_f64; ac_len];
    gaps[0] = (all_corners[1] - all_corners[0]) / 2.0;
    gaps[ac_len - 1] = (all_corners[ac_len - 1] - all_corners[ac_len - 2]) / 2.0;
    for i in 1..ac_len - 1 {
        gaps[i] = (all_corners[i + 1] - all_corners[i - 1]) / 2.0;
    }
    let eff_w: Vec<f64> = (0..ac_len).map(|i| c_arr[i] * gaps[i]).collect();

    let mut order: Vec<usize> = (0..ac_len).collect();
    order.sort_by(|&a, &b| d_all[a].total_cmp(&d_all[b]));
    let d_sorted: Vec<f64> = order.iter().map(|&i| d_all[i]).collect();
    let w_sorted: Vec<f64> = order.iter().map(|&i| eff_w[i]).collect();

    let mut cum_w = vec![0.0_f64; ac_len];
    cum_w[0] = w_sorted[0];
    for i in 1..ac_len {
        cum_w[i] = cum_w[i - 1] + w_sorted[i];
    }
    let total_w = cum_w[ac_len - 1];
    if total_w <= 1e-12 {
        return 0.0;
    }
    let norm_cum: Vec<f64> = cum_w.iter().map(|&w| w / total_w).collect();
    let pcts = [0.945, 0.935, 0.925, 0.915, 0.845, 0.835, 0.825, 0.815];
    let mut idxs = [0usize; 8];
    for (p_i, &p) in pcts.iter().enumerate() {
        idxs[p_i] = np_search_left(&norm_cum, p).min(ac_len - 1);
    }
    let p93 = (d_sorted[idxs[0]] + d_sorted[idxs[1]] + d_sorted[idxs[2]] + d_sorted[idxs[3]]) / 4.0;
    let p83 = (d_sorted[idxs[4]] + d_sorted[idxs[5]] + d_sorted[idxs[6]] + d_sorted[idxs[7]]) / 4.0;

    let mut sum_d5w = 0.0;
    for i in 0..ac_len {
        sum_d5w += d_sorted[i].powi(5) * w_sorted[i];
    }
    let weighted_mean = (sum_d5w / total_w).powf(0.2);

    let mut sr = 0.88 * p93 * 0.25 + 0.94 * p83 * 0.2 + weighted_mean * 0.55;
    let mut total_notes = note_seq.len() as f64;
    for n in &ln_seq {
        total_notes += 0.5 * ((n.t - n.h) as f64).min(1000.0) / 200.0;
    }
    sr *= total_notes / (total_notes + 60.0);
    if sr > 9.0 {
        sr = 9.0 + (sr - 9.0) / 1.2;
    }
    sr *= 0.975;
    if sr.is_finite() {
        sr.max(0.0)
    } else {
        0.0
    }
}

pub fn calculate_sunny_with_od(
    raw_notes: &[RawChartNote],
    key_count: usize,
    rate: f32,
    od: f64,
) -> f64 {
    if raw_notes.is_empty() || key_count == 0 {
        return 0.0;
    }
    let r = (rate as f64).max(0.1);
    let apply_t = |t: i64| -> i32 {
        if (rate - 1.5).abs() < 0.01 {
            ((t as f64) * 2.0 / 3.0).floor() as i32
        } else if (rate - 0.75).abs() < 0.01 {
            ((t as f64) * 4.0 / 3.0).floor() as i32
        } else if (rate - 1.0).abs() > 0.005 {
            ((t as f64) / r).floor() as i32
        } else {
            t as i32
        }
    };
    let notes: Vec<SNote> = raw_notes
        .iter()
        .map(|n| {
            let h = apply_t(n.head_ms).max(0);
            let t = if n.tail_ms >= 0 {
                let tt = apply_t(n.tail_ms).max(0);
                if tt >= h {
                    tt
                } else {
                    h
                }
            } else {
                -1
            };
            SNote {
                k: n.col.min(key_count - 1),
                h,
                t,
            }
        })
        .collect();
    sunny_calculate_notes(&notes, key_count, od)
}

pub fn calculate_pure_ln_dan(
    raw_notes: &[RawChartNote],
    key_count: usize,
    rate: f32,
    od: f64,
) -> Option<f64> {
    if raw_notes.is_empty() || key_count == 0 {
        return None;
    }
    let ln_count = raw_notes.iter().filter(|n| n.tail_ms >= 0).count();
    let ln_pct = (ln_count as f64 / raw_notes.len() as f64) * 100.0;
    if ln_pct < 10.0 {
        return None;
    }
    let full_sunny = calculate_sunny_with_od(raw_notes, key_count, rate, od);
    let no_ln_notes: Vec<RawChartNote> = raw_notes
        .iter()
        .map(|n| RawChartNote {
            col: n.col,
            head_ms: n.head_ms,
            tail_ms: -1,
        })
        .collect();
    let no_ln_sunny = calculate_sunny_with_od(&no_ln_notes, key_count, rate, od);
    if full_sunny <= 0.0 || no_ln_sunny <= 0.0 {
        return None;
    }
    let pure_ln_sunny = full_sunny - no_ln_sunny;
    let base =
        (pure_ln_sunny + 1.0).max(0.0).powf(1.22) * (full_sunny - pure_ln_sunny).max(0.0).powi(3);
    let ln_diff = base.powf(1.0 / 3.0);

    const ANCHORS: [(f64, f64); 20] = [
        (1.0, 3.128),
        (2.0, 4.005),
        (3.0, 4.86),
        (4.0, 4.801),
        (5.0, 4.964),
        (6.0, 5.471),
        (7.0, 6.193),
        (8.0, 6.302),
        (9.0, 6.739),
        (10.0, 7.282),
        (11.0, 7.783),
        (12.0, 8.092),
        (13.0, 8.605),
        (14.0, 9.243),
        (15.0, 9.769),
        (16.0, 9.878),
        (17.0, 10.731),
        (18.0, 10.987),
        (19.0, 11.136),
        (20.0, 11.851),
    ];
    let mut best: Option<f64> = None;
    for w in ANCHORS.windows(2) {
        let (ia, da) = w[0];
        let (ib, db) = w[1];
        let lo = da.min(db) - 1e-9;
        let hi = da.max(db) + 1e-9;
        if ln_diff >= lo && ln_diff <= hi {
            let span = db - da;
            let t = if span.abs() < 1e-9 {
                0.0
            } else {
                (ln_diff - da) / span
            };
            let cand = ia + t * (ib - ia);
            best = Some(best.map_or(cand, |b| b.max(cand)));
        }
    }
    let raw_dan = best.unwrap_or_else(|| {
        if ln_diff < ANCHORS[0].1 {
            let span = ANCHORS[1].1 - ANCHORS[0].1;
            let t = if span.abs() < 1e-9 {
                0.0
            } else {
                (ln_diff - ANCHORS[0].1) / span
            };
            ANCHORS[0].0 + t
        } else {
            let last = ANCHORS[19];
            let prev = ANCHORS[18];
            let span = last.1 - prev.1;
            let t = if span.abs() < 1e-9 {
                0.0
            } else {
                (ln_diff - last.1) / span
            };
            last.0 + t
        }
    });
    Some(raw_dan.clamp(1.0, 20.99))
}

pub fn calculate_interlude(raw_notes: &[RawChartNote], key_count: usize, rate: f32) -> f64 {
    const CURVE_POWER: f32 = 0.6;
    const CURVE_SCALE: f32 = 0.4056;
    const MOST_IMPORTANT_NOTES: f32 = 2500.0;
    const JACK_CURVE_CUTOFF: f32 = 230.0;
    const STREAM_CURVE_CUTOFF: f32 = 10.0;
    const STREAM_CURVE_CUTOFF2: f32 = 10.0;
    const OHT_NERF: f32 = 3.0;
    const STREAM_SCALE: f32 = 6.0;
    const STREAM_POW: f32 = 0.5;
    const STRAIN_SCALE: f32 = 0.01626;
    const STRAIN_TIME_CAP: f32 = 200.0;

    if raw_notes.is_empty() || key_count == 0 {
        return 0.0;
    }

    let r = rate.max(0.1);
    let mut rows: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for n in raw_notes {
        rows.entry(n.head_ms)
            .or_default()
            .push(n.col.min(key_count - 1));
    }

    let ms_to_stream_bpm = |delta_ms: f32| -> f32 {
        let x = 0.02 * delta_ms;
        let v = 300.0 / x - 300.0 / x.powf(STREAM_CURVE_CUTOFF) / STREAM_CURVE_CUTOFF2;
        if v.is_finite() {
            v.max(0.0)
        } else {
            0.0
        }
    };

    let jack_comp = |jack_delta: f32, stream_delta: f32| -> f32 {
        if stream_delta <= 0.0 {
            return 1.0;
        }
        let ratio = jack_delta / stream_delta;
        if ratio <= 0.0 {
            return 0.0;
        }
        ratio.log2().max(0.0).sqrt().min(1.0)
    };

    let strain_func = |half_life_ms: f32, curr: f32, input: f32, delta_ms: f32| -> f32 {
        let decay_rate = 0.5_f32.ln() / half_life_ms;
        let decay = (decay_rate * STRAIN_TIME_CAP.min(delta_ms)).exp();
        let time_cap_decay = if delta_ms > STRAIN_TIME_CAP {
            (decay_rate * (delta_ms - STRAIN_TIME_CAP)).exp()
        } else {
            1.0
        };
        let a = curr * time_cap_decay;
        let b = input * input * STRAIN_SCALE;
        b - (b - a) * decay
    };

    let mut last_in_col = vec![0.0_f32; key_count];
    let mut strain_values = vec![0.0_f32; key_count];
    let mut strain_data: Vec<f32> = Vec::with_capacity(raw_notes.len());
    let hand_split = key_count / 2;

    for (raw_t, cols) in &rows {
        let time = (*raw_t as f32) / r;
        let mut row_strains = vec![0.0_f32; key_count];
        for k in 0..key_count {
            if !cols.contains(&k) {
                continue;
            }
            let jack_delta = time - last_in_col[k];
            let j = if jack_delta > 0.0 {
                (15000.0_f32 / jack_delta).min(JACK_CURVE_CUTOFF)
            } else {
                0.0
            };
            let (hand_lo, hand_hi) = if k < hand_split {
                (0, hand_split.saturating_sub(1))
            } else {
                (hand_split, key_count - 1)
            };
            let mut sl = 0.0_f32;
            let mut sr = 0.0_f32;
            for (hand_k, &last_time) in last_in_col
                .iter()
                .enumerate()
                .take(hand_hi + 1)
                .skip(hand_lo)
            {
                if hand_k != k {
                    let trill_delta = time - last_time;
                    if trill_delta > 0.0 {
                        let trill_v =
                            ms_to_stream_bpm(trill_delta) * jack_comp(jack_delta, trill_delta);
                        if hand_k < k {
                            sl = sl.max(trill_v);
                        } else {
                            sr = sr.max(trill_v);
                        }
                    }
                }
            }
            let note_diff = ((STREAM_SCALE * sl.powf(STREAM_POW)).powf(OHT_NERF)
                + (STREAM_SCALE * sr.powf(STREAM_POW)).powf(OHT_NERF)
                + j.powf(OHT_NERF))
            .powf(1.0 / OHT_NERF);

            let delta = jack_delta.max(0.0);
            strain_values[k] = strain_func(1575.0, strain_values[k], note_diff, delta);
            row_strains[k] = strain_values[k];
            last_in_col[k] = time;
        }
        for s in row_strains {
            if s > 0.0 && s.is_finite() {
                strain_data.push(s);
            }
        }
    }

    if strain_data.is_empty() {
        return 0.0;
    }
    strain_data.sort_by(|a, b| a.total_cmp(b));
    let len = strain_data.len() as f32;
    let mut weight = 0.0_f32;
    let mut total = 0.0_f32;
    for (i, &val) in strain_data.iter().enumerate() {
        let pos = ((i as f32) + MOST_IMPORTANT_NOTES - len) / MOST_IMPORTANT_NOTES;
        let x = pos.max(0.0);
        let w = 0.002 + x.powi(4);
        weight += w;
        total += val * w;
    }
    if weight <= 0.0 {
        return 0.0;
    }
    let result = (total / weight).powf(CURVE_POWER) * CURVE_SCALE;
    if result.is_finite() {
        result as f64
    } else {
        0.0
    }
}

fn read_varint(buf: &[u8], mut pos: usize) -> Option<(u64, usize)> {
    let mut res = 0u64;
    let mut shift = 0u32;
    while pos < buf.len() {
        let b = buf[pos];
        pos += 1;
        res |= ((b & 0x7f) as u64) << shift;
        if (b & 0x80) == 0 {
            return Some((res, pos));
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    None
}

fn parse_proto_fields(buf: &[u8]) -> Vec<(u32, u8, Vec<u8>, u64)> {
    let mut pos = 0usize;
    let mut out = Vec::new();
    while pos < buf.len() {
        let Some((tag, next_pos)) = read_varint(buf, pos) else {
            break;
        };
        pos = next_pos;
        let fnum = (tag >> 3) as u32;
        let wtype = (tag & 7) as u8;
        match wtype {
            0 => {
                let Some((v, np)) = read_varint(buf, pos) else {
                    break;
                };
                pos = np;
                out.push((fnum, wtype, Vec::new(), v));
            }
            1 => {
                if pos + 8 > buf.len() {
                    break;
                }
                out.push((fnum, wtype, buf[pos..pos + 8].to_vec(), 0));
                pos += 8;
            }
            2 => {
                let Some((len, np)) = read_varint(buf, pos) else {
                    break;
                };
                pos = np;
                let l = len as usize;
                if pos + l > buf.len() {
                    break;
                }
                out.push((fnum, wtype, buf[pos..pos + l].to_vec(), 0));
                pos += l;
            }
            5 => {
                if pos + 4 > buf.len() {
                    break;
                }
                out.push((fnum, wtype, buf[pos..pos + 4].to_vec(), 0));
                pos += 4;
            }
            _ => break,
        }
    }
    out
}

fn parse_onnx_bytes(data: &[u8]) -> Option<OnnxEnsemble> {
    let mut base_value = 11.031088_f32;
    let mut nodes_falsenodeids = Vec::new();
    let mut nodes_featureids = Vec::new();
    let mut nodes_modes = Vec::new();
    let mut nodes_nodeids = Vec::new();
    let mut nodes_treeids = Vec::new();
    let mut nodes_truenodeids = Vec::new();
    let mut nodes_values = Vec::new();
    let mut target_nodeids = Vec::new();
    let mut target_treeids = Vec::new();
    let mut target_weights = Vec::new();

    for (fnum, _, val, _) in parse_proto_fields(data) {
        if fnum == 7 {
            for (gf, _, gv, _) in parse_proto_fields(&val) {
                if gf == 1 {
                    for (nf, _, nv, _) in parse_proto_fields(&gv) {
                        if nf == 5 {
                            let mut aname = String::new();
                            let mut floats = Vec::new();
                            let mut ints = Vec::new();
                            let mut strs = Vec::new();
                            for (af, aw, av, aval) in parse_proto_fields(&nv) {
                                if af == 1 {
                                    aname = String::from_utf8_lossy(&av).to_string();
                                } else if af == 4 && av.len() == 4 {
                                    floats.push(f32::from_le_bytes([av[0], av[1], av[2], av[3]]));
                                } else if af == 7 {
                                    if aw == 2 {
                                        for &chunk in av.as_chunks::<4>().0 {
                                            floats.push(f32::from_le_bytes(chunk));
                                        }
                                    } else if aw == 5 && av.len() == 4 {
                                        floats
                                            .push(f32::from_le_bytes([av[0], av[1], av[2], av[3]]));
                                    }
                                } else if af == 8 {
                                    if aw == 2 {
                                        let mut p = 0usize;
                                        while let Some((v, np)) = read_varint(&av, p) {
                                            ints.push(v as usize);
                                            p = np;
                                            if p >= av.len() {
                                                break;
                                            }
                                        }
                                    } else if aw == 0 {
                                        ints.push(aval as usize);
                                    }
                                } else if af == 9 {
                                    strs.push(String::from_utf8_lossy(&av).to_string());
                                }
                            }
                            match aname.as_str() {
                                "base_values" => {
                                    if let Some(&b) = floats.first() {
                                        base_value = b;
                                    }
                                }
                                "nodes_falsenodeids" => nodes_falsenodeids = ints,
                                "nodes_featureids" => nodes_featureids = ints,
                                "nodes_modes" => nodes_modes = strs,
                                "nodes_nodeids" => nodes_nodeids = ints,
                                "nodes_treeids" => nodes_treeids = ints,
                                "nodes_truenodeids" => nodes_truenodeids = ints,
                                "nodes_values" => nodes_values = floats,
                                "target_nodeids" => target_nodeids = ints,
                                "target_treeids" => target_treeids = ints,
                                "target_weights" => target_weights = floats,
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    if nodes_treeids.is_empty() || nodes_values.len() != nodes_treeids.len() {
        return None;
    }
    let num_trees = nodes_treeids.iter().copied().max()? + 1;
    let mut trees = vec![Vec::new(); num_trees];
    for i in 0..nodes_treeids.len() {
        let tid = nodes_treeids[i];
        let nid = nodes_nodeids[i];
        if trees[tid].len() <= nid {
            trees[tid].resize(
                nid + 1,
                OnnxTreeNode {
                    is_leaf: true,
                    feature_id: 0,
                    threshold: 0.0,
                    true_id: 0,
                    false_id: 0,
                    weight: 0.0,
                },
            );
        }
        trees[tid][nid] = OnnxTreeNode {
            is_leaf: nodes_modes.get(i).map(|s| s == "LEAF").unwrap_or(true),
            feature_id: nodes_featureids.get(i).copied().unwrap_or(0),
            threshold: nodes_values.get(i).copied().unwrap_or(0.0),
            true_id: nodes_truenodeids.get(i).copied().unwrap_or(0),
            false_id: nodes_falsenodeids.get(i).copied().unwrap_or(0),
            weight: 0.0,
        };
    }
    for i in 0..target_treeids.len() {
        let tid = target_treeids[i];
        let nid = target_nodeids[i];
        if let Some(node) = trees.get_mut(tid).and_then(|t| t.get_mut(nid)) {
            node.weight = target_weights.get(i).copied().unwrap_or(0.0);
        }
    }
    Some(OnnxEnsemble { base_value, trees })
}

pub fn evaluate_companella_onnx(features: &[f32; 10]) -> f32 {
    let model = ONNX_MODEL.get_or_init(|| {
        if let Some(dir) = dirs_next::data_local_dir() {
            let ext = dir
                .join("oris-mania-utils")
                .join("engines")
                .join("dan_model.onnx");
            if let Ok(bytes) = std::fs::read(&ext) {
                if let Some(m) = parse_onnx_bytes(&bytes) {
                    return Some(m);
                }
            }
        }
        let embedded = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/dan_model.onnx"
        ));
        parse_onnx_bytes(embedded)
    });
    let Some(ens) = model.as_ref() else {
        return (features[0] * 0.55 - 1.5).clamp(0.0, 20.0);
    };
    let mut total = ens.base_value;
    for tree in &ens.trees {
        if tree.is_empty() {
            continue;
        }
        let mut nid = 0usize;
        for _ in 0..64 {
            let Some(node) = tree.get(nid) else { break };
            if node.is_leaf {
                total += node.weight;
                break;
            }
            let fval = features.get(node.feature_id).copied().unwrap_or(0.0);
            nid = if fval <= node.threshold {
                node.true_id
            } else {
                node.false_id
            };
        }
    }
    total.clamp(0.0, 20.0)
}

// companella onnx training targets are 0-based (0=1st dan), shifted +1.5 to align with tier midpoints
pub fn evaluate_companella_multilayer(
    raw_notes: &[RawChartNote],
    key_count: usize,
    rate: f32,
    od: f64,
    msd: &[f32; 8],
) -> (f32, f32, f32) {
    let interlude = calculate_interlude(raw_notes, key_count, rate) as f32;
    let sunny = calculate_sunny_with_od(raw_notes, key_count, rate, od) as f32;
    let features = [
        msd[0], msd[1], msd[2], msd[3], msd[4], msd[5], msd[6], msd[7], interlude, sunny,
    ];
    let raw_onnx = evaluate_companella_onnx(&features);
    let rice_dan = (raw_onnx + 1.5).clamp(1.0, 20.99);

    let ln_count = raw_notes.iter().filter(|n| n.tail_ms >= 0).count();
    let ln_pct = if raw_notes.is_empty() {
        0.0
    } else {
        (ln_count as f32 / raw_notes.len() as f32) * 100.0
    };
    let companella_dan = if ln_pct >= 15.0 {
        if let Some(ln_raw) = calculate_pure_ln_dan(raw_notes, key_count, rate, od) {
            let ln_aligned = (ln_raw as f32 + 0.5).clamp(1.0, 20.99);
            let w = ((ln_pct - 15.0) / 35.0).clamp(0.0, 0.70);
            (rice_dan * (1.0 - w) + ln_aligned * w).clamp(1.0, 20.99)
        } else {
            rice_dan
        }
    } else {
        rice_dan
    };
    (companella_dan, interlude, sunny)
}
