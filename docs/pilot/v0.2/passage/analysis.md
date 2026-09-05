# BibleQuoteBench validated analysis

Evidence: **synthetic_fixture**. Track: **passage**.

- 95% percentile intervals use deterministic stratified reference-cluster bootstrap; translations and repetitions stay together. Intervals are conditional on these runs, not estimates of future provider drift.
- Pairwise intervals are exploratory and unadjusted for multiple comparisons. Small pilot strata can yield degenerate intervals; no superiority claim follows automatically.
- The overall score is the fixed benchmark mixture, not a population estimate for the whole Bible. Provider errors count as end-to-end failures and are excluded only from conditional recall.
- Exact alternative matches establish textual resemblance, not training provenance. Hidden references are not necessarily unseen training text.
- Matched requested temperature and token limits do not guarantee identical effective behavior across providers. A missing resolved model identifier is explicitly marked unresolved.

## openai_compatible / synthetic-a / synthetic-a / 42038f972648

3 complete repetitions; 36 cases each; 12 reference clusters.

ExactText: 40.74% [95% CI 20.37, 61.11]. ExactWords: 50.00% [95% CI 27.78, 71.30].

Provider errors: 8.33%. Recall given provider success: 44.44%.

### Descriptive breakdown

### Overall

| Responses | ExactText | ExactWords | Word accuracy | Refusals | Provider errors | Translation confusion |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 108 | 40.74% | 50.00% | 66.68% | 8.33% | 8.33% | 8.33% |

### Models

| Provider / model | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| openai_compatible / synthetic-a / synthetic-a | 108 | 40.74% | 50.00% | 66.68% | 8.33% |

### Translations

| Translation | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| asv-1901 | 36 | 41.67% | 50.00% | 67.06% | 8.33% |
| bsb-2025-third-printing | 36 | 41.67% | 50.00% | 66.10% | 8.33% |
| web-classic-2020 | 36 | 38.89% | 50.00% | 66.87% | 8.33% |

### Strata

| Stratum | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| passage | 108 | 40.74% | 50.00% | 66.68% | 8.33% |

### Requested → resembles

Counts use exact requested/other-edition matches first, then the closest alternative when it is strictly closer; remaining errors are `_unclassified`.

- **asv-1901**: _empty=3, _provider_error=3, _refusal=3, _unclassified=9, asv-1901=15, bsb-2025-third-printing=3
- **bsb-2025-third-printing**: _empty=3, _provider_error=3, _refusal=3, _unclassified=8, asv-1901=4, bsb-2025-third-printing=15
- **web-classic-2020**: _empty=3, _provider_error=3, _refusal=3, _unclassified=9, bsb-2025-third-printing=4, web-classic-2020=14

Exact other-edition matches (separate from approximate resemblance):

- asv-1901: bsb-2025-third-printing=3
- bsb-2025-third-printing: asv-1901=3
- web-classic-2020: bsb-2025-third-printing=3

### Stability

| Provider / model | Repeated cases | Output consistency | Exact recall |
| --- | ---: | ---: | ---: |
| openai_compatible / synthetic-a / synthetic-a | 36 | 60.19% | 44.91% |

Edition-differing cases: 108; ExactText 40.74%.

| Translation / stratum | Responses | ExactText |
| --- | ---: | ---: |
| asv-1901 / passage | 36 | 41.67% |
| bsb-2025-third-printing / passage | 36 | 41.67% |
| web-classic-2020 / passage | 36 | 38.89% |

## openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12

3 complete repetitions; 36 cases each; 12 reference clusters.

ExactText: 34.26% [95% CI 16.67, 54.63]. ExactWords: 44.44% [95% CI 25.93, 65.74].

Provider errors: 8.33%. Recall given provider success: 37.37%.

### Descriptive breakdown

### Overall

| Responses | ExactText | ExactWords | Word accuracy | Refusals | Provider errors | Translation confusion |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 108 | 34.26% | 44.44% | 65.51% | 9.26% | 8.33% | 12.96% |

### Models

| Provider / model | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| openai_compatible / synthetic-b / synthetic-b | 108 | 34.26% | 44.44% | 65.51% | 12.96% |

### Translations

| Translation | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| asv-1901 | 36 | 33.33% | 44.44% | 66.29% | 13.89% |
| bsb-2025-third-printing | 36 | 36.11% | 47.22% | 66.09% | 11.11% |
| web-classic-2020 | 36 | 33.33% | 41.67% | 64.14% | 13.89% |

### Strata

| Stratum | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| passage | 108 | 34.26% | 44.44% | 65.51% | 12.96% |

### Requested → resembles

Counts use exact requested/other-edition matches first, then the closest alternative when it is strictly closer; remaining errors are `_unclassified`.

