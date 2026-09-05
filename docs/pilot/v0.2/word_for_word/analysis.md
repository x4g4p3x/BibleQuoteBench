# BibleQuoteBench validated analysis

Evidence: **synthetic_fixture**. Track: **word_for_word**.

- 95% percentile intervals use deterministic stratified reference-cluster bootstrap; translations and repetitions stay together. Intervals are conditional on these runs, not estimates of future provider drift.
- Pairwise intervals are exploratory and unadjusted for multiple comparisons. Small pilot strata can yield degenerate intervals; no superiority claim follows automatically.
- The overall score is the fixed benchmark mixture, not a population estimate for the whole Bible. Provider errors count as end-to-end failures and are excluded only from conditional recall.
- Exact alternative matches establish textual resemblance, not training provenance. Hidden references are not necessarily unseen training text.
- Matched requested temperature and token limits do not guarantee identical effective behavior across providers. A missing resolved model identifier is explicitly marked unresolved.

## openai_compatible / synthetic-a / synthetic-a / 42038f972648

3 complete repetitions; 36 cases each; 12 reference clusters.

ExactText: 40.74% [95% CI 27.78, 54.63]. ExactWords: 50.00% [95% CI 36.11, 63.89].

Provider errors: 8.33%. Recall given provider success: 44.44%.

### Descriptive breakdown

### Overall

| Responses | ExactText | ExactWords | Word accuracy | Refusals | Provider errors | Translation confusion |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 108 | 40.74% | 50.00% | 66.02% | 8.33% | 8.33% | 8.33% |

### Models

| Provider / model | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| openai_compatible / synthetic-a / synthetic-a | 108 | 40.74% | 50.00% | 66.02% | 8.33% |

### Translations

| Translation | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| asv-1901 | 36 | 41.67% | 50.00% | 65.70% | 8.33% |
| bsb-2025-third-printing | 36 | 41.67% | 50.00% | 67.44% | 8.33% |
| web-classic-2020 | 36 | 38.89% | 50.00% | 64.92% | 8.33% |

### Strata

| Stratum | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| extremely_famous | 9 | 33.33% | 33.33% | 41.48% | 0.00% |
| long_verse | 27 | 7.41% | 18.52% | 50.52% | 18.52% |
| random | 18 | 33.33% | 33.33% | 51.59% | 0.00% |
| short_verse | 18 | 88.89% | 94.44% | 94.44% | 0.00% |
| translation_sensitive | 18 | 50.00% | 77.78% | 81.82% | 16.67% |
| well_known | 18 | 44.44% | 50.00% | 71.74% | 5.56% |

### Requested → resembles

Counts use exact requested/other-edition matches first, then the closest alternative when it is strictly closer; remaining errors are `_unclassified`.

- **asv-1901**: _empty=3, _provider_error=3, _refusal=3, _unclassified=7, asv-1901=15, bsb-2025-third-printing=5
- **bsb-2025-third-printing**: _empty=3, _provider_error=3, _refusal=3, _unclassified=7, asv-1901=4, bsb-2025-third-printing=15, web-classic-2020=1
- **web-classic-2020**: _empty=3, _provider_error=3, _refusal=3, _unclassified=8, bsb-2025-third-printing=5, web-classic-2020=14

Exact other-edition matches (separate from approximate resemblance):

- asv-1901: bsb-2025-third-printing=3
- bsb-2025-third-printing: asv-1901=3
- web-classic-2020: bsb-2025-third-printing=3

### Stability

| Provider / model | Repeated cases | Output consistency | Exact recall |
| --- | ---: | ---: | ---: |
| openai_compatible / synthetic-a / synthetic-a | 36 | 60.19% | 44.91% |

Edition-differing cases: 99; ExactText 36.36%.

