#!/usr/bin/env python3
"""Regenerate the archived 2026-10-08 Grok trial; no model or network calls."""
import argparse
import difflib
import hashlib
import json
import math
from pathlib import Path
import re
import unicodedata

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import MaxNLocator

ROOT = Path(__file__).resolve().parents[1]
DEFAULT = ROOT / "docs/trials/2026-10-08-grok47-xhigh-cursor"
EXACT, PARTIAL = "#147d74", "#bd5c32"
INK, MUTED, GRID = "#172d3c", "#536877", "#e1e7ec"


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path):
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def reference_key(reference):
    return tuple(reference.get(key) for key in ("book", "chapter", "verse_start", "verse_end"))


def reference_label(reference):
    end = f"–{reference['verse_end']}" if reference.get("verse_end") else ""
    return f"{reference['book']} {reference['chapter']}:{reference['verse_start']}{end}"


def words(text):
    """Display token counts checked against the Rust scorer's saved WER."""
    text = unicodedata.normalize("NFC", text.replace("\r\n", "\n").replace("\r", "\n"))
    result, current = [], ""
    for index, character in enumerate(text):
        apostrophe = character in "'’" and current and index + 1 < len(text) and text[index + 1].isalnum()
        if character.isalnum() or apostrophe:
            current += "'" if apostrophe else character
        elif current:
            result.append(current)
            current = ""
    if current:
        result.append(current)
    return result


def display_alignment(expected, output):
    """Whitespace-token alignment is illustrative, never normative scoring."""
    expected, output = expected.split(), output.split()
    left, right = [], []
    for operation, i, j, k, l in difflib.SequenceMatcher(None, expected, output, autojunk=False).get_opcodes():
        left.extend({"word": word, "changed": operation != "equal"} for word in expected[i:j])
        right.extend({"word": word, "changed": operation != "equal"} for word in output[k:l])
    return {"expected": left, "output": right}


def load_bundle(folder):
    scores, responses, cases = (read_jsonl(folder / name) for name in ("scores.jsonl", "responses.jsonl", "cases.jsonl"))
    report, trial, provenance, catalog = (read_json(folder / name) for name in ("report.json", "trial.json", "provenance.json", "catalog.json"))
    ids = trial["identity"]["expected_case_ids"]
    if not ids or len(ids) != len(set(ids)) or any(not value.startswith("BQ-DEV-") for value in ids):
        raise ValueError("Only unique public development cases may be published")
    if trial["identity"]["evidence"] != "interactive_mcp" or provenance["evidence"] != "interactive_mcp":
        raise ValueError("This renderer requires explicitly interactive evidence")
    specs = {entry["id"]: entry for entry in catalog["translations"]}
    references = read_jsonl(folder / "references.jsonl")
    if any(not specs[record["translation"]]["redistribute_reference_text"] or specs[record["translation"]]["license_kind"] != "public_domain" for record in references):
        raise ValueError("Reference text is not approved for public redistribution")
    for records in (scores, responses, cases):
        if [record["case_id"] for record in records] != ids:
            raise ValueError("Bundle case order or completeness changed")
    expected = {(record["translation"], reference_key(record["reference"])): record["text"] for record in references}
    rows = []
    for order, (score, response, case) in enumerate(zip(scores, responses, cases), 1):
        if case["prompt_variant"] != "canonical" or score["reference"] != case["reference"]:
            raise ValueError("Case identity or prompt variant changed")
        if hashlib.sha256(response["output"].encode()).hexdigest() != score["response_sha256"]:
            raise ValueError("Response output does not match its score commitment")
        text = expected[(case["translation"], reference_key(case["reference"]))]
        length = len(words(text))
        edits = sum(score[key] for key in ("insertions", "deletions", "substitutions"))
        if not math.isclose(edits / length, score["word_error_rate"], abs_tol=1e-12):
            raise ValueError("Reference word count or saved WER changed")
        if not math.isclose(max(0, 1 - score["word_error_rate"]), score["word_accuracy"], abs_tol=1e-12):
            raise ValueError("Saved word accuracy changed")
        rows.append(dict(score, order=order, label=reference_label(case["reference"]), reference_words=length, expected=text, output=response["output"], alignment=display_alignment(text, response["output"])))
    overall = report["overall"]
    # The prose and dashboard describe this particular archived observation.
    # Refuse another dataset instead of attaching its metrics to those claims.
    if (trial["identity"]["run_id"] != "grok47-xhigh-cursor-20261008-01"
            or len(rows) != 10
            or overall["classifications"] != {"exact_requested": 2, "partial": 8}
            or not math.isclose(overall["mean_word_accuracy"], 0.794896849854477, abs_tol=1e-12)
            or report["exact_alternative_matches"]
            or any(row["exact_words"] != row["exact_text"] for row in rows)):
        raise ValueError("This renderer and its editorial template are specific to the archived Grok trial")
    if overall["responses"] != len(rows) or not math.isclose(overall["mean_word_accuracy"], sum(row["word_accuracy"] for row in rows) / len(rows), abs_tol=1e-12):
        raise ValueError("Overall summary does not match case metrics")
    if not math.isclose(overall["exact_text_rate"], sum(row["exact_text"] for row in rows) / len(rows), abs_tol=1e-12):
        raise ValueError("Exact recall denominator changed")
    return {"rows": rows, "report": report, "provenance": provenance, "trial": trial}


