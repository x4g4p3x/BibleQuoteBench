# Scoring contract

`ExactText` compares the reference and response after only two transport-level
normalizations: CRLF/CR line endings become LF, and Unicode is converted to NFC.
Whitespace, capitalization, and punctuation remain significant.

`ExactWords` compares case-sensitive Unicode word sequences. Punctuation is
ignored, and straight and curly apostrophes inside words are treated alike.
Punctuation-only differences do not count as extra prose. Copy and recall tracks
use the same strict metrics; their scores are never combined.

Word and character error rates use Levenshtein alignment. Word accuracy is
`max(0, 1 - WER)`. Exact scores cannot succeed for a provider-error response.

An exact match for another catalogued translation of the same passage is
classified as `translation_confusion`. Merely being closer to another edition is
diagnostic resemblance and never sufficient to assert confusion. All exact-match
and closest-distance ties are retained in sorted lists. Singular legacy fields
contain the first sorted match; consumers should prefer the full lists.

The requested-to-resembles matrix labels ambiguous exact and approximate matches
separately. The exact-alternative matrix counts every matching alternative; a
response can contribute to multiple cells, while its confusion rate is counted
only once. Provider failures, empty responses, and refusals get dedicated matrix
categories rather than being attributed to the shortest alternative.

`alternative_edition_token_overlap` is the fraction of produced tokens that fail
to align with the target but do align in at least one available alternative.
Deleted target words are excluded because no competing token was produced. The
value is undefined when there are no wrong produced tokens or no alternatives.
Overlap is evidence of textual resemblance, not evidence of training provenance
or a causal explanation of an error. Old JSON using
`translation_contamination_rate` remains readable; new JSON uses the new name.

Refusal detection remains a transparent phrase heuristic, not a semantic
classifier. All raw responses remain independently auditable.

## Validated comparisons

The lightweight `report` and `summarize` commands describe only supplied rows.
They do not verify completion or comparability. Use `analyze` for comparisons.

Each `run` writes a companion `.manifest.json` binding the intended case IDs,
typed dataset and catalog hashes, executable prompts, response records, engine
version, endpoint digest, requested model, temperature, reasoning effort, and
output-token limit. A case-limited smoke run intentionally fails full-dataset
analysis. Prepare a smaller dataset when a complete pilot is desired.

`analyze` rejects missing, duplicate, and unexpected cases; duplicate runs;
altered artifacts; inconsistent response settings; within-run model/fingerprint
changes; unbalanced translation coverage; differing strata for the same reference;
overlapping passage clusters; and unequal repetitions across configurations.
Model configurations and resolved versions remain separate. Synthetic and live
evidence cannot be pooled. Hashes provide integrity checks, not provider-signed
proof that a model actually produced a response.

End-to-end exact recall includes provider failures in its denominator. Conditional
recall excludes only provider failures and is undefined when no requests succeed.
Refusals and empty successful responses remain recall failures. Repeated provider
errors cannot inflate output stability; duplicate run IDs are not repetitions.

Provider token-limit cutoffs are classified as `truncated`, separately from recall
errors and transport/provider errors. Both exact metrics fail for a cutoff. Cutoffs
remain in end-to-end and provider-success-conditional recall denominators, but are
excluded from output stability and receive a dedicated resemblance-matrix column.
See [execution accounting](EXECUTION.md) for resume and budget semantics.

Reports include edition-differing subsets, translation-by-stratum breakdowns,
and deterministic illustrative failures with display-level word diffs. Reference
and output examples are suppressed for hidden IDs or private licensed catalogs.

## Statistical interpretation

Reported 95% percentile intervals use 2,000 deterministic bootstrap resamples by
default. First average translations and repetitions within each reference; then
resample references within strata, preserving each stratum's size. Paired model
differences resample the same reference groups on both sides. Thus 1,200 cases
over three editions represent 400 reference clusters, not 1,200 independent units.

Comparisons require equal repetitions and matching requested temperature,
reasoning effort, and output-token limits. Unspecified temperature/default behavior
produces descriptive reports without paired intervals. Matching requested settings
still does not guarantee identical provider behavior. Model aliases without a
returned version identifier are explicitly marked unresolved.

Intervals describe reference-sampling uncertainty conditional on these observed
runs; they do not estimate future provider drift or uncertainty in curation.
Tiny strata may produce degenerate intervals. Pairwise intervals are exploratory,
unadjusted for multiple comparisons, and do not automatically declare a winner.

The paired bootstrap follows the general method described by
[Koehn (2004)](https://aclanthology.org/W04-3250/), adapted here to stratified
reference clusters with editions and repetitions kept together.