- **asv-1901**: _empty=3, _provider_error=3, _refusal=3, _unclassified=8, asv-1901=12, bsb-2025-third-printing=7
- **bsb-2025-third-printing**: _empty=3, _provider_error=3, _refusal=3, _unclassified=9, asv-1901=5, bsb-2025-third-printing=13
- **web-classic-2020**: _empty=3, _provider_error=3, _refusal=4, _unclassified=9, bsb-2025-third-printing=5, web-classic-2020=12

Exact other-edition matches (separate from approximate resemblance):

- asv-1901: bsb-2025-third-printing=5
- bsb-2025-third-printing: asv-1901=4
- web-classic-2020: bsb-2025-third-printing=5

### Stability

| Provider / model | Repeated cases | Output consistency | Exact recall |
| --- | ---: | ---: | ---: |
| openai_compatible / synthetic-b / synthetic-b | 36 | 57.41% | 38.43% |

Edition-differing cases: 108; ExactText 34.26%.

| Translation / stratum | Responses | ExactText |
| --- | ---: | ---: |
| asv-1901 / passage | 36 | 33.33% |
| bsb-2025-third-printing / passage | 36 | 36.11% |
| web-classic-2020 / passage | 36 | 33.33% |

## Paired comparisons

Differences are left minus right, in percentage points.

- openai_compatible / synthetic-a / synthetic-a / 42038f972648 versus openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12: 6.48% [95% CI -25.00, 37.04]

## Annotated failures

Whitespace-token display diff; scoring uses the normative Unicode tokenizer. Deletions use [-word], additions [+word]. Examples select the first case per diagnostic category deterministically; they are illustrative, not prevalence estimates.

### 2 Chronicles 15:6-8 — BQ-DEV-0A8391B4C89643C7-passage (ProviderError)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: web-classic-2020.

Expected: They were broken in pieces, nation against nation, and city against city; for God troubled them with all adversity. But you be strong! Don’t let your hands be slack, for your work will be rewarded.” When Asa heard these words and the prophecy of Oded the prophet, he took courage, and put away the abominations out of all the land of Judah and Benjamin, and out of the cities which he had taken from the hill country of Ephraim; and he renewed Yahweh’s altar that was before Yahweh’s porch.

Produced:

Edits: &#91;-They&#93; &#91;-were&#93; &#91;-broken&#93; &#91;-in&#93; &#91;-pieces,&#93; &#91;-nation&#93; &#91;-against&#93; &#91;-nation,&#93; &#91;-and&#93; &#91;-city&#93; &#91;-against&#93; &#91;-city;&#93; &#91;-for&#93; &#91;-God&#93; &#91;-troubled&#93; &#91;-them&#93; &#91;-with&#93; &#91;-all&#93; &#91;-adversity.&#93; &#91;-But&#93; &#91;-you&#93; &#91;-be&#93; &#91;-strong!&#93; &#91;-Don’t&#93; &#91;-let&#93; &#91;-your&#93; &#91;-hands&#93; &#91;-be&#93; &#91;-slack,&#93; &#91;-for&#93; &#91;-your&#93; &#91;-work&#93; &#91;-will&#93; &#91;-be&#93; &#91;-rewarded.”&#93; &#91;-When&#93; &#91;-Asa&#93; &#91;-heard&#93; &#91;-these&#93; &#91;-words&#93; &#91;-and&#93; &#91;-the&#93; &#91;-prophecy&#93; &#91;-of&#93; &#91;-Oded&#93; &#91;-the&#93; &#91;-prophet,&#93; &#91;-he&#93; &#91;-took&#93; &#91;-courage,&#93; &#91;-and&#93; &#91;-put&#93; &#91;-away&#93; &#91;-the&#93; &#91;-abominations&#93; &#91;-out&#93; &#91;-of&#93; &#91;-all&#93; &#91;-the&#93; &#91;-land&#93; &#91;-of&#93; &#91;-Judah&#93; &#91;-and&#93; &#91;-Benjamin,&#93; &#91;-and&#93; &#91;-out&#93; &#91;-of&#93; &#91;-the&#93; &#91;-cities&#93; &#91;-which&#93; &#91;-he&#93; &#91;-had&#93; &#91;-taken&#93; &#91;-from&#93; &#91;-the&#93; &#91;-hill&#93; &#91;-country&#93; &#91;-of&#93; &#91;-Ephraim;&#93; &#91;-and&#93; &#91;-he&#93; &#91;-renewed&#93; &#91;-Yahweh’s&#93; &#91;-altar&#93; &#91;-that&#93; &#91;-was&#93; &#91;-before&#93; &#91;-Yahweh’s&#93; &#91;-porch.&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: undefined.


### 1 Samuel 17:4-6 — BQ-DEV-1015B25C5B9ECAA0-passage (TranslationConfusion)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: bsb-2025-third-printing.

Expected: Then a champion named Goliath, who was from Gath, came out from the Philistine camp. He was six cubits and a span in height, and he had a bronze helmet on his head. He wore a bronze coat of mail weighing five thousand shekels, and he had armor of bronze on his legs and a javelin of bronze slung between his shoulders.