def style_axis(axis):
    for side in ("top", "right", "left"):
        axis.spines[side].set_visible(False)
    axis.spines["bottom"].set_color(GRID)
    axis.tick_params(length=0, colors=MUTED, pad=8)
    axis.set_axisbelow(True)
    axis.grid(axis="x", color=GRID, linewidth=0.7)


def save_figure(figure, folder, name, footer):
    figure.text(0.015, 0.018, footer, fontsize=9, color=MUTED)
    figure.savefig(folder / f"{name}.svg", metadata={"Date": None}, facecolor="white")
    # Normalize the Windows writer's newlines before hashing or embedding SVG.
    # Removing trailing path-data spaces preserves the newline separator.
    svg_path = folder / f"{name}.svg"
    svg_path.write_text("\n".join(line.rstrip() for line in svg_path.read_text(encoding="utf-8").splitlines()) + "\n", encoding="utf-8", newline="\n")
    figure.savefig(folder / f"{name}.png", dpi=180, metadata={"Software": f"Matplotlib {matplotlib.__version__}"}, facecolor="white")
    plt.close(figure)


def figures(bundle, folder):
    folder.mkdir(exist_ok=True)
    plt.rcParams.update({"font.family": "DejaVu Sans", "font.size": 11, "axes.labelcolor": INK, "text.color": INK, "svg.fonttype": "none", "svg.hashsalt": "BibleQuoteBench-Grok47-trial-v1"})
    rows, report = bundle["rows"], bundle["report"]
    footer = "Grok 4.7 xHigh · Fast off · BSB 2025 third printing · 8 October 2026 · Interactive diagnostic, n=10"
    fig, axis = plt.subplots(figsize=(9, 3.8))
    fig.subplots_adjust(left=0.22, right=0.93, top=0.75, bottom=0.24)
    axis.barh([1, 0], [2, 8], color=[EXACT, PARTIAL], height=0.55)
    axis.set_yticks([1, 0], ["Exact passage", "Partial passage"])
    axis.set_xlim(0, 10)
    axis.xaxis.set_major_locator(MaxNLocator(integer=True))
    axis.set_xlabel("Passages out of ten")
    for y, count in [(1, 2), (0, 8)]:
        axis.text(count + 0.12, y, f"{count}/10  ({count * 10}%)", va="center", fontsize=12)
    style_axis(axis)
    fig.suptitle("Two passages reproduced exactly", x=0.03, ha="left", fontsize=20, fontweight="bold")
    fig.text(0.03, 0.83, "ExactText and ExactWords both succeed for the same two passages.", color=MUTED)
    save_figure(fig, folder, "outcomes", footer)

    fig, axis = plt.subplots(figsize=(10, 6.8))
    fig.subplots_adjust(left=0.31, right=0.94, top=0.82, bottom=0.15)
    positions = list(range(len(rows)))
    axis.barh(positions, [row["word_accuracy"] * 100 for row in rows], color=[EXACT if row["exact_text"] else PARTIAL for row in rows], height=0.56)
    axis.set_yticks(positions, [f"{row['order']:02d}  {row['label']}" for row in rows])
    axis.invert_yaxis()
    axis.set_xlim(0, 110)
    axis.set_xticks([0, 25, 50, 75, 100], ["0%", "25%", "50%", "75%", "100%"])
    axis.set_xlabel("Word accuracy = max(0, 1 − WER)")
    for position, row in enumerate(rows):
        axis.text(row["word_accuracy"] * 100 + 1, position, f"{row['word_accuracy'] * 100:.1f}%", va="center", fontsize=10, bbox={"facecolor": "white", "edgecolor": "none", "pad": 1.5})
    axis.axvline(report["overall"]["mean_word_accuracy"] * 100, color=INK, linewidth=1.1, linestyle="--")
    style_axis(axis)
    fig.suptitle("Close wording can still fail exact recall", x=0.03, ha="left", fontsize=20, fontweight="bold")
    fig.text(0.03, 0.895, "Green = exact passage; rust = partial. Dashed line = mean across cases, 79.49%.", color=MUTED)
    save_figure(fig, folder, "case-accuracy", footer)

    fig, axis = plt.subplots(figsize=(10, 6.8))
    fig.subplots_adjust(left=0.31, right=0.94, top=0.80, bottom=0.15)
    left = [0] * len(rows)
    for key, label, color in [("insertions", "Insertions", "#3276a3"), ("deletions", "Deletions", "#c58c28"), ("substitutions", "Substitutions", "#765b94")]:
        values = [row[key] for row in rows]
        axis.barh(positions, values, left=left, color=color, height=0.56, label=f"{label} ({sum(values)})")
        for position, (start, value) in enumerate(zip(left, values)):
            if value:
                axis.text(start + value / 2, position, str(value), ha="center", va="center", color="white", fontsize=9, fontweight="bold")
        left = [start + value for start, value in zip(left, values)]
    axis.set_yticks(positions, [f"{row['order']:02d}  {row['label']}" for row in rows])
    axis.invert_yaxis()
    axis.set_xlim(0, max(left) + 3)
    axis.xaxis.set_major_locator(MaxNLocator(integer=True))
    axis.set_xlabel("Word edits in the normative Levenshtein alignment")
    for position, count in enumerate(left):
        if not count:
            axis.text(0.3, position, "0 · exact", va="center", color=EXACT)
    axis.legend(loc="upper left", bbox_to_anchor=(0, 1.12), frameon=False, ncol=3, fontsize=10)
    style_axis(axis)
    fig.suptitle("Seventy word edits across the ten passages", x=0.03, ha="left", fontsize=20, fontweight="bold")
    fig.text(0.03, 0.90, "Raw counts depend on passage length. Use per-case WER to compare normalized error.", color=MUTED)
    save_figure(fig, folder, "word-edits", footer)

    strata = list(report["by_stratum"].items())
    fig, axes = plt.subplots(1, 2, figsize=(13, 5.3), gridspec_kw={"width_ratios": [1.35, 1]})
    fig.subplots_adjust(left=0.23, right=0.96, top=0.76, bottom=0.2, wspace=0.75)
    axis = axes[0]
    axis.scatter([summary["mean_word_accuracy"] * 100 for _, summary in strata], list(range(len(strata))), color=INK, s=42)
    axis.set_yticks(list(range(len(strata))), [f"{name.replace('_', ' ').title()}  (n={summary['responses']})" for name, summary in strata])
    axis.invert_yaxis()
    axis.set_xlim(0, 110)
    axis.set_xticks([0, 50, 100], ["0%", "50%", "100%"])
    axis.set_xlabel("Mean word accuracy")
    axis.set_title("Six sampled strata", loc="left", fontsize=13, fontweight="bold", pad=18)
    style_axis(axis)
    axis = axes[1]
    destinations = report["requested_to_resembles"]["bsb-2025-third-printing"]
    labels = ["Exact BSB", "Closer to ASV", "Closer to WEB", "Unclassified error"]
    values = [destinations.get(key, 0) for key in ["bsb-2025-third-printing", "asv-1901", "web-classic-2020", "_unclassified"]]
    axis.barh(list(range(4)), values, color=[EXACT, "#3276a3", "#765b94", PARTIAL], height=0.5)
    axis.set_yticks(list(range(4)), labels)
    axis.invert_yaxis()
    axis.set_xlim(0, 7)
    axis.xaxis.set_major_locator(MaxNLocator(integer=True))
    axis.set_xlabel("Passages")
    axis.set_title("Resemblance destinations", loc="left", fontsize=13, fontweight="bold", pad=18)
    for position, count in enumerate(values):
        axis.text(count + 0.1, position, str(count), va="center")
    style_axis(axis)
    fig.suptitle("Sparse strata and approximate resemblance", x=0.03, ha="left", fontsize=20, fontweight="bold")
    fig.text(0.03, 0.86, "Most strata contain one passage. Closer wording is not an exact alternative-edition match.", color=MUTED)
    save_figure(fig, folder, "diagnostics", footer)


