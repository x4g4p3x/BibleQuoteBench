# Dataset construction

BibleQuoteBench v0.1 uses one shared set of 500 single-verse references across
BSB, ASV, and WEB. Using the same references is essential for meaningful
translation-confusion comparisons.

The source USFM archives are not committed. Each archive, every USFM member, and
the imported corpus have SHA-256 digests in `data/locks`. The build script refuses
upstream bytes or importer output that differs from those locks.

The selection is deterministic and independent of hash-map iteration order.
Public references are ranked by SHA-256 over the public seed, purpose, and
canonical reference. Hidden references use a separate private seed stored at
`data/hidden/sampling-secret.txt`; publishing only the output commitment prevents
the hidden set from being reconstructed from the repository. The 500 references
contain:

- 50 editorially curated famous or well-known verses;
- 50 automatically selected translation-sensitive verses with the greatest
  mean pairwise whitespace-token edit distance;
- 25 shortest remaining verses;
- 25 longest remaining verses;
- 350 random remaining verses.

Quotas are divided 20/80 before selection: 100 public development references are
selected with the public seed, then 400 non-overlapping hidden evaluation
references are selected with the private seed. With three translations this
produces 300 public and 1,200 hidden cases.

Hidden references and texts are generated under `data/hidden`, which Git ignores.
The public release manifest records their counts and a SHA-256 commitment to the
ordered hidden case file. Publishing that file later allows independent
verification that the hidden set was not changed after results were observed.
The build script creates a cryptographically random private seed if one is not
already present. An evaluator must back that ignored file up securely; the same
seed reproduces the same hidden files and commitment. A fresh checkout without
that seed intentionally creates a different hidden set while reproducing the
same public development set.

“Famous” is an editorial label and should eventually be backed by a documented
quotation-frequency dataset. “Translation-sensitive”, “short”, and “long” are
mechanically reproducible labels. No verse is currently labeled “obscure” without
external evidence.