Produced: And there went out a champion out of the camp of the Philistines, named Goliath, of Gath, whose height was six cubits and a span. And he had a helmet of brass upon his head, and he was clad with a coat of mail; and the weight of the coat was five thousand shekels of brass. And he had greaves of brass upon his legs, and a javelin of brass between his shoulders.

Edits: &#91;+And&#93; &#91;+there&#93; &#91;+went&#93; &#91;-Then&#93;&#91;+out&#93; a champion &#91;-named&#93;&#91;+out&#93; &#91;-Goliath,&#93;&#91;+of&#93; &#91;-who&#93;&#91;+the&#93; &#91;-was&#93;&#91;+camp&#93; &#91;-from&#93;&#91;+of&#93; &#91;-Gath,&#93;&#91;+the&#93; &#91;-came&#93;&#91;+Philistines,&#93; &#91;-out&#93;&#91;+named&#93; &#91;-from&#93;&#91;+Goliath,&#93; &#91;-the&#93;&#91;+of&#93; &#91;-Philistine&#93;&#91;+Gath,&#93; &#91;-camp.&#93;&#91;+whose&#93; &#91;-He&#93;&#91;+height&#93; was six cubits and a &#91;-span&#93; &#91;-in&#93; &#91;-height,&#93;&#91;+span.&#93; &#91;-and&#93;&#91;+And&#93; he had a &#91;+helmet&#93; &#91;-bronze&#93;&#91;+of&#93; &#91;-helmet&#93;&#91;+brass&#93; &#91;-on&#93;&#91;+upon&#93; his &#91;+head,&#93; &#91;+and&#93; &#91;+he&#93; &#91;-head.&#93;&#91;+was&#93; &#91;-He&#93;&#91;+clad&#93; &#91;-wore&#93;&#91;+with&#93; a &#91;+coat&#93; &#91;+of&#93; &#91;+mail;&#93; &#91;+and&#93; &#91;-bronze&#93;&#91;+the&#93; &#91;-coat&#93;&#91;+weight&#93; of &#91;+the&#93; &#91;-mail&#93;&#91;+coat&#93; &#91;-weighing&#93;&#91;+was&#93; five thousand &#91;+shekels&#93; &#91;+of&#93; &#91;-shekels,&#93;&#91;+brass.&#93; &#91;-and&#93;&#91;+And&#93; he had &#91;-armor&#93;&#91;+greaves&#93; of &#91;-bronze&#93;&#91;+brass&#93; &#91;-on&#93;&#91;+upon&#93; his &#91;-legs&#93;&#91;+legs,&#93; and a javelin of &#91;-bronze&#93; &#91;-slung&#93;&#91;+brass&#93; between his shoulders.

Exact alternative editions: asv-1901. Closest alternatives: asv-1901. Alternative-edition token overlap: 100.00%.


### 1 Samuel 17:4-6 — BQ-DEV-1015B25C5B9ECAA0-passage (Partial)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: bsb-2025-third-printing.

Expected: Then a champion named Goliath, who was from Gath, came out from the Philistine camp. He was six cubits and a span in height, and he had a bronze helmet on his head. He wore a bronze coat of mail weighing five thousand shekels, and he had armor of bronze on his legs and a javelin of bronze slung between his shoulders.

Produced: Then a champion named Goliath, who was from Gath, came out from the Philistine camp. He was six cubits and a span in height, and he had a bronze helmet on he was clad with a coat of mail; and the weight of the coat was five thousand shekels of brass. And he had greaves of brass upon his legs, and a javelin of brass between his shoulders.

Edits: Then a champion named Goliath, who was from Gath, came out from the Philistine camp. He was six cubits and a span in height, and he had a bronze helmet on &#91;-his&#93;&#91;+he&#93; &#91;-head.&#93;&#91;+was&#93; &#91;-He&#93;&#91;+clad&#93; &#91;-wore&#93;&#91;+with&#93; a &#91;+coat&#93; &#91;+of&#93; &#91;+mail;&#93; &#91;+and&#93; &#91;-bronze&#93;&#91;+the&#93; &#91;-coat&#93;&#91;+weight&#93; of &#91;+the&#93; &#91;-mail&#93;&#91;+coat&#93; &#91;-weighing&#93;&#91;+was&#93; five thousand &#91;+shekels&#93; &#91;+of&#93; &#91;-shekels,&#93;&#91;+brass.&#93; &#91;-and&#93;&#91;+And&#93; he had &#91;-armor&#93;&#91;+greaves&#93; of &#91;-bronze&#93;&#91;+brass&#93; &#91;-on&#93;&#91;+upon&#93; his &#91;-legs&#93;&#91;+legs,&#93; and a javelin of &#91;-bronze&#93; &#91;-slung&#93;&#91;+brass&#93; between his shoulders.

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: 100.00%.


### 1 Samuel 17:4-6 — BQ-DEV-1015B25C5B9ECAA0-passage (Refusal)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: bsb-2025-third-printing.

