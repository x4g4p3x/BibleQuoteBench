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

- **Overview:** the opening interpretation explains what was measured, the observed
  accuracy, and what paired uncertainty can support. Synthetic responses remain
  explicitly labeled as demonstrations. **Exact text** (ExactText) and **Words
  only** (ExactWords) affect the summary, accuracy chart, and edition breakdown.
  Paired comparisons and request reliability are labeled as fixed Exact text;
  switching to Words only never implies a paired interval was calculated for it.
  Headline percentages use one decimal place; exact values and configuration
  identifiers are available in expandable details. Configurations sharing a model
  name receive distinct numbered labels throughout the report.
- **Edition diagnostics:** select a model to inspect accuracy by edition and
  reference group, the edition-differing subset, and the resemblance matrix.
  Exact-alternative cells count matches, so a response matching multiple editions
  can contribute more than once. These descriptive breakdowns do not have their
  own confidence intervals. Matrix counts are independent of the accuracy metric.
- **Failure explorer:** select an outcome count or a nonzero matrix cell to open
  matching exported examples. The selection shows the originating aggregate count
  separately from the number of illustrative examples; an empty selection does
  not mean no such outcome occurred. Clear the selection, reset all filters, or
  return to the result with keyboard focus restored. Search and failure-type
  filters further narrow the selected subset. Changing the model clears the
  originating selection. Highlights show word additions and deletions and do not
  replace the scorer's normalization or edit metrics.

New examples include the scorer-derived `resembles` destination so matrix
navigation preserves operational categories and ambiguous edition ties. Older
v0.2 analyses still render, but examples without this metadata cannot be matched
to resemblance cells; the explorer explains this limitation. Exact-alternative
links require both the translation-confusion class and the matching alternative.

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
