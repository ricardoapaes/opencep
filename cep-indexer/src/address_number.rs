use crate::normalization::normalize_text;
use anyhow::{Context, Result};

#[derive(Clone, Copy)]
enum Parity {
    Even,
    Odd,
}

impl Parity {
    fn of(number: u64) -> Self {
        if number.is_multiple_of(2) {
            Self::Even
        } else {
            Self::Odd
        }
    }

    fn matches(self, number: u64) -> bool {
        matches!((self, number % 2), (Self::Even, 0) | (Self::Odd, 1))
    }
}

pub enum NumberMatch {
    ConstrainedMatch,
    ConstrainedMiss,
    Unrestricted,
}

pub fn split_street_and_number(street: &str) -> Result<(String, Option<u64>)> {
    let Some((street_name, suffix)) = street.rsplit_once(',') else {
        return Ok((street.to_string(), None));
    };
    let suffix = suffix.trim();
    if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok((street.to_string(), None));
    }

    let number = suffix
        .parse::<u64>()
        .context("address number is too large")?;
    Ok((street_name.trim().to_string(), Some(number)))
}

pub fn matches_complement(complement: &str, number: u64) -> NumberMatch {
    let normalized = normalize_text(complement);
    let tokens = normalized.split_whitespace().collect::<Vec<_>>();
    let numbers = tokens
        .iter()
        .filter_map(|token| token.parse::<u64>().ok())
        .collect::<Vec<_>>();
    let explicit_parity = tokens.iter().find_map(|token| match *token {
        "par" => Some(Parity::Even),
        "impar" => Some(Parity::Odd),
        _ => None,
    });

    if tokens.first() == Some(&"ate") && !numbers.is_empty() {
        let upper_bound = parity_value(&numbers[..numbers.len().min(2)], Parity::of(number))
            .unwrap_or(numbers[0]);
        return constrained(number <= upper_bound);
    }

    if tokens.first() == Some(&"de") && contains_sequence(&tokens, "ao", "fim") {
        let lower_bound = explicit_parity
            .and_then(|parity| parity_value(&numbers, parity))
            .or_else(|| numbers.first().copied());
        if let Some(lower_bound) = lower_bound {
            return constrained(
                number >= lower_bound
                    && explicit_parity.is_none_or(|parity| parity.matches(number)),
            );
        }
    }

    if tokens.first() == Some(&"de") && tokens.contains(&"a") {
        if numbers.len() >= 4 {
            let parity = Parity::of(number);
            let lower_bound = parity_value(&numbers[0..2], parity).unwrap_or(numbers[0]);
            let upper_bound = parity_value(&numbers[2..4], parity).unwrap_or(numbers[3]);
            return constrained(number >= lower_bound && number <= upper_bound);
        }
        if numbers.len() >= 2 {
            return constrained(number >= numbers[0] && number <= numbers[1]);
        }
    }

    if contains_sequence(&tokens, "lado", "par") {
        return constrained(Parity::Even.matches(number));
    }
    if contains_sequence(&tokens, "lado", "impar") {
        return constrained(Parity::Odd.matches(number));
    }

    NumberMatch::Unrestricted
}

fn parity_value(numbers: &[u64], parity: Parity) -> Option<u64> {
    numbers
        .iter()
        .copied()
        .find(|number| parity.matches(*number))
}

fn contains_sequence(tokens: &[&str], first: &str, second: &str) -> bool {
    tokens.windows(2).any(|window| window == [first, second])
}

fn constrained(matches: bool) -> NumberMatch {
    if matches {
        NumberMatch::ConstrainedMatch
    } else {
        NumberMatch::ConstrainedMiss
    }
}
