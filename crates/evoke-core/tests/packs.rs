//! Every row of every built-in pack reads as what it stands for, through the readers that read a person's words:
//! each weekday, short form, relative day and month as that day; each number word as its value, and the numbers
//! a pack writes as one word as their composition; each duration unit, currency word and named time as its value;
//! each joiner as a place to cut. A row that reads as nothing, or as something else, fails here by its pack and
//! its word, so that a pack is checked whole whenever it changes.

use evoke_core::calendar::Day;
use evoke_core::manifest::Pick;
use evoke_core::pack::{self, Pack};
use evoke_core::propose::{PickValue, propose};
use evoke_core::spoken::heard;
use evoke_core::text::Input;
use evoke_core::weave::reading;

/// The candidates a text proposes, and the days its words spell in a form no recognizer reads — a short form, a
/// day misspelt — each value beside its words.
fn read(text: &str) -> Vec<(String, PickValue)> {
    let input = Input::new(text).expect("a short text reads");
    let proposed = propose(&input);
    let days = heard(&input, &Pick::Date, &[], &proposed).said;
    proposed
        .into_iter()
        .map(|p| (p.span.text().to_string(), p.value))
        .chain(
            days.into_iter()
                .map(|day| (day.span.text().to_string(), day.value)),
        )
        .collect()
}

