//! Static web UI: reads inputs, calls bp_core, writes results. Plain web-sys, no framework.

use bp_core::{evaluate, suggest, Cell, CellPreset, Chemistry, Load, Pack, Target, Verdict};
use std::cell::Cell as StdCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, Document, HtmlCanvasElement, HtmlInputElement, HtmlSelectElement};

thread_local! {
    static KO: StdCell<bool> = const { StdCell::new(false) };
}

fn ko() -> bool {
    KO.with(|k| k.get())
}

/// UI strings: (key, Korean, English).
const STRINGS: &[(&str, &str, &str)] = &[
    ("title", "로봇 배터리 팩 플래너", "Robot battery pack planner"),
    ("subtitle", "셀 몇 개, 몇 S×P, 로봇은 얼마나 달릴까?", "How many cells, in what S×P, and how long will my robot run?"),
    ("sec_cell", "셀 + 팩", "Cell + pack"),
    ("sec_load", "부하 + 런타임", "Load + runtime"),
    ("sec_sugg", "S×P 추천", "Suggest S×P"),
    ("sec_grid", "셀 배열", "Cell grid"),
    ("chem", "화학 조성", "Chemistry"),
    ("preset", "셀 프리셋 (typical, 수정 가능)", "Cell preset (typical, editable)"),
    ("p18650", "일반 18650", "generic 18650"),
    ("p21700", "일반 21700", "generic 21700"),
    ("v_nom", "공칭 전압 (V)", "Nominal voltage (V)"),
    ("v_full", "만충 전압 (V)", "Full voltage (V)"),
    ("v_empty", "방전 종지 전압 (V)", "Empty voltage (V)"),
    ("cap", "셀 용량 (Ah)", "Cell capacity (Ah)"),
    ("maxc", "최대 연속 방전 (C)", "Max continuous discharge (C)"),
    ("ir", "내부 저항 (mΩ)", "Internal resistance (mΩ)"),
    ("mass", "셀 무게 (g)", "Cell mass (g)"),
    ("s", "직렬 S", "Series S"),
    ("p", "병렬 P", "Parallel P"),
    ("ovh", "팩 오버헤드 무게 (%)", "Pack overhead mass (%)"),
    ("avg", "평균 전력 (W)", "Average power (W)"),
    ("peak", "피크 전력 (W)", "Peak power (W)"),
    ("dod", "사용 방전 심도 DoD (%)", "Usable depth of discharge (%)"),
    ("eff", "컨버터 효율 (0–1)", "Converter efficiency (0–1)"),
    ("tvmin", "목표 공칭 전압 최소 (V)", "Target nominal voltage min (V)"),
    ("tvmax", "목표 공칭 전압 최대 (V)", "Target nominal voltage max (V)"),
    ("trun", "목표 런타임 (분)", "Target runtime (min)"),
    ("o_v", "전압 공칭 / 만충 / 방전", "Voltage nominal / full / empty"),
    ("o_cap", "용량", "Capacity"),
    ("o_mass", "무게", "Mass"),
    ("o_imax", "최대 연속 전류", "Max continuous current"),
    ("o_r", "팩 저항 (S·R/P)", "Pack resistance (S·R/P)"),
    ("o_run", "런타임 (평균 부하)", "Runtime (average load)"),
    ("o_iavg", "평균 / 피크 전류", "Average / peak current"),
    ("o_sag", "피크 전압 강하 (I·R)", "Voltage sag at peak (I·R)"),
    ("o_vend", "DoD 끝 부하 전압 (종지 전압)", "Loaded voltage at end of DoD (cutoff)"),
    ("o_heat", "피크 I²R 발열", "I²R heat at peak"),
    ("o_crate", "피크 C-rate", "C-rate at peak"),
    ("ok", "✅ 여유 있음", "✅ ok"),
    ("tight", "⚠️ 빠듯함 (전류 ≥ 한계 90 % 또는 종지 전압 아래로 처짐)", "⚠️ tight (current ≥ 90 % of limit or sags below cutoff)"),
    ("over", "❌ 전류 한계 초과", "❌ over current limit"),
    ("none", "조건을 만족하는 조합이 없습니다 (S 1–24, P 1–10).", "No combination meets the targets (S 1–24, P 1–10)."),
    ("h_combo", "구성", "Config"),
    ("h_run", "런타임", "Runtime"),
    ("h_peak", "피크 전류", "Peak current"),
    ("use", "적용", "Use"),
    ("disclaimer", "추정값일 뿐입니다. 반드시 셀 데이터시트를 따르고 제대로 된 BMS를 사용하세요.", "Estimates only — follow cell datasheets and use a proper BMS."),
];

fn t(key: &str) -> &'static str {
    STRINGS
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, k, e)| if ko() { *k } else { *e })
        .unwrap_or("?")
}

fn doc() -> Document {
    web_sys::window().unwrap().document().unwrap()
}

