use std::fmt::Display;

use regex::Regex;
use SubmissionStateSingle::*;

use crate::config::Config;

pub type SubmissionState = u32;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum SubmissionStateSingle {
    IE = 1 << 7,
    CE = 1 << 6,
    RE = 1 << 5,
    QLE = 1 << 4,
    OLE = 1 << 3,
    WA = 1 << 2,
    TLE = 1 << 1,
    MLE = 1 << 0,
    AC = 0,
}

impl SubmissionStateSingle {
    pub const ERR_ORDER: &[SubmissionStateSingle] = &[IE, CE, RE, OLE, WA, QLE, TLE, MLE];
}

impl TryFrom<SubmissionState> for SubmissionStateSingle {
    type Error = ();

    fn try_from(state: SubmissionState) -> core::result::Result<Self, Self::Error> {
        for &item in Self::ERR_ORDER {
            if state & item as SubmissionState != 0 {
                return core::result::Result::Ok(item);
            }
        }

        if state == 0 {
            return core::result::Result::Ok(AC);
        }
        Err(())
    }
}

impl Display for SubmissionStateSingle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Default for SubmissionStateSingle {
    fn default() -> Self {
        AC
    }
}

pub fn parse_state(text: &str, config: &Config) -> SubmissionState {
    let mut state = AC as u32;

    if Regex::new(&config.parse_result.force_ac_regex)
        .unwrap()
        .is_match(&text)
    {
        return AC as u32;
    }

    if Regex::new(&config.parse_result.ie_regex)
        .unwrap()
        .is_match(&text)
    {
        state |= IE as u32;
    }
    if Regex::new(&config.parse_result.ce_regex)
        .unwrap()
        .is_match(&text)
    {
        state |= CE as u32;
    }
    if Regex::new(&config.parse_result.re_regex)
        .unwrap()
        .is_match(&text)
    {
        state |= RE as u32;
    }
    if Regex::new(&config.parse_result.qle_regex)
        .unwrap()
        .is_match(&text)
    {
        state |= QLE as u32;
    }
    if Regex::new(&config.parse_result.ole_regex)
        .unwrap()
        .is_match(&text)
    {
        state |= OLE as u32;
    }
    if Regex::new(&config.parse_result.wa_regex)
        .unwrap()
        .is_match(&text)
    {
        state |= WA as u32;
    }
    if Regex::new(&config.parse_result.tle_regex)
        .unwrap()
        .is_match(&text)
    {
        state |= TLE as u32;
    }
    if Regex::new(&config.parse_result.mle_regex)
        .unwrap()
        .is_match(&text)
    {
        state |= MLE as u32;
    }

    state
}
