mod companella;
mod xdc1;

use serde::{Deserialize, Serialize};

pub use companella::{
    calculate_interlude, calculate_pure_ln_dan, calculate_sunny, calculate_sunny_with_od,
    evaluate_companella_multilayer, evaluate_companella_onnx,
};
pub use xdc1::{
    analyse_xdc1, analyse_xdc1_with_curve, points_to_raw, raw_to_dan, xdc1_diff_to_numeric,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DanResult {
    pub level: f32,
    pub tier: u8,
    pub symbol: String,
    pub name: String,
    pub variant: String,
    pub sub_label: String,
    pub display: String,
    pub xdc1_dan: f32,
    pub xdc1_points: f32,
    pub xdc1_rice_points: f32,
    pub companella_dan: f32,
    pub interlude: f32,
    pub sunny: f32,
    #[serde(default)]
    pub strain_curve: Vec<f32>,
    #[serde(default)]
    pub duration_ms: i64,
}

#[derive(Clone, Copy, Debug)]
pub struct RawChartNote {
    pub col: usize,
    pub head_ms: i64,
    pub tail_ms: i64,
}

pub fn default_dan_tier_color(tier: u8) -> &'static str {
    match tier {
        1..=3 => "#7dd3fc",
        4..=6 => "#93c5fd",
        7..=8 => "#a5b4fc",
        9..=10 => "#e2e8f0",
        11 => "#ff5a5a",
        12 => "#ffd84d",
        13 => "#00ffd5",
        14 => "#ff7b00",
        15 => "#ff7a9e",
        16 => "#d7f7ff",
        17 => "#ff2b2b",
        18 => "#cc00ff",
        19 => "#f8fafc",
        20 => "#ffffff",
        _ => "#a1a1a6",
    }
}

pub fn format_dan_tier(level: f32) -> (u8, String, String, String, String, String) {
    if level <= 0.05 {
        return (
            0,
            "-".into(),
            "unrated".into(),
            "".into(),
            "".into(),
            "-".into(),
        );
    }
    let clamped = level.clamp(1.0, 20.99);
    let tier = (clamped.floor() as u8).clamp(1, 20);
    let frac = clamped - (tier as f32);
    let variant = if frac < 0.20 {
        "--"
    } else if frac < 0.40 {
        "-"
    } else if frac < 0.60 {
        ""
    } else if frac < 0.80 {
        "+"
    } else {
        "++"
    };
    let sub_label = if frac < 0.333 {
        "low"
    } else if frac < 0.667 {
        "mid"
    } else {
        "high"
    };

    let (symbol, name) = match tier {
        1 => ("I", "1st dan"),
        2 => ("II", "2nd dan"),
        3 => ("III", "3rd dan"),
        4 => ("IV", "4th dan"),
        5 => ("V", "5th dan"),
        6 => ("VI", "6th dan"),
        7 => ("VII", "7th dan"),
        8 => ("VIII", "8th dan"),
        9 => ("IX", "9th dan"),
        10 => ("X", "10th dan"),
        11 => ("α", "alpha"),
        12 => ("β", "beta"),
        13 => ("γ", "gamma"),
        14 => ("δ", "delta"),
        15 => ("ε", "epsilon"),
        16 => ("ζ", "zeta"),
        17 => ("η", "eta"),
        18 => ("θ", "theta"),
        19 => ("ι", "iota"),
        _ => ("κ", "kappa"),
    };
    let display = if variant.is_empty() {
        format!("{} · {} {}", symbol, name, sub_label)
    } else {
        format!("{}{} · {} {}", symbol, variant, name, sub_label)
    };
    (
        tier,
        symbol.into(),
        name.into(),
        variant.into(),
        sub_label.into(),
        display,
    )
}

pub fn calculate_combined_dan(
    raw_notes: &[RawChartNote],
    key_count: usize,
    rate: f32,
    msd: &[f32; 8],
    mode: &str,
) -> DanResult {
    calculate_combined_dan_with_od(raw_notes, key_count, rate, 8.0, msd, mode)
}