Expected: Then a champion named Goliath, who was from Gath, came out from the Philistine camp. He was six cubits and a span in height, and he had a bronze helmet on his head. He wore a bronze coat of mail weighing five thousand shekels, and he had armor of bronze on his legs and a javelin of bronze slung between his shoulders.

Produced: I cannot provide that passage.

Edits: &#91;-Then&#93; &#91;-a&#93; &#91;-champion&#93; &#91;-named&#93; &#91;-Goliath,&#93; &#91;-who&#93; &#91;-was&#93; &#91;-from&#93; &#91;-Gath,&#93; &#91;-came&#93; &#91;-out&#93; &#91;-from&#93; &#91;-the&#93; &#91;-Philistine&#93; &#91;-camp.&#93; &#91;-He&#93; &#91;-was&#93; &#91;-six&#93; &#91;-cubits&#93; &#91;-and&#93; &#91;-a&#93; &#91;-span&#93; &#91;-in&#93; &#91;-height,&#93; &#91;-and&#93; &#91;-he&#93; &#91;-had&#93; &#91;-a&#93; &#91;-bronze&#93; &#91;-helmet&#93; &#91;-on&#93; &#91;-his&#93; &#91;-head.&#93; &#91;-He&#93; &#91;-wore&#93; &#91;-a&#93; &#91;-bronze&#93; &#91;-coat&#93; &#91;-of&#93; &#91;-mail&#93; &#91;-weighing&#93; &#91;-five&#93; &#91;-thousand&#93; &#91;-shekels,&#93; &#91;-and&#93; &#91;-he&#93; &#91;-had&#93; &#91;-armor&#93; &#91;-of&#93; &#91;-bronze&#93; &#91;-on&#93; &#91;-his&#93; &#91;-legs&#93; &#91;-and&#93; &#91;-a&#93; &#91;-javelin&#93; &#91;-of&#93; &#91;-bronze&#93;&#91;+I&#93; &#91;-slung&#93;&#91;+cannot&#93; &#91;-between&#93;&#91;+provide&#93; &#91;-his&#93;&#91;+that&#93; &#91;-shoulders.&#93;&#91;+passage.&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: 0.00%.


### 1 Samuel 13:21-23 — BQ-DEV-2CB242707886D004-passage (Partial)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: bsb-2025-third-printing.

Expected: The charge was a pim for sharpening a plowshare or mattock, a third of a shekel for sharpening a pitchfork or an axe, and a third of a shekel for repointing an oxgoad. So on the day of battle not a sword or spear could be found in the hands of the troops with Saul and Jonathan; only Saul and his son Jonathan had weapons. And a garrison of the Philistines had gone out to the pass at Michmash.

Produced: The charge was a pim for sharpening a plowshare or mattock a third of a shekel for sharpening a pitchfork or an axe and a third of a shekel for repointing an oxgoad So on the day of battle not a sword or spear could be found in the hands of the troops with Saul and Jonathan only Saul and his son Jonathan had weapons And a garrison of the Philistines had gone out to the pass at Michmash

Edits: The charge was a pim for sharpening a plowshare or &#91;-mattock,&#93;&#91;+mattock&#93; a third of a shekel for sharpening a pitchfork or an &#91;-axe,&#93;&#91;+axe&#93; and a third of a shekel for repointing an &#91;-oxgoad.&#93;&#91;+oxgoad&#93; So on the day of battle not a sword or spear could be found in the hands of the troops with Saul and &#91;-Jonathan;&#93;&#91;+Jonathan&#93; only Saul and his son Jonathan had &#91;-weapons.&#93;&#91;+weapons&#93; And a garrison of the Philistines had gone out to the pass at &#91;-Michmash.&#93;&#91;+Michmash&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: undefined.


### 1 Kings 18:13-15 — BQ-DEV-3C3391E361BAC905-passage (ExtraneousText)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: bsb-2025-third-printing.

Expected: Was it not reported to my lord what I did when Jezebel slaughtered the prophets of the LORD? I hid a hundred prophets of the LORD, fifty men per cave, and I provided them with food and water. And now you say, ‘Go tell your lord that Elijah is here!’ He will kill me!” Then Elijah said, “As surely as the LORD of Hosts lives, before whom I stand, I will present myself to Ahab today.”

Produced: Here is the passage: Was it not reported to my lord what I did when Jezebel slaughtered the prophets of the LORD? I hid a hundred prophets of the LORD, fifty men per cave, and I provided them with food and water. And now you say, ‘Go tell your lord that Elijah is here!’ He will kill me!” Then Elijah said, “As surely as the LORD of Hosts lives, before whom I stand, I will present myself to Ahab today.”

