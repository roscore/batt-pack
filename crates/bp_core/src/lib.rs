//! Textbook battery-pack arithmetic. No web dependencies.
//!
//! Model: constant nominal voltage for energy/current, linear internal resistance,
//! open-circuit voltage linear between full and empty (only used for the end-of-DoD check),
//! no temperature or aging effects. Battery-side current = P / (V_nominal * efficiency).

/// Per-cell parameters. All values are user-editable "typical" numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub v_nom: f64,
    pub v_full: f64,
    pub v_empty: f64,
    pub capacity_ah: f64,
    pub max_c: f64,
    pub ir_mohm: f64,
    pub mass_g: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chemistry {
    LiIonNmc,
    LiFePo4,
    LiPo,
}

impl Chemistry {
    pub const ALL: [Chemistry; 3] = [Chemistry::LiIonNmc, Chemistry::LiFePo4, Chemistry::LiPo];

    /// (nominal, full, empty) cell voltage.
    pub fn voltages(self) -> (f64, f64, f64) {
        match self {
            Chemistry::LiIonNmc => (3.6, 4.2, 3.0),
            Chemistry::LiFePo4 => (3.2, 3.65, 2.5),
            Chemistry::LiPo => (3.7, 4.2, 3.3),
        }
    }
}

/// Generic, typical cell presets (not any specific product).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellPreset {
    Generic18650,
    Generic21700,
}

impl CellPreset {
    /// (capacity Ah, max continuous C, internal resistance mΩ, mass g)
    pub fn params(self) -> (f64, f64, f64, f64) {
        match self {
            CellPreset::Generic18650 => (3.0, 5.0, 30.0, 47.0),
            CellPreset::Generic21700 => (5.0, 3.0, 20.0, 70.0),
        }
    }

