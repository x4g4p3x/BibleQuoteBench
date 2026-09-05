//! Deterministic paired, stratified cluster bootstrap over reference groups.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interval {
    pub estimate: f64,
    pub lower: f64,
    pub upper: f64,
    pub clusters: usize,
    pub resamples: usize,
}

/// Bootstrap a mean or paired difference, preserving each stratum's sample size.
/// Input values must already average translations and repetitions within a reference.
///
/// # Panics
/// Panics on empty groups, nonfinite values, or fewer than 100 resamples.
#[allow(clippy::cast_precision_loss)]
pub fn bootstrap(groups: &BTreeMap<String, Vec<f64>>, resamples: usize) -> Interval {
    assert!(resamples >= 100 && !groups.is_empty());
    assert!(
        groups
            .values()
            .all(|values| !values.is_empty() && values.iter().all(|v| v.is_finite()))
    );
    let clusters: usize = groups.values().map(Vec::len).sum();
    let estimate = groups.values().flatten().sum::<f64>() / clusters as f64;
    let mut random = Random(0);
    let mut samples = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        let mut sum = 0.0;
        for values in groups.values() {
            for _ in values {
                sum += values[random.index(values.len())];
            }
        }
        samples.push(sum / clusters as f64);
    }
    samples.sort_by(f64::total_cmp);
    Interval {
        estimate,
        lower: samples[resamples * 25 / 1000],
        upper: samples[(resamples * 975 / 1000).min(resamples - 1)],
        clusters,
        resamples,
    }
}

struct Random(u64);
impl Random {
    fn index(&mut self, length: usize) -> usize {
        let bound = u64::try_from(length).expect("length fits u64");
        let ceiling = u64::MAX - u64::MAX % bound;
        loop {
            let hash = Sha256::digest(format!("bqb/bootstrap/v1/{}", self.0));
            self.0 += 1;
            let value = u64::from_le_bytes(hash[..8].try_into().expect("eight bytes"));
            if value < ceiling {
                return usize::try_from(value % bound).expect("index fits usize");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constant_paired_difference_and_strata_stay_fixed() {
        let groups = BTreeMap::from([("a".into(), vec![0.25; 3]), ("b".into(), vec![0.75])]);
        let result = bootstrap(&groups, 500);
        assert!((result.estimate - 0.375).abs() < f64::EPSILON);
        assert!((result.lower - result.upper).abs() < f64::EPSILON);
        assert_eq!(result, bootstrap(&groups, 500));
        assert_eq!(result.clusters, 4);
    }
    #[test]
    fn variable_samples_have_uncertainty() {
        let result = bootstrap(
            &BTreeMap::from([("random".into(), vec![0.0, 1.0, 0.0, 1.0])]),
            1000,
        );
        assert!(result.lower < result.estimate && result.upper > result.estimate);
    }
}