Edits: &#91;+Here&#93; &#91;+is&#93; &#91;+the&#93; &#91;+passage:&#93; Was it not reported to my lord what I did when Jezebel slaughtered the prophets of the LORD? I hid a hundred prophets of the LORD, fifty men per cave, and I provided them with food and water. And now you say, ‘Go tell your lord that Elijah is here!’ He will kill me!” Then Elijah said, “As surely as the LORD of Hosts lives, before whom I stand, I will present myself to Ahab today.”

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: 0.00%.


### 1 Kings 18:13-15 — BQ-DEV-3C3391E361BAC905-passage (Empty)

Model: openai_compatible / synthetic-a / synthetic-a / 42038f972648. Requested edition: bsb-2025-third-printing.

Expected: Was it not reported to my lord what I did when Jezebel slaughtered the prophets of the LORD? I hid a hundred prophets of the LORD, fifty men per cave, and I provided them with food and water. And now you say, ‘Go tell your lord that Elijah is here!’ He will kill me!” Then Elijah said, “As surely as the LORD of Hosts lives, before whom I stand, I will present myself to Ahab today.”

Produced:

Edits: &#91;-Was&#93; &#91;-it&#93; &#91;-not&#93; &#91;-reported&#93; &#91;-to&#93; &#91;-my&#93; &#91;-lord&#93; &#91;-what&#93; &#91;-I&#93; &#91;-did&#93; &#91;-when&#93; &#91;-Jezebel&#93; &#91;-slaughtered&#93; &#91;-the&#93; &#91;-prophets&#93; &#91;-of&#93; &#91;-the&#93; &#91;-LORD?&#93; &#91;-I&#93; &#91;-hid&#93; &#91;-a&#93; &#91;-hundred&#93; &#91;-prophets&#93; &#91;-of&#93; &#91;-the&#93; &#91;-LORD,&#93; &#91;-fifty&#93; &#91;-men&#93; &#91;-per&#93; &#91;-cave,&#93; &#91;-and&#93; &#91;-I&#93; &#91;-provided&#93; &#91;-them&#93; &#91;-with&#93; &#91;-food&#93; &#91;-and&#93; &#91;-water.&#93; &#91;-And&#93; &#91;-now&#93; &#91;-you&#93; &#91;-say,&#93; &#91;-‘Go&#93; &#91;-tell&#93; &#91;-your&#93; &#91;-lord&#93; &#91;-that&#93; &#91;-Elijah&#93; &#91;-is&#93; &#91;-here!’&#93; &#91;-He&#93; &#91;-will&#93; &#91;-kill&#93; &#91;-me!”&#93; &#91;-Then&#93; &#91;-Elijah&#93; &#91;-said,&#93; &#91;-“As&#93; &#91;-surely&#93; &#91;-as&#93; &#91;-the&#93; &#91;-LORD&#93; &#91;-of&#93; &#91;-Hosts&#93; &#91;-lives,&#93; &#91;-before&#93; &#91;-whom&#93; &#91;-I&#93; &#91;-stand,&#93; &#91;-I&#93; &#91;-will&#93; &#91;-present&#93; &#91;-myself&#93; &#91;-to&#93; &#91;-Ahab&#93; &#91;-today.”&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: undefined.


### 1 Kings 8:26-28 — BQ-DEV-0590F03501241002-passage (Partial)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: asv-1901.

Expected: Now therefore, O God of Israel, let thy word, I pray thee, be verified, which thou spakest unto thy servant David my father. But will God in very deed dwell on the earth? behold, heaven and the heaven of heavens cannot contain thee; how much less this house that I have builded! Yet have thou respect unto the prayer of thy servant, and to his supplication, O Jehovah my God, to hearken unto the cry and to the prayer which thy servant prayeth before thee this day;

Produced: Now therefore O God of Israel let thy word I pray thee be verified which thou spakest unto thy servant David my father But will God in very deed dwell on the earth behold heaven and the heaven of heavens cannot contain thee how much less this house that I have builded Yet have thou respect unto the prayer of thy servant and to his supplication O Jehovah my God to hearken unto the cry and to the prayer which thy servant prayeth before thee this day

Edits: Now &#91;-therefore,&#93;&#91;+therefore&#93; O God of &#91;-Israel,&#93;&#91;+Israel&#93; let thy &#91;-word,&#93;&#91;+word&#93; I pray &#91;-thee,&#93;&#91;+thee&#93; be &#91;-verified,&#93;&#91;+verified&#93; which thou spakest unto thy servant David my &#91;-father.&#93;&#91;+father&#93; But will God in very deed dwell on the &#91;-earth?&#93;&#91;+earth&#93; &#91;-behold,&#93;&#91;+behold&#93; heaven and the heaven of heavens cannot contain &#91;-thee;&#93;&#91;+thee&#93; how much less this house that I have &#91;-builded!&#93;&#91;+builded&#93; Yet have thou respect unto the prayer of thy &#91;-servant,&#93;&#91;+servant&#93; and to his &#91;-supplication,&#93;&#91;+supplication&#93; O Jehovah my &#91;-God,&#93;&#91;+God&#93; to hearken unto the cry and to the prayer which thy servant prayeth before thee this &#91;-day;&#93;&#91;+day&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: undefined.


