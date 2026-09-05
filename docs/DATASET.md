# Dataset construction

BibleQuoteBench v0.2 uses 500 shared single-verse references across BSB, ASV,
and WEB: 100 public development references (300 cases), and 400 privately
selected evaluation references (1,200 cases). This is a fixed benchmark mixture,
not a prevalence-weighted estimate for all Bible verses.

| Stratum | Total references | Public | Hidden |
| --- | ---: | ---: | ---: |
| Editorial famous/well-known pool | 20 | 4 | 16 |
| Translation-sensitive candidate pool | 50 | 10 | 40 |
| Short candidate pool | 25 | 5 | 20 |
| Long candidate pool | 25 | 5 | 20 |
| Remaining random references | 380 | 76 | 304 |

The curated pool retains the original 50 editorially chosen references. Sampling
20 instead of taking all 50 leaves uncertainty about which remaining references
are hidden. The former 30 famous slots move to random selection. These labels
remain editorial judgments; they do not claim measured quotation frequency.

Selection proceeds independently for public and hidden partitions, with quotas
allocated first. Public cases use the published seed. Their references are then
removed before the hidden partition uses a private seed. Within each partition:

1. Famous references are sampled from the curated pool, which must have at least
   twice the partition's selected quota.
2. Eligible translation-sensitive references are ranked by mean pairwise
   normalized whitespace-token edit distance. Selection is hash-randomized within
   the top five times the quota, rather than taking the top-ranked verses directly.
3. Short and long references use the same top-five-times-quota pool policy,
   ranked by mean whitespace-token length across translations. Already selected
   references are excluded at each step. Available pools must be at least twice
   the selected quota.
4. Remaining random slots use hash-ranked selection from unselected references.

Stable ordering and domain-separated SHA-256 rankings make selection independent
of input ordering and hash-map iteration. Tests establish that changing the
private seed changes membership in every hidden stratum without changing public
cases. Public candidate pools are still known: this reduces targeted tuning
opportunities but does not make the underlying scripture unseen training data.

## Provenance and reproducibility

Source archives are not committed. Archive bytes, USFM members, and imported
corpora have SHA-256 locks in `data/locks`; reconstruction rejects changed source
or corpus bytes. The v0.2 release manifest records the selection policy, counts,
source-corpus hashes, public case digest, and hidden case-file commitment.

Run `./scripts/build-dataset.ps1` twice. The retained ignored secret at
`data/hidden/sampling-secret.txt` must produce the same hidden commitment. Back
that secret up securely. A fresh seed intentionally makes a different hidden set;
it must not be presented as the committed evaluation release. The public manifest
commits to the ordered case file, not to a claim of training-data cleanliness.

The v0.1 public cases, references, catalog, and example responses are preserved in
`data/releases/v0.1/dev`. Its original release manifest and sampling configuration
remain available. Scores from the two dataset versions must not be pooled.
Schema-1 sampling retains its original selection algorithm for reconstruction.

## Diagnostic tracks

`prepare-pilot` selects public anchors round-robin across observed strata and
creates separate canonical, concise, word-for-word, and copy-control datasets.
The first four tracks share references and differ only in prompt treatment.
Copy controls explicitly supply the passage and are open-book diagnostics.

The passage track selects nonoverlapping three-verse spans from public development
anchors with complete coverage in every edition. Verses are joined with one ASCII
space, without verse numbers, and never cross chapter boundaries. Passage results
stay separate from single-verse results. This is a diagnostic sample, not a matched
length experiment; a passage score change alone cannot be attributed to length.