pub fn calculate_combined_dan_with_od(
    raw_notes: &[RawChartNote],
    key_count: usize,
    rate: f32,
    od: f64,
    msd: &[f32; 8],
    mode: &str,
) -> DanResult {
    if raw_notes.len() < 2 {
        return DanResult::default();
    }
    let (companella_dan, interlude, sunny) =
        evaluate_companella_multilayer(raw_notes, key_count, rate, od, msd);

    let (xdc1_dan_diff, xdc1_pts, xdc1_rice_pts, strain_curve, duration_ms) = if key_count == 4 {
        analyse_xdc1_with_curve(raw_notes, rate)
    } else {
        (0.0, 0.0, 0.0, Vec::new(), 0)
    };
    let xdc1_dan = if key_count == 4 {
        xdc1_diff_to_numeric(xdc1_dan_diff) as f32
    } else {
        0.0
    };

    let combined_level = match mode {
        "xdc1" if key_count == 4 && xdc1_dan > 0.5 => xdc1_dan.clamp(1.0, 20.99),
        "companella" => companella_dan.clamp(1.0, 20.99),
        _ => {
            if key_count != 4 || xdc1_dan < 2.0 {
                companella_dan.clamp(1.0, 20.99)
            } else if xdc1_dan < 4.0 {
                let t = ((xdc1_dan - 2.0) / 2.0).clamp(0.0, 1.0);
                (companella_dan * (1.0 - 0.50 * t) + xdc1_dan * (0.50 * t)).clamp(1.0, 20.99)
            } else {
                (0.50 * xdc1_dan + 0.50 * companella_dan).clamp(1.0, 20.99)
            }
        }
    };

    let (tier, symbol, name, variant, sub_label, display) = format_dan_tier(combined_level);
    DanResult {
        level: (combined_level * 100.0).round() / 100.0,
        tier,
        symbol,
        name,
        variant,
        sub_label,
        display,
        xdc1_dan: (xdc1_dan * 100.0).round() / 100.0,
        xdc1_points: (xdc1_pts as f32 * 100.0).round() / 100.0,
        xdc1_rice_points: (xdc1_rice_pts as f32 * 100.0).round() / 100.0,
        companella_dan: (companella_dan * 100.0).round() / 100.0,
        interlude: (interlude * 100.0).round() / 100.0,
        sunny: (sunny * 100.0).round() / 100.0,
        strain_curve,
        duration_ms,
    }
}

impl DanResult {
    pub fn resolve_for_mode(
        &self,
        mode: &str,
    ) -> (f32, u8, String, String, String, String, String) {
        let lvl = match mode {
            "xdc1" if self.xdc1_dan > 0.5 => self.xdc1_dan,
            "companella" if self.companella_dan > 0.5 => self.companella_dan,
            _ => self.level,
        };
        let (tier, sym, name, var, sub, disp) = format_dan_tier(lvl);
        (
            (lvl * 100.0).round() / 100.0,
            tier,
            sym,
            name,
            var,
            sub,
            disp,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xdc1_matches_reference() {
        let notes: Vec<RawChartNote> = (0..400)
            .map(|i| RawChartNote {
                col: i % 4,
                head_ms: (i as i64) * 65,
                tail_ms: -1,
            })
            .collect();
        let (dan, pts, pts_rice) = analyse_xdc1(&notes, 1.0);
        assert!((pts - 18.1436).abs() < 0.01, "pts={}", pts);
        assert!((pts_rice - 18.1436).abs() < 0.01, "pts_rice={}", pts_rice);
        assert!((dan - 0.8022).abs() < 0.01, "dan={}", dan);
    }

    #[test]
    fn test_companella_onnx_matches_reference() {
        let msd = 26.0_f32;
        let feats = [
            msd,
            msd * 0.9,
            msd * 0.85,
            msd * 0.8,
            msd * 0.88,
            msd * 0.75,
            msd * 0.82,
            msd * 0.78,
            msd * 0.28,
            msd * 0.32,
        ];
        let val = evaluate_companella_onnx(&feats);
        assert!((val - 11.706).abs() < 0.02, "onnx val={}", val);
    }

    #[test]
    fn test_interlude_and_sunny_realistic_ranges() {
        let notes: Vec<RawChartNote> = (0..600)
            .map(|i| RawChartNote {
                col: [0, 2, 1, 3][i % 4],
                head_ms: (i as i64) * 62 + 100,
                tail_ms: -1,
            })
            .collect();
        let inter = calculate_interlude(&notes, 4, 1.0);
        let sunny = calculate_sunny_with_od(&notes, 4, 1.0, 8.0);
        assert!(inter > 4.0 && inter < 16.0, "inter={}", inter);
        assert!(sunny > 3.0 && sunny < 12.0, "sunny={}", sunny);
    }

    #[test]
    fn test_engines_stay_aligned_on_realistic_chart() {
        let mut osu = String::from(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:4\nOverallDifficulty:8.5\n\n[HitObjects]\n",
        );
        let xs = [64, 192, 320, 448];
        for i in 0..800 {
            let t = i * 62 + 1000;
            let c1 = [0, 2, 1, 3][i as usize % 4];
            osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", xs[c1], t));
            if i % 4 == 0 {
                let c2 = [1, 3, 2, 0][i as usize % 4];
                osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", xs[c2], t));
            }
        }
        let sr = crate::calc::calculate_from_osu_content(&osu, 1.0, None, None).expect("calc ok");
        assert!(
            (sr.dan.xdc1_dan - sr.dan.companella_dan).abs() < 2.2,
            "xdc1={} comp={}",
            sr.dan.xdc1_dan,
            sr.dan.companella_dan
        );
        assert!(
            sr.dan.level >= 7.0 && sr.dan.level <= 15.0,
            "level={}",
            sr.dan.level
        );
    }
}
