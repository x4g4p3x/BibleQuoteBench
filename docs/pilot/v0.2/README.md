# BibleQuoteBench v0.2 synthetic pilot

**Synthetic validation only. These are constructed responses, not measurements of GPT-6 or any live model. No paid calls were made.**

The two synthetic generators deliberately exercise exact recall, punctuation changes, edition substitution, mixed wording, refusal, empty output, and provider failure. Their names and rates have no model-performance meaning. Three repetitions per generator are evaluated with complete coverage.

| Track | Cases/run | Reference clusters | Synthetic A ExactText | Synthetic B ExactText |
| --- | ---: | ---: | ---: | ---: |
| [canonical](canonical/analysis.md) | 36 | 12 | 40.74% | 38.89% |
| [concise](concise/analysis.md) | 36 | 12 | 40.74% | 38.89% |
| [word_for_word](word_for_word/analysis.md) | 36 | 12 | 40.74% | 38.89% |
| [copy_control](copy_control/analysis.md) | 36 | 12 | 91.67% | 91.67% |
| [passage](passage/analysis.md) | 36 | 12 | 40.74% | 34.26% |

[Open the interactive results report](index.html) for model comparisons, edition diagnostics, and annotated failure exploration. It is self-contained and works offline.

Each linked analysis includes cluster-bootstrap intervals, paired differences, edition-differing subsets, translation-by-stratum breakdowns, provider-error accounting, and annotated failures. Raw synthetic responses and manifest hashes permit independent reconstruction.

Live pilot: prepared but held by user. Requested target: GPT-6 Astra (max reasoning); budget ceiling: EUR 20; spent: EUR 0. The documented API model is gpt-6-astra; the requested max reasoning level is documented. Account access, current pricing, and an explicitly enabled budget policy must be verified before any later live execution.
