//! Romaji → kana matching engine.
//!
//! A line is split into typing *units* (one kana, or a kana + small ゃゅょ digraph).
//! The matcher accepts every common IME spelling (shi/si, tsu/tu, っ via a doubled
//! consonant, ん as n/nn/n'/xn) and reports which units were completed by each key.

use std::ops::Range;

/// Preferred spelling first.
#[rustfmt::skip]
const TABLE: &[(&str, &[&str])] = &[
    ("あ", &["a"]), ("い", &["i", "yi"]), ("う", &["u", "wu", "whu"]), ("え", &["e"]), ("お", &["o"]),
    ("か", &["ka", "ca"]), ("き", &["ki"]), ("く", &["ku", "cu", "qu"]), ("け", &["ke"]), ("こ", &["ko", "co"]),
    ("さ", &["sa"]), ("し", &["shi", "si", "ci"]), ("す", &["su"]), ("せ", &["se", "ce"]), ("そ", &["so"]),
    ("た", &["ta"]), ("ち", &["chi", "ti"]), ("つ", &["tsu", "tu"]), ("て", &["te"]), ("と", &["to"]),
    ("な", &["na"]), ("に", &["ni"]), ("ぬ", &["nu"]), ("ね", &["ne"]), ("の", &["no"]),
    ("は", &["ha"]), ("ひ", &["hi"]), ("ふ", &["fu", "hu"]), ("へ", &["he"]), ("ほ", &["ho"]),
    ("ま", &["ma"]), ("み", &["mi"]), ("む", &["mu"]), ("め", &["me"]), ("も", &["mo"]),
    ("や", &["ya"]), ("ゆ", &["yu"]), ("よ", &["yo"]),
    ("ら", &["ra"]), ("り", &["ri"]), ("る", &["ru"]), ("れ", &["re"]), ("ろ", &["ro"]),
    ("わ", &["wa"]), ("ゐ", &["wi"]), ("ゑ", &["we"]), ("を", &["wo"]),
    ("が", &["ga"]), ("ぎ", &["gi"]), ("ぐ", &["gu"]), ("げ", &["ge"]), ("ご", &["go"]),
    ("ざ", &["za"]), ("じ", &["ji", "zi"]), ("ず", &["zu"]), ("ぜ", &["ze"]), ("ぞ", &["zo"]),
    ("だ", &["da"]), ("ぢ", &["di"]), ("づ", &["du", "dzu"]), ("で", &["de"]), ("ど", &["do"]),
    ("ば", &["ba"]), ("び", &["bi"]), ("ぶ", &["bu"]), ("べ", &["be"]), ("ぼ", &["bo"]),
    ("ぱ", &["pa"]), ("ぴ", &["pi"]), ("ぷ", &["pu"]), ("ぺ", &["pe"]), ("ぽ", &["po"]),
    ("ゔ", &["vu"]),
    ("ぁ", &["xa", "la"]), ("ぃ", &["xi", "li"]), ("ぅ", &["xu", "lu"]), ("ぇ", &["xe", "le"]), ("ぉ", &["xo", "lo"]),
    ("ゃ", &["xya", "lya"]), ("ゅ", &["xyu", "lyu"]), ("ょ", &["xyo", "lyo"]), ("ゎ", &["xwa", "lwa"]),
    ("ゕ", &["xka", "lka"]), ("ゖ", &["xke", "lke"]),
    ("っ", &["xtu", "ltu", "xtsu", "ltsu"]),
    ("ー", &["-"]),
    ("きゃ", &["kya"]), ("きゅ", &["kyu"]), ("きょ", &["kyo"]), ("きぇ", &["kye"]),
    ("しゃ", &["sha", "sya"]), ("しゅ", &["shu", "syu"]), ("しょ", &["sho", "syo"]), ("しぇ", &["she", "sye"]),
    ("ちゃ", &["cha", "tya", "cya"]), ("ちゅ", &["chu", "tyu", "cyu"]), ("ちょ", &["cho", "tyo", "cyo"]), ("ちぇ", &["che", "tye", "cye"]),
    ("にゃ", &["nya"]), ("にゅ", &["nyu"]), ("にょ", &["nyo"]),
    ("ひゃ", &["hya"]), ("ひゅ", &["hyu"]), ("ひょ", &["hyo"]),
    ("みゃ", &["mya"]), ("みゅ", &["myu"]), ("みょ", &["myo"]),
    ("りゃ", &["rya"]), ("りゅ", &["ryu"]), ("りょ", &["ryo"]),
    ("ぎゃ", &["gya"]), ("ぎゅ", &["gyu"]), ("ぎょ", &["gyo"]),
    ("じゃ", &["ja", "zya", "jya"]), ("じゅ", &["ju", "zyu", "jyu"]), ("じょ", &["jo", "zyo", "jyo"]), ("じぇ", &["je", "zye", "jye"]),
    ("ぢゃ", &["dya"]), ("ぢゅ", &["dyu"]), ("ぢょ", &["dyo"]),
    ("びゃ", &["bya"]), ("びゅ", &["byu"]), ("びょ", &["byo"]),
    ("ぴゃ", &["pya"]), ("ぴゅ", &["pyu"]), ("ぴょ", &["pyo"]),
    ("ふぁ", &["fa"]), ("ふぃ", &["fi"]), ("ふぇ", &["fe"]), ("ふぉ", &["fo"]), ("ふゅ", &["fyu"]),
    ("てぃ", &["thi"]), ("でぃ", &["dhi"]), ("でゅ", &["dhu"]), ("とぅ", &["twu"]), ("どぅ", &["dwu"]),
    ("うぃ", &["wi"]), ("うぇ", &["we"]), ("うぉ", &["who"]),
    ("ゔぁ", &["va"]), ("ゔぃ", &["vi"]), ("ゔぇ", &["ve"]), ("ゔぉ", &["vo"]),
    ("つぁ", &["tsa"]),
];