def inline_svg(path):
    svg = path.read_text(encoding="utf-8")
    svg = svg[svg.index("<svg"):]
    prefix = path.stem + "-"
    svg = re.sub(r'id="([^"]+)"', lambda match: f'id="{prefix}{match[1]}"', svg)
    svg = re.sub(r'url\(#([^)]+)\)', lambda match: f'url(#{prefix}{match[1]})', svg)
    svg = re.sub(r'(?:xlink:)?href="#([^"]+)"', lambda match: match[0].replace("#", "#" + prefix, 1), svg)
    return svg


def dashboard(bundle, folder):
    template = (ROOT / "scripts/trial-dashboard.html").read_text(encoding="utf-8")
    public = {"rows": bundle["rows"], "overall": bundle["report"]["overall"], "provenance": bundle["provenance"]}
    embedded = json.dumps(public, ensure_ascii=False, separators=(",", ":")).replace("&", "\\u0026").replace("<", "\\u003c").replace(">", "\\u003e")
    template = template.replace("@@TRIAL_DATA@@", embedded)
    for name in ("outcomes", "case-accuracy", "word-edits", "diagnostics"):
        template = template.replace("@@" + name.upper().replace("-", "_") + "@@", inline_svg(folder / "figures" / f"{name}.svg"))
    if "@@" in template:
        raise ValueError("Unresolved dashboard template marker")
    (folder / "index.html").write_text(template, encoding="utf-8", newline="\n")