### 1 Kings 8:26-28 — BQ-DEV-0590F03501241002-passage (TranslationConfusion)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: asv-1901.

Expected: Now therefore, O God of Israel, let thy word, I pray thee, be verified, which thou spakest unto thy servant David my father. But will God in very deed dwell on the earth? behold, heaven and the heaven of heavens cannot contain thee; how much less this house that I have builded! Yet have thou respect unto the prayer of thy servant, and to his supplication, O Jehovah my God, to hearken unto the cry and to the prayer which thy servant prayeth before thee this day;

Produced: And now, O God of Israel, please confirm what You promised to Your servant, my father David. But will God indeed dwell upon the earth? The heavens, even the highest heavens, cannot contain You, much less this temple I have built. Yet regard the prayer and plea of Your servant, O LORD my God, so that You may hear the cry and the prayer that Your servant is praying before You today.

Edits: &#91;-Now&#93;&#91;+And&#93; &#91;-therefore,&#93;&#91;+now,&#93; O God of Israel, &#91;-let&#93; &#91;-thy&#93; &#91;-word,&#93; &#91;-I&#93; &#91;-pray&#93; &#91;-thee,&#93; &#91;-be&#93;&#91;+please&#93; &#91;-verified,&#93;&#91;+confirm&#93; &#91;-which&#93;&#91;+what&#93; &#91;-thou&#93;&#91;+You&#93; &#91;-spakest&#93;&#91;+promised&#93; &#91;-unto&#93;&#91;+to&#93; &#91;-thy&#93;&#91;+Your&#93; &#91;-servant&#93;&#91;+servant,&#93; &#91;-David&#93;&#91;+my&#93; &#91;-my&#93;&#91;+father&#93; &#91;-father.&#93;&#91;+David.&#93; But will God &#91;-in&#93; &#91;-very&#93; &#91;-deed&#93;&#91;+indeed&#93; dwell &#91;-on&#93;&#91;+upon&#93; the earth? &#91;-behold,&#93;&#91;+The&#93; &#91;-heaven&#93;&#91;+heavens,&#93; &#91;-and&#93;&#91;+even&#93; the &#91;-heaven&#93; &#91;-of&#93;&#91;+highest&#93; &#91;-heavens&#93;&#91;+heavens,&#93; cannot contain &#91;-thee;&#93; &#91;-how&#93;&#91;+You,&#93; much less this &#91;-house&#93; &#91;-that&#93;&#91;+temple&#93; I have &#91;-builded!&#93;&#91;+built.&#93; Yet &#91;-have&#93; &#91;-thou&#93; &#91;-respect&#93; &#91;-unto&#93;&#91;+regard&#93; the prayer &#91;-of&#93; &#91;-thy&#93; &#91;-servant,&#93;&#91;+and&#93; &#91;-and&#93;&#91;+plea&#93; &#91;-to&#93;&#91;+of&#93; &#91;-his&#93;&#91;+Your&#93; &#91;-supplication,&#93;&#91;+servant,&#93; O &#91;-Jehovah&#93;&#91;+LORD&#93; my God, &#91;+so&#93; &#91;+that&#93; &#91;-to&#93;&#91;+You&#93; &#91;-hearken&#93;&#91;+may&#93; &#91;-unto&#93;&#91;+hear&#93; the cry and &#91;-to&#93; the prayer &#91;-which&#93;&#91;+that&#93; &#91;-thy&#93;&#91;+Your&#93; servant &#91;-prayeth&#93;&#91;+is&#93; &#91;-before&#93;&#91;+praying&#93; &#91;-thee&#93;&#91;+before&#93; &#91;-this&#93;&#91;+You&#93; &#91;-day;&#93;&#91;+today.&#93;

Exact alternative editions: bsb-2025-third-printing. Closest alternatives: bsb-2025-third-printing. Alternative-edition token overlap: 100.00%.


### 1 Chronicles 9:33-35 — BQ-DEV-0A65F718E801DFAE-passage (Partial)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: bsb-2025-third-printing.

Expected: Those who were musicians, the heads of Levite families, stayed in the temple chambers and were exempt from other duties because they were on duty day and night. All these were heads of Levite families, chiefs according to their genealogies, and they lived in Jerusalem. Jeiel the father of Gibeon lived in Gibeon. His wife’s name was Maacah.

Produced: Those who were musicians, the heads of Levite families, stayed in the temple chambers and were exempt from other duties because they were on duty day and night. All night. These were heads of fathers’ houses of the Levites, throughout their generations, chief men: these dwelt at Jerusalem. And in Gibeon there dwelt the father of Gibeon, Jeiel, whose wife’s name was Maacah;

