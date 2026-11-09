# batt-pack

> **추정값일 뿐입니다 — 반드시 셀 데이터시트를 따르고 제대로 된 BMS를 사용하세요.**
> **Estimates only — follow cell datasheets and use a proper BMS.**

## 한국어

### 무엇인가요
로봇용 배터리 팩 플래너입니다. "셀 몇 개를, 몇 S×P로 묶으면, 로봇이 얼마나 달릴까?"에 답하는
단일 페이지 정적 웹 앱입니다 (Rust → WebAssembly). 모든 계산은 브라우저 안에서만 이루어지며,
서버·분석·트래커·외부 요청이 전혀 없습니다. 언어 설정만 선택적으로 localStorage에 저장됩니다.

### 기능
1. **셀 + 팩**: 화학 조성 프리셋(Li-ion NMC 3.6/4.2/3.0 V, LiFePO4 3.2/3.65/2.5 V, LiPo 3.7/4.2/3.3 V),
   셀 프리셋 "일반 18650" / "일반 21700" (typical 값, 모두 수정 가능), S·P, 팩 오버헤드 무게 %(기본 15 %).
   팩 전압(공칭/만충/방전), Ah, Wh, 무게, Wh/kg, 최대 연속 전류, 팩 저항을 계산합니다.
2. **부하 + 런타임**: 평균/피크 전력, 사용 DoD(기본 80 %), 컨버터 효율(기본 0.95).
   런타임, 평균/피크 전류, 피크 전압 강하, DoD 끝 부하 전압, I²R 발열, 피크 C-rate, 판정(✅/⚠️/❌).
3. **S×P 추천**: 목표 공칭 전압 범위와 목표 런타임을 만족하는 가장 가벼운 조합 최대 5개 (S 1–24, P 1–10).
4. **셀 배열 그림**: canvas로 S×P 셀 배열을 그립니다 (직렬 그룹마다 S1, S2 … 라벨).
5. 한국어 / English 전환 (브라우저 언어 기본), 모바일 대응, 다크 모드는 `prefers-color-scheme`.

### 계산식과 가정
- 팩 전압 = S × 셀 전압, 용량 Ah = P × 셀 Ah, 에너지 Wh = 팩 공칭 V × Ah
- 무게 = S × P × 셀 무게 × (1 + 오버헤드 %/100)
- 최대 연속 전류 = C × 팩 Ah, 팩 저항 R = S × R셀 / P
- 런타임 h = Wh × DoD × 효율 / 평균 W
- 배터리 측 전류 I = P / (팩 공칭 V × 효율) — 평균·피크 전류, 전압 강하, 발열, C-rate, 추천 모두 동일
- 전압 강하 = I피크 × R, 발열 = I피크² × R, C-rate = I피크 / 팩 Ah
- DoD 끝 부하 전압 = (만충 V − DoD × (만충 V − 방전 V)) − 전압 강하
- 판정: ❌ I피크 > 최대 연속 전류 / ⚠️ I피크 ≥ 한계의 90 % 또는 DoD 끝 부하 전압 < 방전 종지 전압 / ✅ 그 외
- 가정: 에너지·전류는 **공칭 전압 일정**으로 계산, **내부 저항은 선형**(전류에 무관한 상수),
  개방 전압은 DoD 끝 판정에만 만충↔방전 사이 직선으로 근사, **온도·노화는 고려하지 않음**.

### 사용법
값을 입력하면 즉시 다시 계산됩니다. 추천 목록의 "적용" 버튼으로 해당 S×P를 바로 적용할 수 있습니다.

### 로컬 빌드
```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.22.1 --locked
cargo test -p bp_core
trunk build --release   # 결과물: dist/ (GitHub Pages 경로 /batt-pack/)
sh tools/privacy_check.sh
```
`trunk serve`로 로컬 확인 시 `http://localhost:8080/batt-pack/` 로 접속하세요.

### 라이선스
MIT — `LICENSE` 참고 (Copyright batt-pack contributors).

---

## English

### What it is
A robot battery pack planner: a single-page static web app (Rust → WebAssembly) that answers
"how many cells, in what S×P, and how long will my robot run?". Everything runs in your browser —
no server, analytics, trackers or network requests. Only the language choice is optionally kept in localStorage.

### Features
1. **Cell + pack**: chemistry presets (Li-ion NMC 3.6/4.2/3.0 V, LiFePO4 3.2/3.65/2.5 V, LiPo 3.7/4.2/3.3 V),
   cell presets "generic 18650" / "generic 21700" (typical values, all editable), S and P, pack overhead mass %
   (default 15 %). Outputs pack nominal/full/empty voltage, Ah, Wh, mass, Wh/kg, max continuous current, pack resistance.
2. **Load + runtime**: average/peak power, usable DoD (default 80 %), converter efficiency (default 0.95).
   Outputs runtime, average/peak current, sag at peak, loaded voltage at end of DoD, I²R heat, C-rate at peak, verdict.
3. **Suggest S×P**: up to 5 lightest combos (S 1–24, P 1–10) meeting a nominal voltage window and target runtime.
4. **Cell grid**: canvas drawing of the S×P layout with series groups labelled S1, S2, …
5. Korean / English toggle (browser default), mobile friendly, dark mode via `prefers-color-scheme`.

### Math and assumptions
- Pack V = S × cell V, Ah = P × cell Ah, Wh = pack nominal V × Ah
- Mass = S × P × cell mass × (1 + overhead %/100)
- Max continuous current = C × pack Ah, pack resistance R = S × R_cell / P
- Runtime h = Wh × DoD × efficiency / average W
- Battery-side current I = P / (pack nominal V × efficiency), used everywhere (currents, sag, heat, C-rate, search)
- Sag = I_peak × R, heat = I_peak² × R, C-rate = I_peak / pack Ah
- Loaded voltage at end of DoD = (V_full − DoD × (V_full − V_empty)) − sag
- Verdict: ❌ I_peak > max continuous current / ⚠️ I_peak ≥ 90 % of the limit or loaded end-of-DoD voltage
  below the empty cutoff / ✅ otherwise
- Assumptions: **constant nominal voltage** for energy and current, **linear internal resistance**
  (constant, independent of current), open-circuit voltage approximated as a straight line between full and
  empty only for the end-of-DoD check, **no temperature or aging effects**.

### How to use
Edit any value and results update immediately. Press "Use" in the suggestion list to apply that S×P.

### Local build
```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.22.1 --locked
cargo test -p bp_core
trunk build --release   # output: dist/ (served under /batt-pack/ on GitHub Pages)
sh tools/privacy_check.sh
```
With `trunk serve`, open `http://localhost:8080/batt-pack/`.

### License
MIT — see `LICENSE` (Copyright batt-pack contributors).