def case_details(bundle, folder):
    lines = ["# Full passage comparisons", "", "All ten public cases in presentation order. Reference wording comes from the", "archived BSB subset; submitted wording is copied from the raw response records.", "Percentages are rounded to two decimals. The saved JSONL scores are authoritative.", "", "[Return to the report](README.md) · [Open the offline explorer](index.html)", ""]
    for row in bundle["rows"]:
        lines.extend([f"## {row['order']:02d} · {row['label']}", "", f"Case `{row['case_id']}` · `{row['stratum']}` · `{row['classification']}`.", "", f"Word accuracy **{row['word_accuracy'] * 100:.2f}%**; WER {row['word_error_rate'] * 100:.2f}%; CER {row['character_error_rate'] * 100:.2f}%. Word edits: {row['insertions']} insertions, {row['deletions']} deletions, {row['substitutions']} substitutions against {row['reference_words']} reference words.", "", "**Expected · BSB 2025 third printing**", "", "> " + row["expected"].replace("\n", "\n> "), "", "**Submitted · Grok 4.7 xHigh**", "", "> " + row["output"].replace("\n", "\n> "), ""])
    (folder / "case-details.md").write_text("\n".join(lines), encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trial-dir", type=Path, default=DEFAULT)
    args = parser.parse_args()
    folder = args.trial_dir.resolve()
    bundle = load_bundle(folder)
    figures(bundle, folder / "figures")
    dashboard(bundle, folder)
    case_details(bundle, folder)
    generated = [folder / "index.html", *sorted((folder / "figures").glob("*.svg")), *sorted((folder / "figures").glob("*.png"))]
    (folder / "figure-hashes.json").write_text(json.dumps({str(path.relative_to(folder)).replace("\\", "/"): hashlib.sha256(path.read_bytes()).hexdigest() for path in generated}, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"Rendered {len(bundle['rows'])} audited cases, four figure pairs, and the offline dashboard.")


if __name__ == "__main__":
    main()