| Translation / stratum | Responses | ExactText |
| --- | ---: | ---: |
| asv-1901 / extremely_famous | 3 | 33.33% |
| asv-1901 / long_verse | 9 | 0.00% |
| asv-1901 / random | 6 | 33.33% |
| asv-1901 / short_verse | 6 | 100.00% |
| asv-1901 / translation_sensitive | 6 | 50.00% |
| asv-1901 / well_known | 6 | 50.00% |
| bsb-2025-third-printing / extremely_famous | 3 | 0.00% |
| bsb-2025-third-printing / long_verse | 9 | 11.11% |
| bsb-2025-third-printing / random | 6 | 16.67% |
| bsb-2025-third-printing / short_verse | 6 | 83.33% |
| bsb-2025-third-printing / translation_sensitive | 6 | 83.33% |
| bsb-2025-third-printing / well_known | 6 | 50.00% |
| web-classic-2020 / extremely_famous | 3 | 66.67% |
| web-classic-2020 / long_verse | 9 | 11.11% |
| web-classic-2020 / random | 6 | 50.00% |
| web-classic-2020 / short_verse | 6 | 83.33% |
| web-classic-2020 / translation_sensitive | 6 | 16.67% |
| web-classic-2020 / well_known | 6 | 33.33% |

## openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12

3 complete repetitions; 36 cases each; 12 reference clusters.

ExactText: 38.89% [95% CI 28.70, 50.93]. ExactWords: 49.07% [95% CI 38.89, 61.11].

Provider errors: 8.33%. Recall given provider success: 42.42%.

### Descriptive breakdown

### Overall

| Responses | ExactText | ExactWords | Word accuracy | Refusals | Provider errors | Translation confusion |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 108 | 38.89% | 49.07% | 63.43% | 9.26% | 8.33% | 9.26% |

### Models

| Provider / model | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| openai_compatible / synthetic-b / synthetic-b | 108 | 38.89% | 49.07% | 63.43% | 9.26% |

### Translations

| Translation | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| asv-1901 | 36 | 38.89% | 50.00% | 63.42% | 8.33% |
| bsb-2025-third-printing | 36 | 38.89% | 50.00% | 62.97% | 8.33% |
| web-classic-2020 | 36 | 38.89% | 47.22% | 63.90% | 11.11% |

### Strata

| Stratum | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| extremely_famous | 9 | 88.89% | 100.00% | 100.00% | 0.00% |
| long_verse | 27 | 33.33% | 33.33% | 53.20% | 0.00% |
| random | 18 | 66.67% | 77.78% | 81.58% | 5.56% |
| short_verse | 18 | 50.00% | 77.78% | 87.41% | 16.67% |
| translation_sensitive | 18 | 0.00% | 5.56% | 27.76% | 16.67% |
| well_known | 18 | 22.22% | 33.33% | 54.04% | 16.67% |

### Requested → resembles

Counts use exact requested/other-edition matches first, then the closest alternative when it is strictly closer; remaining errors are `_unclassified`.

- **asv-1901**: _empty=3, _provider_error=3, _refusal=3, _unclassified=8, asv-1901=14, bsb-2025-third-printing=4, web-classic-2020=1
- **bsb-2025-third-printing**: _empty=3, _provider_error=3, _refusal=3, _unclassified=8, asv-1901=5, bsb-2025-third-printing=14
- **web-classic-2020**: _empty=3, _provider_error=3, _refusal=4, _unclassified=7, bsb-2025-third-printing=5, web-classic-2020=14

Exact other-edition matches (separate from approximate resemblance):

- asv-1901: bsb-2025-third-printing=3
- bsb-2025-third-printing: asv-1901=3
- web-classic-2020: bsb-2025-third-printing=4

### Stability

| Provider / model | Repeated cases | Output consistency | Exact recall |
| --- | ---: | ---: | ---: |
| openai_compatible / synthetic-b / synthetic-b | 36 | 59.26% | 43.06% |

Edition-differing cases: 99; ExactText 36.36%.