fn input(id: &str) -> HtmlInputElement {
    doc().get_element_by_id(id).unwrap().dyn_into().unwrap()
}

fn select(id: &str) -> HtmlSelectElement {
    doc().get_element_by_id(id).unwrap().dyn_into().unwrap()
}

fn num(id: &str) -> f64 {
    input(id).value().trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite()).unwrap_or(0.0)
}

fn int(id: &str, lo: u32, hi: u32) -> u32 {
    (num(id).round().max(lo as f64) as u32).min(hi)
}

fn set_val(id: &str, v: f64) {
    input(id).set_value(&format!("{}", (v * 1000.0).round() / 1000.0));
}

fn set_html(id: &str, html: &str) {
    if let Some(el) = doc().get_element_by_id(id) {
        el.set_inner_html(html);
    }
}

fn chem() -> Chemistry {
    Chemistry::ALL[select("chem").selected_index().clamp(0, 2) as usize]
}

fn preset() -> CellPreset {
    if select("preset").selected_index() == 1 { CellPreset::Generic21700 } else { CellPreset::Generic18650 }
}

fn apply_chem() {
    let (n, f, e) = chem().voltages();
    set_val("v_nom", n);
    set_val("v_full", f);
    set_val("v_empty", e);
}

fn apply_preset() {
    let (cap, c, ir, m) = preset().params();
    set_val("cap", cap);
    set_val("maxc", c);
    set_val("ir", ir);
    set_val("mass", m);
}

fn read_cell() -> Cell {
    Cell {
        v_nom: num("v_nom"),
        v_full: num("v_full"),
        v_empty: num("v_empty"),
        capacity_ah: num("cap"),
        max_c: num("maxc"),
        ir_mohm: num("ir"),
        mass_g: num("mass"),
    }
}

fn read_load() -> Load {
    Load {
        avg_w: num("avg"),
        peak_w: num("peak"),
        dod: (num("dod") / 100.0).clamp(0.0, 1.0),
        efficiency: num("eff").clamp(0.01, 1.0),
    }
}

fn row(label: &str, value: String) -> String {
    format!("<tr><th>{}</th><td>{}</td></tr>", t(label), value)
}

fn fmt_runtime(h: f64) -> String {
    if h.is_finite() { format!("{:.2} h · {:.0} min", h, h * 60.0) } else { "∞".into() }
}

fn recompute() {
    let pack = Pack { cell: read_cell(), s: int("s", 1, 99), p: int("p", 1, 99), overhead_pct: num("ovh") };
    let st = pack.stats();
    let load = read_load();
    let r = evaluate(&st, &load);

    let pack_html = [
        row("o_v", format!("{:.1} / {:.1} / {:.1} V", st.v_nom, st.v_full, st.v_empty)),
        row("o_cap", format!("{:.2} Ah · {:.1} Wh", st.capacity_ah, st.energy_wh)),
        row("o_mass", format!("{:.0} g · {:.0} Wh/kg", st.mass_kg * 1000.0, st.wh_per_kg)),
        row("o_imax", format!("{:.1} A", st.max_current_a)),
        row("o_r", format!("{:.1} mΩ", st.r_pack_ohm * 1000.0)),
    ]
    .concat();
    set_html("pack_out", &format!("<table>{pack_html}</table>"));

    let load_html = [
        row("o_run", fmt_runtime(r.runtime_h)),
        row("o_iavg", format!("{:.2} / {:.2} A", r.avg_current_a, r.peak_current_a)),
        row("o_sag", format!("{:.2} V", r.sag_v)),
        row("o_vend", format!("{:.2} V ({:.1} V)", r.loaded_end_v, st.v_empty)),
        row("o_heat", format!("{:.2} W", r.heat_w)),
        row("o_crate", format!("{:.2} C", r.c_rate_peak)),
    ]
    .concat();
    set_html("load_out", &format!("<table>{load_html}</table>"));
    let (cls, key) = match r.verdict {
        Verdict::Ok => ("ok", "ok"),
        Verdict::Tight => ("tight", "tight"),
        Verdict::Over => ("over", "over"),
    };
    set_html("verdict", &format!("<div class=\"verdict {cls}\">{}</div>", t(key)));

    let target = Target { v_min: num("tvmin"), v_max: num("tvmax"), runtime_min: num("trun") };
    let list = suggest(pack.cell, pack.overhead_pct, &load, &target, 5);
    let sugg_html = if list.is_empty() {
        format!("<p>{}</p>", t("none"))
    } else {
        let rows: String = list
            .iter()
            .map(|x| {
                format!(
                    "<tr><td>{s}S{p}P</td><td>{:.1} V</td><td>{:.0} min</td><td>{:.0} g</td><td>{:.1} A</td>\
                     <td><button type=\"button\" data-s=\"{s}\" data-p=\"{p}\">{}</button></td></tr>",
                    x.stats.v_nom,
                    x.result.runtime_min,
                    x.stats.mass_kg * 1000.0,
                    x.result.peak_current_a,
                    t("use"),
                    s = x.s,
                    p = x.p,
                )
            })
            .collect();
        format!(
            "<table class=\"list\"><tr><th>{}</th><th>V</th><th>{}</th><th>{}</th><th>{}</th><th></th></tr>{rows}</table>",
            t("h_combo"),
            t("h_run"),
            t("o_mass"),
            t("h_peak"),
        )
    };
    set_html("sugg_out", &sugg_html);

    draw_grid(pack.s, pack.p);
}