    pub fn cell(self, chem: Chemistry) -> Cell {
        let (v_nom, v_full, v_empty) = chem.voltages();
        let (capacity_ah, max_c, ir_mohm, mass_g) = self.params();
        Cell { v_nom, v_full, v_empty, capacity_ah, max_c, ir_mohm, mass_g }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pack {
    pub cell: Cell,
    pub s: u32,
    pub p: u32,
    /// BMS, wiring, case overhead as % of cell mass.
    pub overhead_pct: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PackStats {
    pub v_nom: f64,
    pub v_full: f64,
    pub v_empty: f64,
    pub capacity_ah: f64,
    pub energy_wh: f64,
    pub mass_kg: f64,
    pub wh_per_kg: f64,
    pub max_current_a: f64,
    pub r_pack_ohm: f64,
}

impl Pack {
    pub fn stats(&self) -> PackStats {
        let s = self.s as f64;
        let p = self.p as f64;
        let c = &self.cell;
        let capacity_ah = c.capacity_ah * p;
        let energy_wh = c.v_nom * s * capacity_ah;
        let mass_kg = c.mass_g * s * p * (1.0 + self.overhead_pct / 100.0) / 1000.0;
        PackStats {
            v_nom: c.v_nom * s,
            v_full: c.v_full * s,
            v_empty: c.v_empty * s,
            capacity_ah,
            energy_wh,
            mass_kg,
            wh_per_kg: if mass_kg > 0.0 { energy_wh / mass_kg } else { 0.0 },
            max_current_a: c.max_c * capacity_ah,
            r_pack_ohm: s * c.ir_mohm / 1000.0 / p,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Load {
    pub avg_w: f64,
    pub peak_w: f64,
    /// Usable depth of discharge, 0..1.
    pub dod: f64,
    /// Converter efficiency, 0..1.
    pub efficiency: f64,
}

impl Default for Load {
    fn default() -> Self {
        Load { avg_w: 50.0, peak_w: 150.0, dod: 0.8, efficiency: 0.95 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Ok,
    Tight,
    Over,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoadResult {
    pub runtime_h: f64,
    pub runtime_min: f64,
    pub avg_current_a: f64,
    pub peak_current_a: f64,
    pub sag_v: f64,
    /// Open-circuit voltage at the end of usable DoD minus peak sag.
    pub loaded_end_v: f64,
    pub heat_w: f64,
    pub c_rate_peak: f64,
    pub verdict: Verdict,
}

/// Fraction of the current limit at which the verdict becomes "tight".
pub const TIGHT_FRACTION: f64 = 0.9;

pub fn evaluate(st: &PackStats, load: &Load) -> LoadResult {
    let runtime_h = if load.avg_w > 0.0 {
        st.energy_wh * load.dod * load.efficiency / load.avg_w
    } else {
        f64::INFINITY
    };
    let denom = st.v_nom * load.efficiency;
    let current = |w: f64| if denom > 0.0 { w / denom } else { f64::INFINITY };
    let avg_current_a = current(load.avg_w);
    let peak_current_a = current(load.peak_w);
    let sag_v = peak_current_a * st.r_pack_ohm;
    let ocv_end = st.v_full - load.dod * (st.v_full - st.v_empty);
    let loaded_end_v = ocv_end - sag_v;
    let verdict = if peak_current_a > st.max_current_a {
        Verdict::Over
    } else if peak_current_a >= TIGHT_FRACTION * st.max_current_a || loaded_end_v < st.v_empty {
        Verdict::Tight
    } else {
        Verdict::Ok
    };
    LoadResult {
        runtime_h,
        runtime_min: runtime_h * 60.0,
        avg_current_a,
        peak_current_a,
        sag_v,
        loaded_end_v,
        heat_w: peak_current_a * peak_current_a * st.r_pack_ohm,
        c_rate_peak: if st.capacity_ah > 0.0 { peak_current_a / st.capacity_ah } else { 0.0 },
        verdict,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    pub v_min: f64,
    pub v_max: f64,
    pub runtime_min: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Suggestion {
    pub s: u32,
    pub p: u32,
    pub stats: PackStats,
    pub result: LoadResult,
}

pub const MAX_S: u32 = 24;
pub const MAX_P: u32 = 10;

/// Up to `limit` lightest S×P combos (S 1–24, P 1–10) meeting the voltage window,
/// runtime and peak-current limit, sorted by mass (ties: fewer cells, then lower S).
pub fn suggest(cell: Cell, overhead_pct: f64, load: &Load, target: &Target, limit: usize) -> Vec<Suggestion> {
    let mut out = Vec::new();
    for s in 1..=MAX_S {
        for p in 1..=MAX_P {
            let stats = Pack { cell, s, p, overhead_pct }.stats();
            if stats.v_nom < target.v_min || stats.v_nom > target.v_max {
                continue;
            }
            let result = evaluate(&stats, load);
            if result.runtime_min >= target.runtime_min && result.peak_current_a <= stats.max_current_a {
                out.push(Suggestion { s, p, stats, result });
            }
        }
    }
    out.sort_by(|a, b| {
        a.stats.mass_kg.total_cmp(&b.stats.mass_kg).then((a.s * a.p).cmp(&(b.s * b.p))).then(a.s.cmp(&b.s))
    });
    out.truncate(limit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    fn pack_4s2p() -> Pack {
        Pack { cell: CellPreset::Generic18650.cell(Chemistry::LiIonNmc), s: 4, p: 2, overhead_pct: 15.0 }
    }

    #[test]
    fn pack_voltage_capacity_energy() {
        let st = pack_4s2p().stats();
        assert!(close(st.v_nom, 14.4, 1e-9));
        assert!(close(st.v_full, 16.8, 1e-9));
        assert!(close(st.v_empty, 12.0, 1e-9));
        assert!(close(st.capacity_ah, 6.0, 1e-9));
        assert!(close(st.energy_wh, 86.4, 1e-9));
        assert!(close(st.max_current_a, 30.0, 1e-9)); // 5 C * 6 Ah
    }

    #[test]
    fn mass_with_overhead() {
        let st = pack_4s2p().stats();
        // 8 cells * 47 g * 1.15 = 432.4 g
        assert!(close(st.mass_kg, 0.4324, 1e-9));
        assert!(close(st.wh_per_kg, 86.4 / 0.4324, 1e-9));
    }

    #[test]
    fn pack_resistance() {
        let st = pack_4s2p().stats();
        assert!(close(st.r_pack_ohm, 0.060, 1e-12)); // 4 * 30 / 2 = 60 mΩ
    }

    #[test]
    fn runtime_and_peak() {
        let st = pack_4s2p().stats();
        let r = evaluate(&st, &Load::default());
        // 86.4 * 0.8 * 0.95 / 50 = 1.31328 h
        assert!(close(r.runtime_h, 1.31328, 1e-9));
        assert!(close(r.runtime_min, 78.7968, 1e-6));
        // 150 W / (14.4 V * 0.95) = 10.965 A; sag = 10.965 * 0.06 = 0.658 V
        let i = 150.0 / (14.4 * 0.95);
        assert!(close(r.peak_current_a, i, 1e-9));
        assert!(close(r.sag_v, i * 0.06, 1e-9));
        assert!(close(r.heat_w, i * i * 0.06, 1e-9));
        // OCV at 80 % DoD = 16.8 - 0.8 * 4.8 = 12.96 V
        assert!(close(r.loaded_end_v, 12.96 - i * 0.06, 1e-9));
        assert!(close(r.c_rate_peak, i / 6.0, 1e-9));
        assert_eq!(r.verdict, Verdict::Ok);
    }

    #[test]
    fn verdict_thresholds() {
        let st = pack_4s2p().stats(); // limit 30 A
        let at = |amps: f64| Load { peak_w: amps * 14.4 * 0.95, ..Load::default() };
        // Small packs: dod 0 keeps the voltage check out of the way for the current thresholds.
        let cur = |amps: f64| Load { dod: 0.0, ..at(amps) };
        assert_eq!(evaluate(&st, &cur(26.9)).verdict, Verdict::Ok);
        assert_eq!(evaluate(&st, &cur(27.01)).verdict, Verdict::Tight);
        assert_eq!(evaluate(&st, &cur(29.99)).verdict, Verdict::Tight);
        assert_eq!(evaluate(&st, &cur(30.1)).verdict, Verdict::Over);
        // Voltage check: 20 A -> sag 1.2 V > 0.96 V headroom at 80 % DoD.
        assert_eq!(evaluate(&st, &at(20.0)).verdict, Verdict::Tight);
        assert_eq!(evaluate(&st, &at(15.0)).verdict, Verdict::Ok);
    }

    #[test]
    fn suggest_satisfies_constraints_sorted() {
        let cell = CellPreset::Generic18650.cell(Chemistry::LiIonNmc);
        let load = Load::default();
        let target = Target { v_min: 12.0, v_max: 26.0, runtime_min: 60.0 };
        let res = suggest(cell, 15.0, &load, &target, 5);
        assert!(!res.is_empty() && res.len() <= 5);
        for s in &res {
            assert!(s.stats.v_nom >= 12.0 && s.stats.v_nom <= 26.0);
            assert!(s.result.runtime_min >= 60.0);
            assert!(s.result.peak_current_a <= s.stats.max_current_a);
        }
        assert!(res.windows(2).all(|w| w[0].stats.mass_kg <= w[1].stats.mass_kg));
        // 60 min at 50 W needs 65.8 Wh -> 7 cells (75.6 Wh) minimum, only 7S1P fits the window.
        assert_eq!((res[0].s, res[0].p), (7, 1));
    }

    #[test]
    fn suggest_empty_when_impossible() {
        let cell = CellPreset::Generic18650.cell(Chemistry::LiIonNmc);
        let target = Target { v_min: 100.0, v_max: 101.0, runtime_min: 10.0 };
        assert!(suggest(cell, 15.0, &Load::default(), &target, 5).is_empty());
        let target = Target { v_min: 10.0, v_max: 15.0, runtime_min: 100_000.0 };
        assert!(suggest(cell, 15.0, &Load::default(), &target, 5).is_empty());
    }
}