| Translation / stratum | Responses | ExactText |
| --- | ---: | ---: |
| asv-1901 / extremely_famous | 3 | 100.00% |
| asv-1901 / long_verse | 9 | 33.33% |
| asv-1901 / random | 6 | 66.67% |
| asv-1901 / short_verse | 6 | 50.00% |
| asv-1901 / translation_sensitive | 6 | 0.00% |
| asv-1901 / well_known | 6 | 16.67% |
| bsb-2025-third-printing / extremely_famous | 3 | 100.00% |
| bsb-2025-third-printing / long_verse | 9 | 22.22% |
| bsb-2025-third-printing / random | 6 | 66.67% |
| bsb-2025-third-printing / short_verse | 6 | 66.67% |
| bsb-2025-third-printing / translation_sensitive | 6 | 0.00% |
| bsb-2025-third-printing / well_known | 6 | 16.67% |
| web-classic-2020 / extremely_famous | 3 | 66.67% |
| web-classic-2020 / long_verse | 9 | 44.44% |
| web-classic-2020 / random | 6 | 66.67% |
| web-classic-2020 / short_verse | 6 | 33.33% |
| web-classic-2020 / translation_sensitive | 6 | 0.00% |
| web-classic-2020 / well_known | 6 | 33.33% |

## Paired comparisons

Differences are left minus right, in percentage points.

- openai_compatible / synthetic-a / synthetic-a / 42038f972648 versus openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12: 1.85% [95% CI -17.59, 20.37]

## Annotated failures

Whitespace-token display diff; scoring uses the normative Unicode tokenizer. Deletions use [-word], additions [+word]. Examples select the first case per diagnostic category deterministically; they are illustrative, not prevalence estimates.

### Proverbs 3:5 — BQ-DEV-08063CDB23AE1D69-word_for_word (Empty)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: asv-1901.

Expected: Trust in Jehovah with all thy heart, And lean not upon thine own understanding:

Produced:

Edits: &#91;-Trust&#93; &#91;-in&#93; &#91;-Jehovah&#93; &#91;-with&#93; &#91;-all&#93; &#91;-thy&#93; &#91;-heart,&#93; &#91;-And&#93; &#91;-lean&#93; &#91;-not&#93; &#91;-upon&#93; &#91;-thine&#93; &#91;-own&#93; &#91;-understanding:&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: undefined.


### Proverbs 3:5 — BQ-DEV-08063CDB23AE1D69-word_for_word (ProviderError)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: asv-1901.

Expected: Trust in Jehovah with all thy heart, And lean not upon thine own understanding:

Produced:

Edits: &#91;-Trust&#93; &#91;-in&#93; &#91;-Jehovah&#93; &#91;-with&#93; &#91;-all&#93; &#91;-thy&#93; &#91;-heart,&#93; &#91;-And&#93; &#91;-lean&#93; &#91;-not&#93; &#91;-upon&#93; &#91;-thine&#93; &#91;-own&#93; &#91;-understanding:&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: undefined.


### Matthew 5:14 — BQ-DEV-2C191E53533986B5-word_for_word (Refusal)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: web-classic-2020.

Expected: “You are the light of the world. A city located on a hill can’t be hidden.

Produced: I cannot provide that passage.

Edits: &#91;-“You&#93; &#91;-are&#93; &#91;-the&#93; &#91;-light&#93; &#91;-of&#93; &#91;-the&#93; &#91;-world.&#93; &#91;-A&#93; &#91;-city&#93; &#91;-located&#93; &#91;-on&#93; &#91;-a&#93;&#91;+I&#93; &#91;-hill&#93;&#91;+cannot&#93; &#91;-can’t&#93;&#91;+provide&#93; &#91;-be&#93;&#91;+that&#93; &#91;-hidden.&#93;&#91;+passage.&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: 0.00%.


### Matthew 5:14 — BQ-DEV-2C191E53533986B5-word_for_word (ExtraneousText)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: web-classic-2020.

Expected: “You are the light of the world. A city located on a hill can’t be hidden.

Produced: Here is the passage: “You are the light of the world. A city located on a hill can’t be hidden.

Edits: &#91;+Here&#93; &#91;+is&#93; &#91;+the&#93; &#91;+passage:&#93; “You are the light of the world. A city located on a hill can’t be hidden.

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: 0.00%.