Edits: Those who were musicians, the heads of Levite families, stayed in the temple chambers and were exempt from other duties because they were on duty day and night. All &#91;+night.&#93; &#91;-these&#93;&#91;+These&#93; were heads of &#91;+fathers’&#93; &#91;-Levite&#93;&#91;+houses&#93; &#91;-families,&#93;&#91;+of&#93; &#91;-chiefs&#93;&#91;+the&#93; &#91;-according&#93;&#91;+Levites,&#93; &#91;-to&#93;&#91;+throughout&#93; their &#91;+generations,&#93; &#91;+chief&#93; &#91;+men:&#93; &#91;+these&#93; &#91;-genealogies,&#93;&#91;+dwelt&#93; &#91;-and&#93;&#91;+at&#93; &#91;-they&#93;&#91;+Jerusalem.&#93; &#91;-lived&#93;&#91;+And&#93; in &#91;+Gibeon&#93; &#91;-Jerusalem.&#93;&#91;+there&#93; &#91;-Jeiel&#93;&#91;+dwelt&#93; the father of &#91;-Gibeon&#93; &#91;-lived&#93; &#91;-in&#93;&#91;+Gibeon,&#93; &#91;-Gibeon.&#93;&#91;+Jeiel,&#93; &#91;-His&#93;&#91;+whose&#93; wife’s name was &#91;-Maacah.&#93;&#91;+Maacah;&#93;

Exact alternative editions: . Closest alternatives: asv-1901. Alternative-edition token overlap: 100.00%.


### 1 Samuel 17:4-6 — BQ-DEV-1015B25C5B9ECAA0-passage (ExtraneousText)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: bsb-2025-third-printing.

Expected: Then a champion named Goliath, who was from Gath, came out from the Philistine camp. He was six cubits and a span in height, and he had a bronze helmet on his head. He wore a bronze coat of mail weighing five thousand shekels, and he had armor of bronze on his legs and a javelin of bronze slung between his shoulders.

Produced: Here is the passage: Then a champion named Goliath, who was from Gath, came out from the Philistine camp. He was six cubits and a span in height, and he had a bronze helmet on his head. He wore a bronze coat of mail weighing five thousand shekels, and he had armor of bronze on his legs and a javelin of bronze slung between his shoulders.

Edits: &#91;+Here&#93; &#91;+is&#93; &#91;+the&#93; &#91;+passage:&#93; Then a champion named Goliath, who was from Gath, came out from the Philistine camp. He was six cubits and a span in height, and he had a bronze helmet on his head. He wore a bronze coat of mail weighing five thousand shekels, and he had armor of bronze on his legs and a javelin of bronze slung between his shoulders.

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: 25.00%.


### 1 Samuel 17:4-6 — BQ-DEV-1015B25C5B9ECAA0-passage (Empty)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: bsb-2025-third-printing.

Expected: Then a champion named Goliath, who was from Gath, came out from the Philistine camp. He was six cubits and a span in height, and he had a bronze helmet on his head. He wore a bronze coat of mail weighing five thousand shekels, and he had armor of bronze on his legs and a javelin of bronze slung between his shoulders.

Produced:

Edits: &#91;-Then&#93; &#91;-a&#93; &#91;-champion&#93; &#91;-named&#93; &#91;-Goliath,&#93; &#91;-who&#93; &#91;-was&#93; &#91;-from&#93; &#91;-Gath,&#93; &#91;-came&#93; &#91;-out&#93; &#91;-from&#93; &#91;-the&#93; &#91;-Philistine&#93; &#91;-camp.&#93; &#91;-He&#93; &#91;-was&#93; &#91;-six&#93; &#91;-cubits&#93; &#91;-and&#93; &#91;-a&#93; &#91;-span&#93; &#91;-in&#93; &#91;-height,&#93; &#91;-and&#93; &#91;-he&#93; &#91;-had&#93; &#91;-a&#93; &#91;-bronze&#93; &#91;-helmet&#93; &#91;-on&#93; &#91;-his&#93; &#91;-head.&#93; &#91;-He&#93; &#91;-wore&#93; &#91;-a&#93; &#91;-bronze&#93; &#91;-coat&#93; &#91;-of&#93; &#91;-mail&#93; &#91;-weighing&#93; &#91;-five&#93; &#91;-thousand&#93; &#91;-shekels,&#93; &#91;-and&#93; &#91;-he&#93; &#91;-had&#93; &#91;-armor&#93; &#91;-of&#93; &#91;-bronze&#93; &#91;-on&#93; &#91;-his&#93; &#91;-legs&#93; &#91;-and&#93; &#91;-a&#93; &#91;-javelin&#93; &#91;-of&#93; &#91;-bronze&#93; &#91;-slung&#93; &#91;-between&#93; &#91;-his&#93; &#91;-shoulders.&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: undefined.


### 1 Samuel 17:4-6 — BQ-DEV-1015B25C5B9ECAA0-passage (ProviderError)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: bsb-2025-third-printing.