fn draw_grid(s: u32, p: u32) {
    let Some(canvas) = doc().get_element_by_id("grid").and_then(|e| e.dyn_into::<HtmlCanvasElement>().ok()) else {
        return;
    };
    let w = canvas.client_width().max(200) as f64;
    let cell = (w / (s as f64 + 0.5)).min(40.0).max(8.0);
    let h = cell * p as f64 + 28.0;
    canvas.set_width(w as u32);
    canvas.set_height(h as u32);
    let ctx: CanvasRenderingContext2d = canvas.get_context("2d").ok().flatten().unwrap().dyn_into().unwrap();
    let style = web_sys::window().unwrap().get_computed_style(&canvas).ok().flatten();
    let fg = style.and_then(|s| s.get_property_value("color").ok()).unwrap_or_else(|| "#222".into());
    ctx.clear_rect(0.0, 0.0, w, h);
    let x0 = (w - cell * s as f64) / 2.0;
    let r = cell * 0.42;
    ctx.set_line_width(1.5);
    ctx.set_font(&format!("{}px sans-serif", (cell * 0.35).clamp(9.0, 13.0)));
    ctx.set_text_align("center");
    for i in 0..s {
        let cx = x0 + cell * (i as f64 + 0.5);
        // Series group box
        ctx.set_stroke_style_str("#888");
        ctx.stroke_rect(cx - cell / 2.0 + 1.0, 2.0, cell - 2.0, cell * p as f64);
        for j in 0..p {
            let cy = 2.0 + cell * (j as f64 + 0.5);
            ctx.begin_path();
            let _ = ctx.arc(cx, cy, r, 0.0, std::f64::consts::TAU);
            ctx.set_fill_style_str(if i % 2 == 0 { "#4a90d9" } else { "#5bb56a" });
            ctx.fill();
            ctx.set_stroke_style_str(&fg);
            ctx.stroke();
        }
        if s <= 12 || i % 2 == 0 {
            ctx.set_fill_style_str(&fg);
            let _ = ctx.fill_text(&format!("S{}", i + 1), cx, cell * p as f64 + 20.0);
        }
    }
}

fn apply_lang() {
    let d = doc();
    d.document_element().unwrap().set_attribute("lang", if ko() { "ko" } else { "en" }).ok();
    let nodes = d.query_selector_all("[data-i18n]").unwrap();
    for i in 0..nodes.length() {
        if let Some(el) = nodes.item(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
            if let Some(k) = el.get_attribute("data-i18n") {
                el.set_text_content(Some(t(&k)));
            }
        }
    }
    d.set_title(t("title"));
    set_html("lang", if ko() { "English" } else { "한국어" });
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn on<F: FnMut(web_sys::Event) + 'static>(id: &str, ev: &str, f: F) {
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(f);
    if let Some(el) = doc().get_element_by_id(id) {
        let _ = el.add_event_listener_with_callback(ev, cb.as_ref().unchecked_ref());
    }
    cb.forget();
}

#[wasm_bindgen(start)]
pub fn start() {
    let saved = storage().and_then(|s| s.get_item("bp_lang").ok().flatten());
    let lang = saved.unwrap_or_else(|| web_sys::window().unwrap().navigator().language().unwrap_or_default());
    KO.with(|k| k.set(lang.starts_with("ko")));

    apply_chem();
    apply_preset();
    apply_lang();

    on("chem", "change", |_| {
        apply_chem();
        recompute();
    });
    on("preset", "change", |_| {
        apply_preset();
        recompute();
    });
    on("app", "input", |_| recompute());
    on("lang", "click", |_| {
        KO.with(|k| k.set(!k.get()));
        if let Some(s) = storage() {
            let _ = s.set_item("bp_lang", if ko() { "ko" } else { "en" });
        }
        apply_lang();
        recompute();
    });
    on("sugg_out", "click", |e| {
        let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        if let (Some(s), Some(p)) = (el.get_attribute("data-s"), el.get_attribute("data-p")) {
            input("s").set_value(&s);
            input("p").set_value(&p);
            recompute();
        }
    });
    let cb = Closure::<dyn FnMut()>::new(recompute);
    let _ = web_sys::window().unwrap().add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref());
    cb.forget();

    recompute();
}
