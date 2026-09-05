# BibleQuoteBench validated analysis

Evidence: **synthetic_fixture**. Track: **copy_control**.

- 95% percentile intervals use deterministic stratified reference-cluster bootstrap; translations and repetitions stay together. Intervals are conditional on these runs, not estimates of future provider drift.
- Pairwise intervals are exploratory and unadjusted for multiple comparisons. Small pilot strata can yield degenerate intervals; no superiority claim follows automatically.
- The overall score is the fixed benchmark mixture, not a population estimate for the whole Bible. Provider errors count as end-to-end failures and are excluded only from conditional recall.
- Exact alternative matches establish textual resemblance, not training provenance. Hidden references are not necessarily unseen training text.
- Matched requested temperature and token limits do not guarantee identical effective behavior across providers. A missing resolved model identifier is explicitly marked unresolved.

## openai_compatible / synthetic-a / synthetic-a / 42038f972648

3 complete repetitions; 36 cases each; 12 reference clusters.

ExactText: 91.67% [95% CI 87.04, 96.30]. ExactWords: 91.67% [95% CI 87.04, 96.30].

Provider errors: 0.00%. Recall given provider success: 91.67%.

### Descriptive breakdown

### Overall

| Responses | ExactText | ExactWords | Word accuracy | Refusals | Provider errors | Translation confusion |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 108 | 91.67% | 91.67% | 98.73% | 0.00% | 0.00% | 0.00% |

### Models

| Provider / model | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| openai_compatible / synthetic-a / synthetic-a | 108 | 91.67% | 91.67% | 98.73% | 0.00% |

### Translations

| Translation | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| asv-1901 | 36 | 91.67% | 91.67% | 98.77% | 0.00% |
| bsb-2025-third-printing | 36 | 91.67% | 91.67% | 98.69% | 0.00% |
| web-classic-2020 | 36 | 91.67% | 91.67% | 98.73% | 0.00% |

### Strata

| Stratum | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| extremely_famous | 9 | 88.89% | 88.89% | 97.04% | 0.00% |
| long_verse | 27 | 88.89% | 88.89% | 99.34% | 0.00% |
| random | 18 | 83.33% | 83.33% | 97.62% | 0.00% |
| short_verse | 18 | 100.00% | 100.00% | 100.00% | 0.00% |
| translation_sensitive | 18 | 100.00% | 100.00% | 100.00% | 0.00% |
| well_known | 18 | 88.89% | 88.89% | 97.22% | 0.00% |

### Requested → resembles

Counts use exact requested/other-edition matches first, then the closest alternative when it is strictly closer; remaining errors are `_unclassified`.

- **asv-1901**: _unclassified=3, asv-1901=33
- **bsb-2025-third-printing**: _unclassified=3, bsb-2025-third-printing=33
- **web-classic-2020**: _unclassified=3, web-classic-2020=33

Exact other-edition matches (separate from approximate resemblance):


### Stability

| Provider / model | Repeated cases | Output consistency | Exact recall |
| --- | ---: | ---: | ---: |
| openai_compatible / synthetic-a / synthetic-a | 36 | 91.67% | 91.67% |

Edition-differing cases: 99; ExactText 90.91%.

| Translation / stratum | Responses | ExactText |
| --- | ---: | ---: |
| asv-1901 / extremely_famous | 3 | 100.00% |
| asv-1901 / long_verse | 9 | 88.89% |
| asv-1901 / random | 6 | 83.33% |
| asv-1901 / short_verse | 6 | 100.00% |
| asv-1901 / translation_sensitive | 6 | 100.00% |
| asv-1901 / well_known | 6 | 83.33% |
| bsb-2025-third-printing / extremely_famous | 3 | 66.67% |
| bsb-2025-third-printing / long_verse | 9 | 88.89% |
| bsb-2025-third-printing / random | 6 | 83.33% |
| bsb-2025-third-printing / short_verse | 6 | 100.00% |
| bsb-2025-third-printing / translation_sensitive | 6 | 100.00% |
| bsb-2025-third-printing / well_known | 6 | 100.00% |
| web-classic-2020 / extremely_famous | 3 | 100.00% |
| web-classic-2020 / long_verse | 9 | 88.89% |
| web-classic-2020 / random | 6 | 83.33% |
| web-classic-2020 / short_verse | 6 | 100.00% |
| web-classic-2020 / translation_sensitive | 6 | 100.00% |
| web-classic-2020 / well_known | 6 | 83.33% |

## openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12

3 complete repetitions; 36 cases each; 12 reference clusters.

ExactText: 91.67% [95% CI 87.96, 96.30]. ExactWords: 91.67% [95% CI 87.96, 96.30].

