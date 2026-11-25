//! The pairing: match a predicted table against an observed table.
//!
//! Key rule: a predicted row whose id is known is paired with the observed row of the same
//! known id (`MatchClass::ById`). Rows left over are paired by name, exact bytes first and
//! then normalized, but never when both rows carry a known id and the ids differ (they are
//! different parameters), and never an empty name (an empty name is not a key). The verdict
//! reports the key used.

#[cfg(not(feature = "std"))]
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::{
    flags::ParamFlags,
    table::{Cell, ParamDef, Table},
};

/// How a predicted row found its observed row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchClass {
    /// Both rows carry the same known host id.
    ById,
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

/// Which key a pair was made on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairKey {
    Id,
    Name,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamVerdict {
    pub predicted: String,
    pub observed: Option<String>,
    pub class: MatchClass,
    /// The predicted row's id, when known.
    pub id_predicted: Option<i64>,
    /// The key the pair was made on: `Some(PairKey::Id)`, `Some(PairKey::Name)`, `None` when
    /// unmatched.
    pub key: Option<PairKey>,
    pub id: Option<FieldVerdict>,
    pub name: Option<FieldVerdict>,
    pub kind: Option<FieldVerdict>,
    pub default: Option<FieldVerdict>,
    pub min: Option<FieldVerdict>,
    pub max: Option<FieldVerdict>,
    pub flags: Option<FieldVerdict>,
    pub extra: Option<FieldVerdict>,
    pub host_flags: Option<FieldVerdict>,
    pub host_ui_flags: Option<FieldVerdict>,
    /// Bits set in exactly one flag set, when both are known.
    pub flag_diff: Option<ParamFlags>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pairing {
    pub verdicts: Vec<ParamVerdict>,
    /// Observed rows that no predicted row claimed: the name, or `<id i:N>` when the name is
    /// empty (`<no name, no id>` for a hand built row with neither).
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

fn opt_verdict<T: PartialEq>(p: Option<T>, o: Option<T>) -> FieldVerdict {
    match (p, o) {
        (None, _) => FieldVerdict::Unscored,
        (Some(_), None) => FieldVerdict::ObservedUnknown,
        (Some(a), Some(b)) => {
            if a == b {
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
        id_predicted: p.id,
        key: Some(if class == MatchClass::ById {
            PairKey::Id
        } else {
            PairKey::Name
        }),
        id: Some(opt_verdict(p.id, o.id)),
        name: Some(if p.name == o.name {
            FieldVerdict::Same
        } else {
            FieldVerdict::Differ
        }),
        host_flags: Some(opt_verdict(p.host_flags, o.host_flags)),
        host_ui_flags: Some(opt_verdict(p.host_ui_flags, o.host_ui_flags)),
        kind: Some(if p.kind == o.kind {
            FieldVerdict::Same
        } else {
            FieldVerdict::Differ
        }),
        default: Some(cell_verdict(&p.default, &o.default)),
        min: Some(cell_verdict(&p.min, &o.min)),
        max: Some(cell_verdict(&p.max, &o.max)),
        flags: Some(flags),
        extra: Some(if p.extra.is_unknown() {
            FieldVerdict::Unscored
        } else if o.extra.is_unknown() {
            FieldVerdict::ObservedUnknown
        } else if p.extra == o.extra {
            FieldVerdict::Same
        } else {
            FieldVerdict::Differ
        }),
        flag_diff,
    }
}

/// True when both rows carry a known id and the ids differ.
fn ids_conflict(p: &ParamDef, o: &ParamDef) -> bool {
    matches!((p.id, o.id), (Some(a), Some(b)) if a != b)
}

/// Pair each predicted row with at most one observed row: first by known id, then by exact
/// name, then by normalized name, in table order; an observed row is claimed once.
pub fn pair_tables(predicted: &Table, observed: &Table) -> Pairing {
    let mut claimed = vec![false; observed.params.len()];
    let mut slot: Vec<Option<(usize, MatchClass)>> = vec![None; predicted.params.len()];
    for (pi, p) in predicted.params.iter().enumerate() {
        let Some(id) = p.id else { continue };
        if let Some(oi) = observed
            .params
            .iter()
            .enumerate()
            .position(|(oi, o)| !claimed[oi] && o.id == Some(id))
        {
            claimed[oi] = true;
            slot[pi] = Some((oi, MatchClass::ById));
        }
    }
    for (pi, p) in predicted.params.iter().enumerate() {
        if slot[pi].is_some() || p.name.is_empty() {
            continue;
        }
        if let Some(oi) = observed
            .params
            .iter()
            .enumerate()
            .position(|(oi, o)| !claimed[oi] && o.name == p.name && !ids_conflict(p, o))
        {
            claimed[oi] = true;
            slot[pi] = Some((oi, MatchClass::Exact));
        }
    }
    for (pi, p) in predicted.params.iter().enumerate() {
        if slot[pi].is_some() || p.name.is_empty() {
            continue;
        }
        let np = normalize_name(&p.name);
        if let Some(oi) = observed.params.iter().enumerate().position(|(oi, o)| {
            !claimed[oi]
                && !o.name.is_empty()
                && normalize_name(&o.name) == np
                && !ids_conflict(p, o)
        }) {
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
                id_predicted: p.id,
                key: None,
                id: None,
                name: None,
                host_flags: None,
                host_ui_flags: None,
                kind: None,
                default: None,
                min: None,
                max: None,
                flags: None,
                extra: None,
                flag_diff: None,
            },
        })
        .collect();
    let unmatched_observed = observed
        .params
        .iter()
        .zip(&claimed)
        .filter(|(_, c)| !**c)
        .map(|(o, _)| match (o.name.is_empty(), o.id) {
            (false, _) => o.name.clone(),
            (true, Some(id)) => format!("<id i:{id}>"),
            (true, None) => "<no name, no id>".to_string(),
        })
        .collect();
    Pairing {
        verdicts,
        unmatched_observed,
    }
}