/// A word of the pack that shows it and says nothing of a value: a function word of three letters or more that no
/// other pack lists and no cut table holds, to stand before the row under test.
fn frame(pack: &'static Pack) -> &'static str {
    let cuts: Vec<&str> = pack
        .cut
        .then
        .iter()
        .chain(pack.cut.and.iter())
        .chain(pack.cut.leads.iter())
        .chain(pack.cut.negation.iter())
        .chain(pack.cut.condition.iter())
        .chain(pack.cut.contrast.iter())
        .chain(pack.cut.but.iter())
        .chain(pack.cut.subordinators.iter())
        .collect();
    pack.words
        .function
        .iter()
        .find(|word| word.chars().count() >= 3 && pack.shows(word) && !cuts.contains(word))
        .unwrap_or_else(|| panic!("{}: no function word shows the pack", pack.tag))
}

/// Whether the text, framed, proposes a candidate the test accepts.
fn reads(pack: &'static Pack, text: &str, accepts: impl Fn(&PickValue) -> bool) -> bool {
    let framed = format!("{} {text}", frame(pack));
    let shows = pack::lexicon(&framed)
        .packs()
        .iter()
        .any(|read| read.tag == pack.tag);
    assert!(shows, "{}: «{framed}» is not read by its pack", pack.tag);
    read(&framed).iter().any(|(_, value)| accepts(value))
}

#[test]
fn every_day_word_of_every_pack_reads_as_its_day() {
    let mut wrong = Vec::new();
    for pack in pack::packs() {
        for table in [&pack.days.weekdays, &pack.days.short] {
            for (word, form) in table.words() {
                let weekday = pack::weekday(form).expect("a weekday form");
                let right = reads(
                    pack,
                    word,
                    |value| matches!(value, PickValue::Date { value: Day::Weekday { weekday: read, .. } } if *read == weekday),
                );
                if !right {
                    wrong.push(format!("{}: «{word}» is no {form}", pack.tag));
                }
            }
        }
        for (word, form) in pack
            .days
            .relative
            .words()
            .chain(pack.days.relative_short.words())
        {
            let days: i32 = form.parse().expect("an offset form");
            let right = reads(
                pack,
                word,
                |value| matches!(value, PickValue::Date { value: Day::Offset { days: read } } if *read == days),
            );
            if !right {
                wrong.push(format!("{}: «{word}» is no day at {days}", pack.tag));
            }
        }
        for (word, form) in pack.months.words() {
            let month: u8 = form.parse().expect("a month form");
            let right = reads(
                pack,
                &format!("5 {word}"),
                |value| matches!(value, PickValue::Date { value: Day::Calendar { month: read, day: 5, .. } } if *read == month),
            );
            if !right {
                wrong.push(format!(
                    "{}: «5 {word}» is no fifth of month {month}",
                    pack.tag
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "days that do not read:\n  {}",
        wrong.join("\n  ")
    );
}

#[test]
fn every_number_word_of_every_pack_reads_as_its_value() {
    let mut wrong = Vec::new();
    for pack in pack::packs() {
        for (word, value) in pack.numbers.ones.iter().chain(pack.numbers.tens.iter()) {
            #[allow(clippy::cast_precision_loss)]
            let expected = value as f64;
            let right = reads(
                pack,
                word,
                |read| matches!(read, PickValue::Number { value } if (*value - expected).abs() < f64::EPSILON),
            );
            if !right {
                wrong.push(format!("{}: «{word}» is no {value}", pack.tag));
            }
        }
        if pack.numbers.fused {
            let ones: Vec<(&str, i64)> = pack
                .numbers
                .ones
                .iter()
                .filter(|(_, value)| (1..10).contains(value))
                .collect();
            for (tens_word, tens) in pack.numbers.tens.iter() {
                for (ones_word, ones_value) in &ones {
                    let word = format!("{ones_word}{}{tens_word}", pack.numbers.joiner);
                    let expected = u64::try_from(tens + ones_value).expect("a count");
                    if pack.number(&word) != Some(expected) {
                        wrong.push(format!("{}: «{word}» is no {expected}", pack.tag));
                    }
                }
            }
            for (scale_word, scale) in pack.numbers.scale.iter() {
                for (ones_word, ones_value) in &ones {
                    let word = format!("{ones_word}{scale_word}");
                    let expected = u64::try_from(ones_value * scale).expect("a count");
                    if pack.number(&word) != Some(expected) {
                        wrong.push(format!("{}: «{word}» is no {expected}", pack.tag));
                    }
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "numbers that do not read:\n  {}",
        wrong.join("\n  ")
    );
}

#[test]
fn every_unit_currency_and_named_time_of_every_pack_reads() {
    let mut wrong = Vec::new();
    for pack in pack::packs() {
        for form in pack.durations.units.forms() {
            let seconds: u64 = form.parse().expect("a unit's seconds");
            for (n, word) in pack.durations.units.words_of(form).iter().enumerate() {
                let count = if n == 0 { 1 } else { 2 };
                let right = reads(
                    pack,
                    &format!("{count} {word}"),
                    |value| matches!(value, PickValue::Seconds { value } if *value == count * seconds),
                );
                if !right {
                    wrong.push(format!(
                        "{}: «{count} {word}» is no {} seconds",
                        pack.tag,
                        count * seconds
                    ));
                }
            }
        }
        for (word, code) in pack.amounts.words.words() {
            let right = reads(
                pack,
                &format!("1 {word}"),
                |value| matches!(value, PickValue::Amount { value } if value.currency().as_str() == code && (value.amount() - 1.0).abs() < f64::EPSILON),
            );
            if !right {
                wrong.push(format!("{}: «1 {word}» is no amount in {code}", pack.tag));
            }
        }
        for (word, form) in pack.times.named.words() {
            let right = reads(
                pack,
                word,
                |value| matches!(value, PickValue::Time { value } if format!("{:02}:{:02}", value.hour(), value.minute()) == form),
            );
            if !right {
                wrong.push(format!("{}: «{word}» is no {form}", pack.tag));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "values that do not read:\n  {}",
        wrong.join("\n  ")
    );
}

#[test]
fn every_joiner_of_every_pack_is_a_place_to_cut() {
    let mut wrong = Vec::new();
    for pack in pack::packs() {
        let frame = frame(pack);
        for (table, order) in [
            (&pack.cut.then, reading::Order::Then),
            (&pack.cut.and, reading::Order::And),
        ] {
            for phrase in table.iter() {
                let text = format!("{frame} {frame} {phrase} {frame} {frame}");
                let places = reading::places(&text, false);
                let found = places
                    .iter()
                    .any(|place| place.order == order && place.word.to_lowercase() == phrase);
                if !found {
                    wrong.push(format!("{}: «{phrase}» cuts nothing in «{text}»", pack.tag));
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "joiners that cut nothing:\n  {}",
        wrong.join("\n  ")
    );
}