Expected: Then a champion named Goliath, who was from Gath, came out from the Philistine camp. He was six cubits and a span in height, and he had a bronze helmet on his head. He wore a bronze coat of mail weighing five thousand shekels, and he had armor of bronze on his legs and a javelin of bronze slung between his shoulders.

Produced:

Edits: &#91;-Then&#93; &#91;-a&#93; &#91;-champion&#93; &#91;-named&#93; &#91;-Goliath,&#93; &#91;-who&#93; &#91;-was&#93; &#91;-from&#93; &#91;-Gath,&#93; &#91;-came&#93; &#91;-out&#93; &#91;-from&#93; &#91;-the&#93; &#91;-Philistine&#93; &#91;-camp.&#93; &#91;-He&#93; &#91;-was&#93; &#91;-six&#93; &#91;-cubits&#93; &#91;-and&#93; &#91;-a&#93; &#91;-span&#93; &#91;-in&#93; &#91;-height,&#93; &#91;-and&#93; &#91;-he&#93; &#91;-had&#93; &#91;-a&#93; &#91;-bronze&#93; &#91;-helmet&#93; &#91;-on&#93; &#91;-his&#93; &#91;-head.&#93; &#91;-He&#93; &#91;-wore&#93; &#91;-a&#93; &#91;-bronze&#93; &#91;-coat&#93; &#91;-of&#93; &#91;-mail&#93; &#91;-weighing&#93; &#91;-five&#93; &#91;-thousand&#93; &#91;-shekels,&#93; &#91;-and&#93; &#91;-he&#93; &#91;-had&#93; &#91;-armor&#93; &#91;-of&#93; &#91;-bronze&#93; &#91;-on&#93; &#91;-his&#93; &#91;-legs&#93; &#91;-and&#93; &#91;-a&#93; &#91;-javelin&#93; &#91;-of&#93; &#91;-bronze&#93; &#91;-slung&#93; &#91;-between&#93; &#91;-his&#93; &#91;-shoulders.&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: undefined.


### 1 Samuel 13:21-23 — BQ-DEV-2CB242707886D004-passage (Refusal)

Model: openai_compatible / synthetic-b / synthetic-b / bfde5e8bee12. Requested edition: bsb-2025-third-printing.

Expected: The charge was a pim for sharpening a plowshare or mattock, a third of a shekel for sharpening a pitchfork or an axe, and a third of a shekel for repointing an oxgoad. So on the day of battle not a sword or spear could be found in the hands of the troops with Saul and Jonathan; only Saul and his son Jonathan had weapons. And a garrison of the Philistines had gone out to the pass at Michmash.

Produced: I cannot provide that passage.

Edits: &#91;-The&#93; &#91;-charge&#93; &#91;-was&#93; &#91;-a&#93; &#91;-pim&#93; &#91;-for&#93; &#91;-sharpening&#93; &#91;-a&#93; &#91;-plowshare&#93; &#91;-or&#93; &#91;-mattock,&#93; &#91;-a&#93; &#91;-third&#93; &#91;-of&#93; &#91;-a&#93; &#91;-shekel&#93; &#91;-for&#93; &#91;-sharpening&#93; &#91;-a&#93; &#91;-pitchfork&#93; &#91;-or&#93; &#91;-an&#93; &#91;-axe,&#93; &#91;-and&#93; &#91;-a&#93; &#91;-third&#93; &#91;-of&#93; &#91;-a&#93; &#91;-shekel&#93; &#91;-for&#93; &#91;-repointing&#93; &#91;-an&#93; &#91;-oxgoad.&#93; &#91;-So&#93; &#91;-on&#93; &#91;-the&#93; &#91;-day&#93; &#91;-of&#93; &#91;-battle&#93; &#91;-not&#93; &#91;-a&#93; &#91;-sword&#93; &#91;-or&#93; &#91;-spear&#93; &#91;-could&#93; &#91;-be&#93; &#91;-found&#93; &#91;-in&#93; &#91;-the&#93; &#91;-hands&#93; &#91;-of&#93; &#91;-the&#93; &#91;-troops&#93; &#91;-with&#93; &#91;-Saul&#93; &#91;-and&#93; &#91;-Jonathan;&#93; &#91;-only&#93; &#91;-Saul&#93; &#91;-and&#93; &#91;-his&#93; &#91;-son&#93; &#91;-Jonathan&#93; &#91;-had&#93; &#91;-weapons.&#93; &#91;-And&#93; &#91;-a&#93; &#91;-garrison&#93; &#91;-of&#93; &#91;-the&#93; &#91;-Philistines&#93; &#91;-had&#93; &#91;-gone&#93; &#91;-out&#93; &#91;-to&#93;&#91;+I&#93; &#91;-the&#93;&#91;+cannot&#93; &#91;-pass&#93;&#91;+provide&#93; &#91;-at&#93;&#91;+that&#93; &#91;-Michmash.&#93;&#91;+passage.&#93;

Exact alternative editions: . Closest alternatives: . Alternative-edition token overlap: 20.00%.