### Luke 22:9 — BQ-DEV-30B7A7F315281191-word_for_word (Partial)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: asv-1901.

Expected: And they said unto him, Where wilt thou that we make ready?

Produced: And they said unto him Where wilt thou that we make ready

Edits: And they said unto &#91;-him,&#93;&#91;+him&#93; Where wilt thou that we make &#91;-ready?&#93;&#91;+ready&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: undefined.


### Luke 22:9 — BQ-DEV-30B7A7F315281191-word_for_word (TranslationConfusion)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: asv-1901.

Expected: And they said unto him, Where wilt thou that we make ready?

Produced: “Where do You want us to prepare it?” they asked.

Edits: &#91;-And&#93; &#91;-they&#93; &#91;-said&#93;&#91;+“Where&#93; &#91;-unto&#93;&#91;+do&#93; &#91;-him,&#93;&#91;+You&#93; &#91;-Where&#93;&#91;+want&#93; &#91;-wilt&#93;&#91;+us&#93; &#91;-thou&#93;&#91;+to&#93; &#91;-that&#93;&#91;+prepare&#93; &#91;-we&#93;&#91;+it?”&#93; &#91;-make&#93;&#91;+they&#93; &#91;-ready?&#93;&#91;+asked.&#93;

Exact alternative editions: bsb-2025-third-printing. Closest alternatives: bsb-2025-third-printing. Alternative-edition token overlap: 100.00%.


### 2 Kings 1:6 — BQ-DEV-31D278E968BBECF0-word_for_word (Partial)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: asv-1901.

Expected: And they said unto him, There came up a man to meet us, and said unto us, Go, turn again unto the king that sent you, and say unto him, Thus saith Jehovah, Is it because there is no God in Israel, that thou sendest to inquire of Baal-zebub, the god of Ekron? therefore thou shalt not come down from the bed whither thou art gone up, but shalt surely die.

Produced: And they said unto him, There came up a man to meet us, and said unto us, Go, turn again unto the king that sent you, and say unto him, Thus saith Jehovah, Is it God in Israel that you are sending these men to inquire of Baal-zebub, the god of Ekron? Therefore you will not get up from the bed on which you are lying. You will surely die.’”

Edits: And they said unto him, There came up a man to meet us, and said unto us, Go, turn again unto the king that sent you, and say unto him, Thus saith Jehovah, Is it &#91;-because&#93; &#91;-there&#93;&#91;+God&#93; &#91;-is&#93;&#91;+in&#93; &#91;-no&#93;&#91;+Israel&#93; &#91;-God&#93;&#91;+that&#93; &#91;-in&#93;&#91;+you&#93; &#91;-Israel,&#93;&#91;+are&#93; &#91;-that&#93;&#91;+sending&#93; &#91;-thou&#93;&#91;+these&#93; &#91;-sendest&#93;&#91;+men&#93; to inquire of Baal-zebub, the god of Ekron? &#91;-therefore&#93;&#91;+Therefore&#93; &#91;-thou&#93;&#91;+you&#93; &#91;-shalt&#93;&#91;+will&#93; not &#91;-come&#93;&#91;+get&#93; &#91;-down&#93;&#91;+up&#93; from the bed &#91;-whither&#93;&#91;+on&#93; &#91;-thou&#93;&#91;+which&#93; &#91;-art&#93;&#91;+you&#93; &#91;-gone&#93;&#91;+are&#93; &#91;-up,&#93;&#91;+lying.&#93; &#91;-but&#93;&#91;+You&#93; &#91;-shalt&#93;&#91;+will&#93; surely &#91;-die.&#93;&#91;+die.’”&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: 100.00%.


### Matthew 5:14 — BQ-DEV-2C191E53533986B5-word_for_word (ProviderError)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: web-classic-2020.

Expected: “You are the light of the world. A city located on a hill can’t be hidden.

Produced:

Edits: &#91;-“You&#93; &#91;-are&#93; &#91;-the&#93; &#91;-light&#93; &#91;-of&#93; &#91;-the&#93; &#91;-world.&#93; &#91;-A&#93; &#91;-city&#93; &#91;-located&#93; &#91;-on&#93; &#91;-a&#93; &#91;-hill&#93; &#91;-can’t&#93; &#91;-be&#93; &#91;-hidden.&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: undefined.


### 1 Samuel 13:21 — BQ-DEV-2CB242707886D004-word_for_word (Partial)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: bsb-2025-third-printing.

Expected: The charge was a pim for sharpening a plowshare or mattock, a third of a shekel for sharpening a pitchfork or an axe, and a third of a shekel for repointing an oxgoad.

Produced: The charge was a pim for sharpening a plowshare or mattock a third of a shekel for sharpening a pitchfork or an axe and a third of a shekel for repointing an oxgoad

Edits: The charge was a pim for sharpening a plowshare or &#91;-mattock,&#93;&#91;+mattock&#93; a third of a shekel for sharpening a pitchfork or an &#91;-axe,&#93;&#91;+axe&#93; and a third of a shekel for repointing an &#91;-oxgoad.&#93;&#91;+oxgoad&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: undefined.


### 1 Samuel 13:21 — BQ-DEV-2CB242707886D004-word_for_word (TranslationConfusion)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: bsb-2025-third-printing.

Expected: The charge was a pim for sharpening a plowshare or mattock, a third of a shekel for sharpening a pitchfork or an axe, and a third of a shekel for repointing an oxgoad.

Produced: yet they had a file for the mattocks, and for the coulters, and for the forks, and for the axes, and to set the goads.

Edits: &#91;-The&#93;&#91;+yet&#93; &#91;-charge&#93;&#91;+they&#93; &#91;-was&#93;&#91;+had&#93; a &#91;-pim&#93;&#91;+file&#93; for &#91;-sharpening&#93; &#91;-a&#93; &#91;-plowshare&#93; &#91;-or&#93; &#91;-mattock,&#93; &#91;-a&#93; &#91;-third&#93; &#91;-of&#93;&#91;+the&#93; &#91;-a&#93;&#91;+mattocks,&#93; &#91;-shekel&#93;&#91;+and&#93; for &#91;-sharpening&#93;&#91;+the&#93; &#91;-a&#93;&#91;+coulters,&#93; &#91;-pitchfork&#93;&#91;+and&#93; &#91;-or&#93;&#91;+for&#93; &#91;-an&#93;&#91;+the&#93; &#91;-axe,&#93;&#91;+forks,&#93; and &#91;-a&#93; &#91;-third&#93;&#91;+for&#93; &#91;-of&#93;&#91;+the&#93; &#91;-a&#93;&#91;+axes,&#93; &#91;-shekel&#93;&#91;+and&#93; &#91;-for&#93;&#91;+to&#93; &#91;-repointing&#93;&#91;+set&#93; &#91;-an&#93;&#91;+the&#93; &#91;-oxgoad.&#93;&#91;+goads.&#93;

Exact alternative editions: asv-1901. Closest alternatives: asv-1901. Alternative-edition token overlap: 100.00%.


### 1 Samuel 13:21 — BQ-DEV-2CB242707886D004-word_for_word (Partial)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: bsb-2025-third-printing.

Expected: The charge was a pim for sharpening a plowshare or mattock, a third of a shekel for sharpening a pitchfork or an axe, and a third of a shekel for repointing an oxgoad.

Produced: The charge was a pim for sharpening a plowshare or mattock, a third of a shekel and for the forks, and for the axes, and to set the goads.

Edits: The charge was a pim for sharpening a plowshare or mattock, a third of a shekel &#91;-for&#93; &#91;-sharpening&#93; &#91;-a&#93; &#91;-pitchfork&#93;&#91;+and&#93; &#91;-or&#93;&#91;+for&#93; &#91;-an&#93;&#91;+the&#93; &#91;-axe,&#93;&#91;+forks,&#93; and &#91;-a&#93; &#91;-third&#93;&#91;+for&#93; &#91;-of&#93;&#91;+the&#93; &#91;-a&#93;&#91;+axes,&#93; &#91;-shekel&#93;&#91;+and&#93; &#91;-for&#93;&#91;+to&#93; &#91;-repointing&#93;&#91;+set&#93; &#91;-an&#93;&#91;+the&#93; &#91;-oxgoad.&#93;&#91;+goads.&#93;

