# Interactive results

The benchmark generates a single HTML file containing its data, styles, and
controls. Open it directly in a browser; no server, installation, internet
connection, or provider credentials are needed.

Use **Appearance** to choose System, Light, or Dark. System follows your device's
color preference, including changes while the report is open. A manual choice is
remembered when browser storage is available. Print/PDF output uses the light palette.

`analyze` writes `analysis.html` alongside `analysis.json`, `analysis.md`, and
`examples.jsonl`. To render existing v0.2 analyses together:

```console
cargo run -- visualize --analysis docs/pilot/v0.2/canonical/analysis.json --analysis docs/pilot/v0.2/copy_control/analysis.json --output results/report.html
```

Repeat `--analysis` for each input. Each analysis stays separate in the track
selector; the viewer does not pool observations or recompute inference.
`synthetic-pilot` also generates `docs/pilot/v0.2/index.html` with all five tracks.
The checked-in pilot uses constructed responses and is not a provider ranking.

## Reading the report

- **Overview:** switch between ExactText and ExactWords to inspect recall with
  95% reference-cluster bootstrap intervals. Expand the exact-values table for
  accessible numeric detail. Paired comparisons always show ExactText differences
  in percentage points, including whether their intervals cross zero. Outcome
  counts and the request-reliability table keep provider failures visible.
- **Edition diagnostics:** select a model configuration to inspect translation
  and stratum breakdowns, the edition-differing subset, and the requested-to-
  resembles matrix. Exact-alternative cells count matches, so a response matching
  multiple editions can contribute more than once. These descriptive breakdowns
  do not have their own confidence intervals.
- **Failure explorer:** search and filter the supplied illustrative examples,
  then inspect expected and produced wording with word additions and deletions.
  These examples are a selected subset, not all failures. Display highlights do
  not replace the scorer's normalization or edit metrics.

Track and evidence labels remain visible. Copy controls are open-book diagnostics;
prompt variants and passages are also separate from primary canonical recall.
The [scoring contract](SCORING.md) defines denominators, uncertainty, and comparison
limits. A confidence interval crossing zero does not establish equivalence.

## Sharing and printing

Share the HTML file itself. All supplied analysis data, including available
example wording, is embedded in it; filtering does not remove data from the file.
The validated analysis producer suppresses example wording for hidden cases and
private licensed catalogs. Use analyses produced by `analyze`; the viewer's
structural checks do not authenticate hand-edited JSON or rerun provenance checks.

The report makes no network requests. Embedded data is escaped before insertion
and wording is displayed as text rather than executable HTML.

**Print / save PDF** prints the current track and diagnostic selections across
all report sections, including the currently selected example. The HTML retains
interactivity; a PDF is a static snapshot. Keyboard users can move between tabs
with arrow keys, Home, and End, and use ordinary form controls for filters.