fn lookup(kana: &str) -> Option<&'static [&'static str]> {
    TABLE.iter().find(|(k, _)| *k == kana).map(|(_, v)| *v)
}

/// Katakana → hiragana; everything else unchanged.
pub fn to_hiragana(c: char) -> char {
    match c {
        '\u{30A1}'..='\u{30F6}' => char::from_u32(c as u32 - 0x60).unwrap_or(c),
        _ => c,
    }
}

pub fn is_kana(c: char) -> bool {
    let h = to_hiragana(c);
    h == 'ー' || h == 'ん' || lookup(h.encode_utf8(&mut [0; 4])).is_some()
}

pub fn is_small(c: char) -> bool {
    matches!(to_hiragana(c), 'ぁ' | 'ぃ' | 'ぅ' | 'ぇ' | 'ぉ' | 'ゃ' | 'ゅ' | 'ょ' | 'ゎ')
}

/// Splits a run of kana into units. Returns (hiragana unit, char range in the input).
pub fn split_units(chars: &[char]) -> Vec<(String, Range<usize>)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let a = to_hiragana(chars[i]);
        if i + 1 < chars.len() && is_small(chars[i + 1]) {
            let pair: String = [a, to_hiragana(chars[i + 1])].iter().collect();
            if lookup(&pair).is_some() {
                out.push((pair, i..i + 2));
                i += 2;
                continue;
            }
        }
        out.push((a.to_string(), i..i + 1));
        i += 1;
    }
    out
}

/// All spellings of a single unit (not counting context-dependent っ/ん forms).
fn unit_spellings(unit: &str) -> Vec<String> {
    if unit == "ん" {
        return vec!["nn".into(), "n'".into(), "xn".into()];
    }
    let mut v: Vec<String> = lookup(unit).map(|s| s.iter().map(|s| s.to_string()).collect()).unwrap_or_default();
    // Digraphs can also be typed as their two halves: ki + xya.
    let chars: Vec<char> = unit.chars().collect();
    if chars.len() == 2 {
        let (a, b) = (lookup(&chars[0].to_string()), lookup(&chars[1].to_string()));
        if let (Some(a), Some(b)) = (a, b) {
            for x in a {
                for y in b {
                    let s = format!("{x}{y}");
                    if !v.contains(&s) {
                        v.push(s);
                    }
                }
            }
        }
    }
    v
}

