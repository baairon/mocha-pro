//! The pairing: match a predicted table against an observed table, name by name.

use crate::{
    flags::ParamFlags,
    table::{Cell, ParamDef, Table},
};

/// How a predicted row found its observed row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchClass {
    /// Names equal byte for byte.
    Exact,
    /// Names equal after `normalize_name` only.
    Normalized,
    /// No observed row has this name.
    Unmatched,
}

/// The verdict on one compared field of a matched pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldVerdict {
    /// Both known and bit-identical.
    Same,
    /// Both known and different.
    Differ,
    /// The prediction is unknown: nothing to score.
    Unscored,
    /// The prediction is known and the observation is unknown.
    ObservedUnknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamVerdict {
    pub predicted: String,
    pub observed: Option<String>,
    pub class: MatchClass,
    pub kind: Option<FieldVerdict>,
    pub default: Option<FieldVerdict>,
    pub min: Option<FieldVerdict>,
    pub max: Option<FieldVerdict>,
    pub flags: Option<FieldVerdict>,
    /// Bits set in exactly one flag set, when both are known.
    pub flag_diff: Option<ParamFlags>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pairing {
    pub verdicts: Vec<ParamVerdict>,
    /// Observed names that no predicted row claimed.
    pub unmatched_observed: Vec<String>,
}

impl Pairing {
    pub fn count(&self, class: MatchClass) -> usize {
        self.verdicts.iter().filter(|v| v.class == class).count()
    }
}

/// Fold case, trim, collapse inner whitespace runs, drop a trailing `...` or colon.
pub fn normalize_name(s: &str) -> String {
    let joined = s
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let mut t = joined.as_str();
    loop {
        let before = t;
        t = t.trim_end_matches(':').trim_end();
        t = t.trim_end_matches("...").trim_end();
        if t == before {
            break;
        }
    }
    t.to_string()
}

fn cell_verdict(p: &Cell, o: &Cell) -> FieldVerdict {
    match (p.is_unknown(), o.is_unknown()) {
        (true, _) => FieldVerdict::Unscored,
        (false, true) => FieldVerdict::ObservedUnknown,
        _ => {
            if p.bits_eq(o) {
                FieldVerdict::Same
            } else {
                FieldVerdict::Differ
            }
        }
    }
}

fn judge(p: &ParamDef, o: &ParamDef, class: MatchClass) -> ParamVerdict {
    let (flags, flag_diff) = match (p.flags, o.flags) {
        (None, _) => (FieldVerdict::Unscored, None),
        (Some(_), None) => (FieldVerdict::ObservedUnknown, None),
        (Some(a), Some(b)) => {
            if a == b {
                (FieldVerdict::Same, Some(ParamFlags::NONE))
            } else {
                (FieldVerdict::Differ, Some(a.diff(b)))
            }
        }
    };
    ParamVerdict {
        predicted: p.name.clone(),
        observed: Some(o.name.clone()),
        class,
        kind: Some(if p.kind == o.kind {
            FieldVerdict::Same
        } else {
            FieldVerdict::Differ
        }),
        default: Some(cell_verdict(&p.default, &o.default)),
        min: Some(cell_verdict(&p.min, &o.min)),
        max: Some(cell_verdict(&p.max, &o.max)),
        flags: Some(flags),
        flag_diff,
    }
}

/// Pair each predicted row with at most one observed row. Exact names are claimed first, then
/// normalized names, in table order; an observed row is claimed once.
pub fn pair_tables(predicted: &Table, observed: &Table) -> Pairing {
    let mut claimed = vec![false; observed.params.len()];
    let mut slot: Vec<Option<(usize, MatchClass)>> = vec![None; predicted.params.len()];
    for (pi, p) in predicted.params.iter().enumerate() {
        if let Some(oi) = observed
            .params
            .iter()
            .enumerate()
            .position(|(oi, o)| !claimed[oi] && o.name == p.name)
        {
            claimed[oi] = true;
            slot[pi] = Some((oi, MatchClass::Exact));
        }
    }
    for (pi, p) in predicted.params.iter().enumerate() {
        if slot[pi].is_some() {
            continue;
        }
        let np = normalize_name(&p.name);
        if let Some(oi) = observed
            .params
            .iter()
            .enumerate()
            .position(|(oi, o)| !claimed[oi] && normalize_name(&o.name) == np)
        {
            claimed[oi] = true;
            slot[pi] = Some((oi, MatchClass::Normalized));
        }
    }
    let verdicts = predicted
        .params
        .iter()
        .zip(slot)
        .map(|(p, s)| match s {
            Some((oi, class)) => judge(p, &observed.params[oi], class),
            None => ParamVerdict {
                predicted: p.name.clone(),
                observed: None,
                class: MatchClass::Unmatched,
                kind: None,
                default: None,
                min: None,
                max: None,
                flags: None,
                flag_diff: None,
            },
        })
        .collect();
    let unmatched_observed = observed
        .params
        .iter()
        .zip(&claimed)
        .filter(|(_, c)| !**c)
        .map(|(o, _)| o.name.clone())
        .collect();
    Pairing {
        verdicts,
        unmatched_observed,
    }
}
