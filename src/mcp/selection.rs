use std::collections::BTreeMap;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::{BenchmarkCase, PromptVariant, study::digest};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SelectionSpec {
    pub method: String,
    pub seed: String,
    pub case_limit: Option<usize>,
    pub translation: Option<String>,
}

pub(super) fn select(cases: &[BenchmarkCase], spec: &SelectionSpec) -> Result<Vec<BenchmarkCase>> {
    if spec.method != "stratified_reference_v1"
        || spec.seed.trim().is_empty()
        || spec.seed.len() > 256
    {
        bail!("invalid MCP selection method or seed");
    }
    if spec.case_limit == Some(0) {
        bail!("case_limit must be positive");
    }
    let mut groups: BTreeMap<String, Vec<BenchmarkCase>> = BTreeMap::new();
    for case in cases.iter().filter(|case| {
        spec.translation
            .as_ref()
            .is_none_or(|id| id == &case.translation)
    }) {
        if case.prompt_variant == PromptVariant::CopyControl {
            bail!("MCP recall trials do not expose copy-control reference text");
        }
        groups
            .entry(digest(&case.reference))
            .or_default()
            .push(case.clone());
    }
    if groups.is_empty() {
        bail!("selection contains no cases; check the translation");
    }
    let mut translations = None;
    let mut strata: BTreeMap<String, Vec<Vec<BenchmarkCase>>> = BTreeMap::new();
    for mut group in groups.into_values() {
        group.sort_by(|left, right| left.translation.cmp(&right.translation));
        let ids: Vec<_> = group.iter().map(|case| case.translation.clone()).collect();
        if translations
            .as_ref()
            .is_some_and(|expected| expected != &ids)
            || group.iter().any(|case| case.stratum != group[0].stratum)
        {
            bail!("MCP selection requires matched translations and strata across references");
        }
        translations = Some(ids);
        let stratum = serde_json::to_value(group[0].stratum).expect("enum serializes");
        strata
            .entry(stratum.as_str().expect("string enum").into())
            .or_default()
            .push(group);
    }
    let group_size = translations.expect("nonempty groups").len();
    let total_groups: usize = strata.values().map(Vec::len).sum();
    let target = spec
        .case_limit
        .map_or(total_groups, |limit| (limit / group_size).min(total_groups));
    if target == 0 {
        bail!("case_limit must fit a complete reference group of {group_size} translations");
    }
    for bucket in strata.values_mut() {
        bucket.sort_by_key(|group| digest(&(&spec.seed, "reference", &group[0].reference)));
    }
    let mut quotas: BTreeMap<String, usize> = strata
        .keys()
        .map(|name| (name.clone(), usize::from(target >= strata.len())))
        .collect();
    while quotas.values().sum::<usize>() < target {
        let chosen = strata
            .iter()
            .filter(|(name, bucket)| quotas[*name] < bucket.len())
            .min_by_key(|(name, bucket)| {
                let deficit = integer(target) * integer(bucket.len())
                    - integer(quotas[*name]) * integer(total_groups);
                (-deficit, digest(&(&spec.seed, "stratum", *name)))
            })
            .expect("remaining capacity")
            .0
            .clone();
        *quotas.get_mut(&chosen).expect("known stratum") += 1;
    }
    let mut selected: Vec<_> = strata
        .into_iter()
        .flat_map(|(name, bucket)| bucket.into_iter().take(quotas[&name]).flatten())
        .collect();
    // Presentation order is also seeded, rather than grouped by book or stratum.
    selected.sort_by_key(|case| {
        (
            digest(&(&spec.seed, "presentation", &case.reference)),
            case.translation.clone(),
        )
    });
    Ok(selected)
}

fn integer(value: usize) -> i128 {
    i128::try_from(value).expect("collection sizes fit i128")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cases() -> Vec<BenchmarkCase> {
        crate::io::read_jsonl(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/dev/cases.jsonl"),
        )
        .unwrap()
    }

    fn spec() -> SelectionSpec {
        SelectionSpec {
            method: "stratified_reference_v1".into(),
            seed: "fixture".into(),
            case_limit: Some(10),
            translation: Some("bsb-2025-third-printing".into()),
        }
    }

    #[test]
    fn short_trials_cover_strata_and_are_seeded_and_input_order_independent() {
        let mut cases = cases();
        let selected = select(&cases, &spec()).unwrap();
        assert_eq!(selected.len(), 10);
        let strata: std::collections::BTreeSet<_> = selected
            .iter()
            .map(|case| format!("{:?}", case.stratum))
            .collect();
        assert_eq!(strata.len(), 6);
        cases.reverse();
        assert_eq!(selected, select(&cases, &spec()).unwrap());
        assert_ne!(
            selected,
            select(
                &cases,
                &SelectionSpec {
                    seed: "different".into(),
                    ..spec()
                }
            )
            .unwrap()
        );
        assert!(
            selected
                .iter()
                .map(|case| &case.reference.book)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                > 1
        );
    }

    #[test]
    fn multi_edition_trials_keep_reference_groups_complete() {
        let cases = cases();
        let selected = select(
            &cases,
            &SelectionSpec {
                translation: None,
                case_limit: Some(20),
                ..spec()
            },
        )
        .unwrap();
        assert_eq!(selected.len(), 18);
        for group in selected.chunks(3) {
            assert!(
                group
                    .iter()
                    .all(|case| case.reference == group[0].reference)
            );
            assert_eq!(
                group
                    .iter()
                    .map(|case| &case.translation)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                3
            );
        }
        assert_eq!(
            select(
                &cases,
                &SelectionSpec {
                    translation: None,
                    case_limit: None,
                    ..spec()
                }
            )
            .unwrap()
            .len(),
            cases.len()
        );
        assert_eq!(
            select(
                &cases,
                &SelectionSpec {
                    case_limit: Some(10000),
                    ..spec()
                }
            )
            .unwrap()
            .len(),
            100
        );
    }

    #[test]
    fn invalid_sampling_and_unmatched_groups_are_rejected() {
        for spec in [
            SelectionSpec {
                seed: " ".into(),
                ..spec()
            },
            SelectionSpec {
                method: "unknown".into(),
                ..spec()
            },
            SelectionSpec {
                case_limit: Some(0),
                ..spec()
            },
            SelectionSpec {
                translation: Some("missing".into()),
                ..spec()
            },
            SelectionSpec {
                translation: None,
                case_limit: Some(2),
                ..spec()
            },
        ] {
            assert!(select(&cases(), &spec).is_err());
        }
        let mut unmatched = cases();
        unmatched.remove(0);
        assert!(
            select(
                &unmatched,
                &SelectionSpec {
                    translation: None,
                    ..spec()
                }
            )
            .is_err()
        );
        let mut controls = cases();
        controls[0].prompt_variant = PromptVariant::CopyControl;
        assert!(select(&controls, &spec()).is_err());
        let mut mixed = cases();
        mixed[0].stratum = crate::CaseStratum::ExtremelyFamous;
        assert!(
            select(
                &mixed,
                &SelectionSpec {
                    translation: None,
                    ..spec()
                }
            )
            .is_err()
        );
    }
}