/// The canonical romaji of a single unit (used for musical mapping).
pub fn preferred(unit: &str) -> &'static str {
    // Every unit split_units produces is in the table, so the digraph fallback in unit_spellings never applies.
    match unit {
        "ん" => "nn",
        _ => lookup(unit).and_then(|s| s.first().copied()).unwrap_or(""),
    }
}

fn starts_with_vowelish(s: &str) -> bool {
    matches!(s.as_bytes().first(), Some(b'a' | b'i' | b'u' | b'e' | b'o' | b'y' | b'n' | b'\'' | b'-'))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub romaji: String,
    pub units: usize,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Outcome {
    pub accepted: bool,
    /// Unit ranges completed by this key, in order.
    pub completed: Vec<Range<usize>>,
    /// The line ended on a ん typed as a single "n" (a following "n" is harmless).
    pub lone_n_at_end: bool,
}

#[derive(Debug, Clone)]
pub struct Matcher {
    units: Vec<String>,
    pos: usize,
    buf: String,
    typed: Vec<String>,
}

impl Matcher {
    pub fn new(units: Vec<String>) -> Self {
        Self { units, pos: 0, buf: String::new(), typed: Vec::new() }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn is_done(&self) -> bool {
        self.pos >= self.units.len()
    }

    pub fn units(&self) -> &[String] {
        &self.units
    }

    pub fn buffer(&self) -> &str {
        &self.buf
    }

    /// Spellings for the chunk starting at `pos`, preferred first.
    pub fn candidates(&self, pos: usize) -> Vec<Candidate> {
        let Some(unit) = self.units.get(pos) else { return vec![] };
        let next = self.units.get(pos + 1);
        let mut out = Vec::new();
        match unit.as_str() {
            "っ" => {
                if let Some(next) = next.filter(|n| *n != "っ" && *n != "ん") {
                    for s in unit_spellings(next) {
                        if !starts_with_vowelish(&s) {
                            out.push(Candidate { romaji: format!("{}{s}", &s[..1]), units: 2 });
                            if s.starts_with("ch") {
                                out.push(Candidate { romaji: format!("t{s}"), units: 2 });
                            }
                        }
                    }
                }
            }
            "ん" => {
                let lone_ok = match next {
                    None => true,
                    Some(n) => {
                        let sp = unit_spellings(n);
                        n != "っ" && !sp.is_empty() && sp.iter().all(|s| !starts_with_vowelish(s))
                    }
                };
                if lone_ok {
                    out.push(Candidate { romaji: "n".into(), units: 1 });
                }
            }
            _ => {}
        }
        for s in unit_spellings(unit) {
            out.push(Candidate { romaji: s, units: 1 });
        }
        out
    }

    /// The spelling we would show for the chunk at `pos` given what's typed so far.
    pub fn current_hint(&self) -> Option<Candidate> {
        self.candidates(self.pos).into_iter().find(|c| c.romaji.starts_with(&self.buf))
    }

    /// Romaji chunks already committed in this line.
    pub fn typed(&self) -> &[String] {
        &self.typed
    }

    /// Preferred romaji chunks for everything after the current chunk.
    pub fn preview_after_current(&self) -> Vec<String> {
        let mut pos = self.pos + self.current_hint().map_or(1, |c| c.units);
        let mut out = Vec::new();
        while pos < self.units.len() {
            match self.candidates(pos).into_iter().next() {
                Some(c) => {
                    out.push(c.romaji);
                    pos += c.units;
                }
                None => pos += 1,
            }
        }
        out
    }

    fn commit(&mut self, units: usize, out: &mut Outcome) {
        let start = self.pos;
        self.pos = (self.pos + units).min(self.units.len());
        self.typed.push(std::mem::take(&mut self.buf));
        out.completed.push(start..self.pos);
    }

    pub fn input(&mut self, c: char) -> Outcome {
        let mut out = Outcome::default();
        self.input_inner(c, &mut out, true);
        out
    }

    fn input_inner(&mut self, c: char, out: &mut Outcome, may_resolve: bool) {
        if self.is_done() {
            return;
        }
        let cands = self.candidates(self.pos);
        let mut nb = self.buf.clone();
        nb.push(c);
        let matching: Vec<&Candidate> = cands.iter().filter(|k| k.romaji.starts_with(&nb)).collect();

        if !matching.is_empty() {
            out.accepted = true;
            self.buf = nb;
            let exact = matching.iter().find(|k| k.romaji == self.buf).map(|k| k.units);
            let longer = matching.iter().any(|k| k.romaji.len() > self.buf.len());
            if let Some(units) = exact {
                let at_end = self.pos + units >= self.units.len();
                if !longer || at_end {
                    out.lone_n_at_end = at_end && self.buf == "n";
                    self.commit(units, out);
                }
            }
            return;
        }

        // The buffer may already be a complete (but ambiguous) spelling, e.g. "n" before "k".
        if may_resolve && let Some(units) = cands.iter().find(|k| k.romaji == self.buf).map(|k| k.units) {
            self.commit(units, out);
            self.input_inner(c, out, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher(kana: &str) -> Matcher {
        let chars: Vec<char> = kana.chars().collect();
        Matcher::new(split_units(&chars).into_iter().map(|(u, _)| u).collect())
    }

    fn type_all(m: &mut Matcher, s: &str) -> bool {
        s.chars().all(|c| m.input(c).accepted)
    }

    fn accepts(kana: &str, romaji: &str) -> bool {
        let mut m = matcher(kana);
        type_all(&mut m, romaji) && m.is_done()
    }

    #[test]
    fn basic() {
        assert!(accepts("すし", "sushi"));
        assert!(accepts("すし", "susi"));
        assert!(accepts("つち", "tuti"));
        assert!(accepts("カタカナ", "katakana"));
        assert!(!accepts("すし", "susa"));
    }

    #[test]
    fn digraphs() {
        assert!(accepts("きょう", "kyou"));
        assert!(accepts("きょう", "kixyou"));
        assert!(accepts("ちゃ", "cha"));
        assert!(accepts("ちゃ", "tya"));
        assert!(accepts("じゃ", "ja"));
        assert!(accepts("じゃ", "zya"));
    }

    #[test]
    fn sokuon() {
        assert!(accepts("いっぽ", "ippo"));
        assert!(accepts("いっぽ", "ixtupo"));
        assert!(accepts("まっちゃ", "maccha"));
        assert!(accepts("まっちゃ", "matcha"));
        assert!(accepts("まっちゃ", "mattya"));
        assert!(!accepts("いっぽ", "iipo"));
    }

    #[test]
    fn n_handling() {
        assert!(accepts("さんねん", "sannnen"));
        assert!(accepts("かんじ", "kanji"));
        assert!(accepts("かんじ", "kannji"));
        assert!(accepts("きんえん", "kin'en"));
        assert!(accepts("きんえん", "kinnen"));
        assert!(!accepts("きんえん", "kinen"));
        assert!(accepts("ほん", "hon"));
        let mut m = matcher("ほん");
        assert!(type_all(&mut m, "ho"));
        let o = m.input('n');
        assert!(o.lone_n_at_end && m.is_done());
    }

    #[test]
    fn long_vowel_and_completion_ranges() {
        assert!(accepts("ラーメン", "ra-men"));
        let mut m = matcher("かんき");
        assert!(m.input('k').completed.is_empty());
        assert_eq!(m.input('a').completed, vec![0..1]);
        assert!(m.input('n').completed.is_empty());
        assert_eq!(m.input('k').completed, vec![1..2]);
        assert_eq!(m.input('i').completed, vec![2..3]);
    }

    #[test]
    fn preferred_matches_first_spelling() {
        for unit in TABLE.iter().map(|(k, _)| *k).chain(["ん"]) {
            assert_eq!(preferred(unit), unit_spellings(unit)[0], "{unit}");
        }
    }

    #[test]
    fn hints() {
        let mut m = matcher("しゃしん");
        assert_eq!(m.current_hint().unwrap().romaji, "sha");
        m.input('s');
        m.input('y');
        assert_eq!(m.current_hint().unwrap().romaji, "sya");
        assert_eq!(m.preview_after_current(), vec!["shi", "n"]);
    }
}