Exact alternative editions: . Closest alternatives: asv-1901. Alternative-edition token overlap: 100.00%.


### Luke 22:9 — BQ-DEV-30B7A7F315281191-word_for_word (Refusal)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: asv-1901.

Expected: And they said unto him, Where wilt thou that we make ready?

Produced: I cannot provide that passage.

Edits: &#91;-And&#93; &#91;-they&#93; &#91;-said&#93; &#91;-unto&#93; &#91;-him,&#93; &#91;-Where&#93;&#91;+I&#93; &#91;-wilt&#93;&#91;+cannot&#93; &#91;-thou&#93;&#91;+provide&#93; that &#91;-we&#93; &#91;-make&#93; &#91;-ready?&#93;&#91;+passage.&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: 0.00%.


### Luke 22:9 — BQ-DEV-30B7A7F315281191-word_for_word (ExtraneousText)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: asv-1901.

Expected: And they said unto him, Where wilt thou that we make ready?

Produced: Here is the passage: And they said unto him, Where wilt thou that we make ready?

Edits: &#91;+Here&#93; &#91;+is&#93; &#91;+the&#93; &#91;+passage:&#93; And they said unto him, Where wilt thou that we make ready?

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: 0.00%.


### 2 Kings 1:6 — BQ-DEV-31D278E968BBECF0-word_for_word (Empty)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: asv-1901.

Expected: And they said unto him, There came up a man to meet us, and said unto us, Go, turn again unto the king that sent you, and say unto him, Thus saith Jehovah, Is it because there is no God in Israel, that thou sendest to inquire of Baal-zebub, the god of Ekron? therefore thou shalt not come down from the bed whither thou art gone up, but shalt surely die.

Produced:

Edits: &#91;-And&#93; &#91;-they&#93; &#91;-said&#93; &#91;-unto&#93; &#91;-him,&#93; &#91;-There&#93; &#91;-came&#93; &#91;-up&#93; &#91;-a&#93; &#91;-man&#93; &#91;-to&#93; &#91;-meet&#93; &#91;-us,&#93; &#91;-and&#93; &#91;-said&#93; &#91;-unto&#93; &#91;-us,&#93; &#91;-Go,&#93; &#91;-turn&#93; &#91;-again&#93; &#91;-unto&#93; &#91;-the&#93; &#91;-king&#93; &#91;-that&#93; &#91;-sent&#93; &#91;-you,&#93; &#91;-and&#93; &#91;-say&#93; &#91;-unto&#93; &#91;-him,&#93; &#91;-Thus&#93; &#91;-saith&#93; &#91;-Jehovah,&#93; &#91;-Is&#93; &#91;-it&#93; &#91;-because&#93; &#91;-there&#93; &#91;-is&#93; &#91;-no&#93; &#91;-God&#93; &#91;-in&#93; &#91;-Israel,&#93; &#91;-that&#93; &#91;-thou&#93; &#91;-sendest&#93; &#91;-to&#93; &#91;-inquire&#93; &#91;-of&#93; &#91;-Baal-zebub,&#93; &#91;-the&#93; &#91;-god&#93; &#91;-of&#93; &#91;-Ekron?&#93; &#91;-therefore&#93; &#91;-thou&#93; &#91;-shalt&#93; &#91;-not&#93; &#91;-come&#93; &#91;-down&#93; &#91;-from&#93; &#91;-the&#93; &#91;-bed&#93; &#91;-whither&#93; &#91;-thou&#93; &#91;-art&#93; &#91;-gone&#93; &#91;-up,&#93; &#91;-but&#93; &#91;-shalt&#93; &#91;-surely&#93; &#91;-die.&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: undefined.
