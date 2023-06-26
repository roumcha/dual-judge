#[allow(unused_imports)]
use anyhow::{anyhow, bail, ensure, Context, Result};
use std::{cmp::Ordering, fmt::Display, fs, path::Path};

use regex::Regex;

use crate::{
    comma_sep_int,
    config::Config,
    submission_state::SubmissionStateSingle,
    submission_state::{self, SubmissionState, SubmissionStateSingle::*},
};

pub type SubmissionId = usize;
pub type Time = f64;
pub type Score = f64;
pub type Rate = f64;

#[derive(Clone, Debug)]
pub struct CaseSummary {
    pub name: String,
    pub state: SubmissionState,
    pub time: Time,
    pub score: Score,
    pub rate: Rate,
}

impl Display for CaseSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state_str = SubmissionStateSingle::try_from(self.state)
            .and_then(|single| core::result::Result::Ok(single.to_string()))
            .unwrap_or("???".into());

        let score_comma = comma_sep_int(self.score.round() as i128);

        write!(
            f,
            "{:^10}| {:3} | {:>5.0} ms | {:>18} pt | {:>6.2} %",
            self.name,
            state_str,
            self.time * 1000.,
            score_comma,
            self.rate * 100.
        )
    }
}

impl CaseSummary {
    pub fn zero(name: &str, state: SubmissionState) -> Self {
        Self {
            name: name.into(),
            state,
            time: 0.,
            score: 0.,
            rate: 0.,
        }
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self {
            name: self.name.clone(),
            state: self.state | other.state,
            time: self.time.max(other.time),
            score: self.score.max(other.score),
            rate: self.rate.max(other.rate),
        }
    }

    pub fn parse_file(name: &str, path: &Path, config: &Config) -> Result<Self> {
        let text = fs::read_to_string(path)?;
        Ok(Self {
            name: name.into(),
            state: submission_state::parse_state(&text, config),
            time: parse_time(config, &text).unwrap_or(0.),
            score: parse_score(config, &text).unwrap_or(0.),
            rate: parse_rate(config, &text).unwrap_or(0.),
        })
    }
}

fn parse_time(config: &Config, text: &String) -> Result<Time> {
    Ok(Regex::new(&config.parse_result.time_regex)?
        .captures(text)
        .context("正規表現が実行時間にマッチしません")?
        .iter()
        .skip(1)
        .filter_map(|m| m.and_then(|s| s.as_str().parse::<f64>().ok()))
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less))
        .unwrap_or(0.)
        * config.parse_result.time_multiplier)
}

fn parse_score(config: &Config, text: &String) -> Result<Score> {
    Ok(Regex::new(&config.parse_result.score_regex)?
        .captures(text)
        .context("正規表現がスコアにマッチしません")?
        .iter()
        .skip(1)
        .filter_map(|m| m.and_then(|s| s.as_str().parse::<f64>().ok()))
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less))
        .unwrap_or(0.)
        * config.parse_result.score_multiplier)
}

fn parse_rate(config: &Config, text: &String) -> Result<Rate> {
    Ok(Regex::new(&config.parse_result.rate_regex)?
        .captures(text)
        .context("正規表現が割合にマッチしません")?
        .iter()
        .skip(1)
        .filter_map(|m| m.and_then(|s| s.as_str().parse::<f64>().ok()))
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less))
        .unwrap_or(0.)
        * config.parse_result.rate_multiplier)
}

#[derive(Clone, Debug)]
pub struct FinalSummary {
    pub subm_id: u32,
    pub state: SubmissionState,
    pub time: Time,
    pub scores: Vec<Score>,
    pub rates: Vec<Rate>,
    pub count: usize,
    pub ac_count: usize,
}

impl Display for FinalSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state_str: String = SubmissionStateSingle::try_from(self.state)
            .and_then(|single| core::result::Result::Ok(single.to_string()))
            .unwrap_or("???".into());

        writeln!(f, "[提出@{}]", self.subm_id)?;
        writeln!(f, "")?;
        writeln!(f, "状態: {state_str} ({}/{})", self.ac_count, self.count)?;
        writeln!(
            f,
            "平均: {}pt, {:.2}%",
            comma_sep_int((self.scores.iter().sum::<f64>() / self.count.max(1) as f64) as i128),
            self.rates.iter().sum::<f64>() / self.count as f64 * 100.
        )?;
        writeln!(
            f,
            "中央: {}pt, {:.2}%",
            comma_sep_int(median_or_0(&self.scores) as i128),
            median_or_0(&self.rates) * 100.
        )?;
        writeln!(
            f,
            "最小: {}pt, {:.2}%",
            comma_sep_int(
                self.scores
                    .iter()
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                    .unwrap_or(&0.)
                    .to_owned() as i128
            ),
            self.rates
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                .unwrap_or(&0.)
                * 100.
        )?;
        writeln!(
            f,
            "最大: {}pt, {:.2}%",
            comma_sep_int(
                self.scores
                    .iter()
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                    .unwrap_or(&0.)
                    .to_owned() as i128
            ),
            self.rates
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                .unwrap_or(&0.)
                * 100.
        )?;
        writeln!(f, "時間: {} ms", self.time * 1000.)?;
        writeln!(f, "")
    }
}

pub fn median_or_0(src: &[f64]) -> f64 {
    let mut sorted = src.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let len = sorted.len();
    if len == 0 {
        0.
    } else if len == 1 {
        sorted[0]
    } else if len % 2 == 1 {
        sorted[len / 2]
    } else {
        (sorted[len / 2] + sorted[len / 2 - 1]) / 2.
    }
}

impl FinalSummary {
    pub fn zero(subm_id: u32) -> Self {
        Self {
            subm_id,
            state: AC as u32,
            time: 0.,
            scores: vec![],
            rates: vec![],
            count: 0,
            ac_count: 0,
        }
    }

    pub fn next_case(&self, case: &CaseSummary) -> Self {
        let mut scores = self.scores.clone();
        scores.push(case.score);

        let mut rates = self.rates.clone();
        rates.push(case.rate);

        Self {
            subm_id: self.subm_id,
            state: self.state | case.state,
            time: self.time.max(case.time),
            scores,
            rates,
            count: self.count + 1,
            ac_count: self.ac_count + if case.state == AC as u32 { 1 } else { 0 },
        }
    }
}
