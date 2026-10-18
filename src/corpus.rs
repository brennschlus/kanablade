//! Phrases and their furigana-annotated parsing.
//!
//! Format, one phrase per line: `猿[さる]も木[き]から落[お]ちる | перевод`.
//! A reading in brackets applies to the run of kanji right before it.

use crate::kana;
use std::ops::Range;

pub const BUILTIN: &str = "\
古池[ふるいけ]や蛙[かわず]飛[と]び込[こ]む水[みず]の音[おと] | Старый пруд. Прыгнула лягушка — всплеск воды. (Басё)
猿[さる]も木[き]から落[お]ちる | И обезьяны падают с деревьев.
七転[ななころ]び八起[やお]き | Семь раз упасть, восемь раз подняться.
一期一会[いちごいちえ] | Одна жизнь — одна встреча.
花[はな]より団子[だんご] | Клёцки важнее цветов.
石[いし]の上[うえ]にも三年[さんねん] | И на камне за три года нагреешь место.
雨[あめ]降[ふ]って地[じ]固[かた]まる | После дождя земля твердеет.
急[いそ]がば回[まわ]れ | Спешишь — иди в обход.
塵[ちり]も積[つ]もれば山[やま]となる | И пылинки, копясь, становятся горой.
千里[せんり]の道[みち]も一歩[いっぽ]から | Путь в тысячу ри начинается с одного шага.
静[しず]かな夜[よる]に月[つき]が昇[のぼ]る | Тихой ночью восходит луна.
桜[さくら]の花[はな]が風[かぜ]に舞[ま]う | Лепестки сакуры кружатся на ветру.
今日[きょう]もいい天気[てんき]ですね | Сегодня снова хорошая погода, правда?
ありがとうございます | Большое спасибо.
お茶[ちゃ]を一杯[いっぱい]どうぞ | Прошу, чашечку чая.
閑[しずか]さや岩[いわ]にしみ入[い]る蝉[せみ]の声[こえ] | Тишина. В скалы впитывается стрёкот цикад. (Басё)
柿[かき]食[く]えば鐘[かね]が鳴[な]るなり法隆寺[ほうりゅうじ] | Ем хурму — и звонит колокол Хорю-дзи. (Сики)
雪[ゆき]が静[しず]かに降[ふ]っている | Тихо падает снег.
ゆく河[かわ]の流[なが]れは絶[た]えずして | Течение уходящей реки не прерывается. (Тёмэй)
心[こころ]を込[こ]めて | Вкладывая душу.
一歩[いっぽ]ずつ進[すす]もう | Будем идти шаг за шагом.
猫[ねこ]が日向[ひなた]で眠[ねむ]っている | Кошка дремлет на солнышке.
コーヒーを飲[の]みましょう | Давай выпьем кофе.
ラーメンが大好[だいす]きです | Обожаю рамен.
明日[あした]は晴[は]れるでしょう | Завтра, наверное, будет ясно.
時間[じかん]がゆっくり流[なが]れる | Время течёт медленно.
海[うみ]の向[む]こうに島[しま]が見[み]える | За морем виднеется остров.
夢[ゆめ]を追[お]いかけて | Следуя за мечтой.
風[かぜ]の音[おと]を聞[き]きながら | Слушая шум ветра.
本[ほん]を読[よ]むのが好[す]きです | Люблю читать книги.
少[すこ]しずつ上手[じょうず]になる | Понемногу становлюсь лучше.
窓[まど]の外[そと]は雨[あめ]です | За окном дождь.
春[はる]はあけぼの | Весной — рассвет. (Сэй-сёнагон)
森[もり]の中[なか]で深呼吸[しんこきゅう] | Глубокий вдох посреди леса.
一緒[いっしょ]に行[い]きませんか | Не пойти ли нам вместе?
ゆっくりでいいよ | Можно не спешить.
出[で]る杭[くい]は打[う]たれる | Торчащий кол забивают.
知[し]らぬが仏[ほとけ] | Не знаешь — и спокоен, как Будда.
笑[わら]う門[かど]には福[ふく]来[きた]る | В дом, где смеются, приходит счастье.
案[あん]ずるより産[う]むが易[やす]し | Сделать легче, чем тревожиться.
蛍[ほたる]の光[ひかり]が川辺[かわべ]に揺[ゆ]れる | Свет светлячков дрожит у реки.
秋[あき]の夕暮[ゆうぐ]れ | Осенние сумерки.
お疲[つか]れ様[さま]でした | Спасибо за труды.
小[ちい]さな幸[しあわ]せを見[み]つける | Находить маленькое счастье.
新[あたら]しい朝[あさ]が来[き]た | Пришло новое утро.
雨[あめ]ニモマケズ、風[かぜ]ニモマケズ | Не сдаваясь дождю, не сдаваясь ветру. (Миядзава Кэндзи)
春眠[しゅんみん]暁[あかつき]を覚[おぼ]えず | Весенний сон не замечает рассвета.
我[われ]思[おも]う、故[ゆえ]に我[われ]あり | Мыслю, следовательно, существую.
東京[とうきょう]タワーが見[み]える | Видна Токийская башня.
山[やま]の上[うえ]から町[まち]を眺[なが]める | Смотрю на город с вершины горы.
";