Provider errors: 0.00%. Recall given provider success: 91.67%.

### Descriptive breakdown

### Overall

| Responses | ExactText | ExactWords | Word accuracy | Refusals | Provider errors | Translation confusion |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 108 | 91.67% | 91.67% | 98.58% | 0.00% | 0.00% | 0.00% |

### Models

| Provider / model | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| openai_compatible / synthetic-b / synthetic-b | 108 | 91.67% | 91.67% | 98.58% | 0.00% |

### Translations

| Translation | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| asv-1901 | 36 | 91.67% | 91.67% | 98.78% | 0.00% |
| bsb-2025-third-printing | 36 | 91.67% | 91.67% | 98.92% | 0.00% |
| web-classic-2020 | 36 | 91.67% | 91.67% | 98.04% | 0.00% |

### Strata

| Stratum | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| extremely_famous | 9 | 100.00% | 100.00% | 100.00% | 0.00% |
| long_verse | 27 | 81.48% | 81.48% | 98.94% | 0.00% |
| random | 18 | 100.00% | 100.00% | 100.00% | 0.00% |
| short_verse | 18 | 100.00% | 100.00% | 100.00% | 0.00% |
| translation_sensitive | 18 | 83.33% | 83.33% | 94.54% | 0.00% |
| well_known | 18 | 94.44% | 94.44% | 98.52% | 0.00% |

### Requested → resembles

Counts use exact requested/other-edition matches first, then the closest alternative when it is strictly closer; remaining errors are `_unclassified`.

- **asv-1901**: _unclassified=3, asv-1901=33
- **bsb-2025-third-printing**: _unclassified=3, bsb-2025-third-printing=33
- **web-classic-2020**: _unclassified=3, web-classic-2020=33

Exact other-edition matches (separate from approximate resemblance):


### Stability

| Provider / model | Repeated cases | Output consistency | Exact recall |
| --- | ---: | ---: | ---: |
| openai_compatible / synthetic-b / synthetic-b | 36 | 91.67% | 91.67% |

Edition-differing cases: 99; ExactText 90.91%.

| Translation / stratum | Responses | ExactText |
| --- | ---: | ---: |
| asv-1901 / extremely_famous | 3 | 100.00% |
| asv-1901 / long_verse | 9 | 77.78% |
| asv-1901 / random | 6 | 100.00% |
| asv-1901 / short_verse | 6 | 100.00% |
| asv-1901 / translation_sensitive | 6 | 83.33% |
| asv-1901 / well_known | 6 | 100.00% |
| bsb-2025-third-printing / extremely_famous | 3 | 100.00% |
| bsb-2025-third-printing / long_verse | 9 | 77.78% |
| bsb-2025-third-printing / random | 6 | 100.00% |
| bsb-2025-third-printing / short_verse | 6 | 100.00% |
| bsb-2025-third-printing / translation_sensitive | 6 | 100.00% |
| bsb-2025-third-printing / well_known | 6 | 83.33% |
| web-classic-2020 / extremely_famous | 3 | 100.00% |
| web-classic-2020 / long_verse | 9 | 88.89% |
| web-classic-2020 / random | 6 | 100.00% |
| web-classic-2020 / short_verse | 6 | 100.00% |
| web-classic-2020 / translation_sensitive | 6 | 66.67% |
| web-classic-2020 / well_known | 6 | 100.00% |

## Paired comparisons

Differences are left minus right, in percentage points.

- openai_compatible / synthetic-a / synthetic-a / 42038f972648 versus openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12: 0.00% [95% CI -7.41, 7.41]

## Annotated failures

Whitespace-token display diff; scoring uses the normative Unicode tokenizer. Deletions use [-word], additions [+word]. Examples select the first case per diagnostic category deterministically; they are illustrative, not prevalence estimates.

### Matthew 5:14 — BQ-DEV-2C191E53533986B5-copy_control (ExtraneousText)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: web-classic-2020.

Expected: “You are the light of the world. A city located on a hill can’t be hidden.

Produced: Here is the passage: “You are the light of the world. A city located on a hill can’t be hidden.

Edits: &#91;+Here&#93; &#91;+is&#93; &#91;+the&#93; &#91;+passage:&#93; “You are the light of the world. A city located on a hill can’t be hidden.

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: 0.00%.


### Luke 22:9 — BQ-DEV-30B7A7F315281191-copy_control (ExtraneousText)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: asv-1901.

Expected: And they said unto him, Where wilt thou that we make ready?

Produced: Here is the passage: And they said unto him, Where wilt thou that we make ready?

Edits: &#91;+Here&#93; &#91;+is&#93; &#91;+the&#93; &#91;+passage:&#93; And they said unto him, Where wilt thou that we make ready?

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: 0.00%.