#[derive(Debug, Clone)]
pub struct Glyph {
    pub ch: char,
    /// Units that must be completed for this glyph to light up: `start..end`.
    /// Empty for punctuation (lit as soon as the cursor passes `start`).
    pub units: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct Ruby {
    pub glyphs: Range<usize>,
    pub chars: Vec<Glyph>,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub glyphs: Vec<Glyph>,
    pub rubies: Vec<Ruby>,
    pub units: Vec<String>,
    pub translation: String,
}

fn is_kanji(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '々' | '〆' | 'ヶ' | 'ヵ')
}

enum Piece {
    Kana(char),
    Other(char),
    Ruby { base: Vec<char>, reading: Vec<char> },
}

pub fn parse_line(src: &str) -> Option<Line> {
    let src = src.trim();
    if src.is_empty() || src.starts_with('#') {
        return None;
    }
    let (text, translation) = match src.split_once('|') {
        Some((a, b)) => (a.trim(), b.trim().to_string()),
        None => (src, String::new()),
    };

    let mut pieces: Vec<Piece> = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' || c == '［' {
            let reading: Vec<char> = chars.by_ref().take_while(|&c| c != ']' && c != '］').collect();
            let mut base = Vec::new();
            while let Some(Piece::Other(k)) = pieces.last() {
                if !is_kanji(*k) {
                    break;
                }
                base.insert(0, *k);
                pieces.pop();
            }
            if base.is_empty() {
                // A reading with nothing to attach to: type it as plain kana.
                pieces.extend(reading.into_iter().map(Piece::Kana));
            } else {
                pieces.push(Piece::Ruby { base, reading });
            }
        } else if c.is_whitespace() {
            continue;
        } else if kana::is_kana(c) {
            pieces.push(Piece::Kana(c));
        } else {
            pieces.push(Piece::Other(c));
        }
    }

    let mut line = Line { glyphs: vec![], rubies: vec![], units: vec![], translation };
    let mut i = 0;
    while i < pieces.len() {
        match &pieces[i] {
            Piece::Kana(_) => {
                let start = i;
                while matches!(pieces.get(i), Some(Piece::Kana(_))) {
                    i += 1;
                }
                let run: Vec<char> = pieces[start..i]
                    .iter()
                    .map(|p| if let Piece::Kana(c) = p { *c } else { unreachable!() })
                    .collect();
                let mut glyphs: Vec<Glyph> = run.iter().map(|&ch| Glyph { ch, units: 0..0 }).collect();
                for (unit, range) in kana::split_units(&run) {
                    let u = line.units.len();
                    line.units.push(unit);
                    for g in &mut glyphs[range] {
                        g.units = u..u + 1;
                    }
                }
                line.glyphs.extend(glyphs);
                continue;
            }
            Piece::Other(c) => {
                let n = line.units.len();
                line.glyphs.push(Glyph { ch: *c, units: n..n });
            }
            Piece::Ruby { base, reading } => {
                let first_unit = line.units.len();
                let mut chars: Vec<Glyph> = reading.iter().map(|&ch| Glyph { ch, units: 0..0 }).collect();
                for (unit, range) in kana::split_units(reading) {
                    let u = line.units.len();
                    line.units.push(unit);
                    for g in &mut chars[range] {
                        g.units = u..u + 1;
                    }
                }
                let units = first_unit..line.units.len();
                let g0 = line.glyphs.len();
                line.glyphs.extend(base.iter().map(|&ch| Glyph { ch, units: units.clone() }));
                line.rubies.push(Ruby { glyphs: g0..line.glyphs.len(), chars });
            }
        }
        i += 1;
    }

    (!line.units.is_empty()).then_some(line)
}

pub fn parse(src: &str) -> Vec<Line> {
    src.lines().filter_map(parse_line).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kana::Matcher;

    #[test]
    fn builtin_is_fully_typeable() {
        let lines = parse(BUILTIN);
        assert!(lines.len() > 40);
        for line in &lines {
            let mut m = Matcher::new(line.units.clone());
            let hint = m.current_hint().unwrap_or_else(|| panic!("no hint in {:?}", line.units));
            let full = hint.romaji + &m.preview_after_current().concat();
            for c in full.chars() {
                assert!(m.input(c).accepted, "{:?} rejected {c} in {full}", line.units);
            }
            assert!(m.is_done(), "{:?} not done after {full}", line.units);
        }
    }

    #[test]
    fn ruby_mapping() {
        let l = parse_line("今日[きょう]は晴[は]れ、 | x").unwrap();
        assert_eq!(l.units, vec!["きょ", "う", "は", "は", "れ"]);
        assert_eq!(l.glyphs.len(), 6);
        assert_eq!(l.glyphs[0].units, 0..2);
        assert_eq!(l.glyphs[1].units, 0..2);
        assert_eq!(l.glyphs[2].units, 2..3);
        assert_eq!(l.glyphs[5].units, 5..5);
        assert_eq!(l.rubies[0].chars[1].units, 0..1);
        assert_eq!(l.rubies[0].chars[2].units, 1..2);
    }
}
