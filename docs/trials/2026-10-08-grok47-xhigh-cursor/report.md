# Interactive MCP trial

Evidence: `interactive_mcp`. Limitations: self-reported model identity; shared conversation context; client-side retrieval restrictions are not enforced; model sampling settings and usage are unknown. Do not pool with controlled provider runs.

# BibleQuoteBench report

Exploratory descriptive report: supplied rows only; coverage and configuration comparability are not verified here. Use `analyze` for validated comparisons.

## Overall

| Responses | ExactText | ExactWords | Word accuracy | Refusals | Provider errors | Translation confusion |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 10 | 20.00% | 20.00% | 79.49% | 0.00% | 0.00% | 0.00% |

## Models

| Provider / model | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| interactive_mcp / grok-4.7-xhigh / unresolved | 10 | 20.00% | 20.00% | 79.49% | 0.00% |

## Translations

| Translation | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| bsb-2025-third-printing | 10 | 20.00% | 20.00% | 79.49% | 0.00% |

## Strata

| Stratum | Responses | ExactText | ExactWords | Word accuracy | Confusion |
| --- | ---: | ---: | ---: | ---: | ---: |
| extremely_famous | 1 | 100.00% | 100.00% | 100.00% | 0.00% |
| long_verse | 1 | 0.00% | 0.00% | 55.93% | 0.00% |
| random | 5 | 0.00% | 0.00% | 74.13% | 0.00% |
| short_verse | 1 | 0.00% | 0.00% | 75.00% | 0.00% |
| translation_sensitive | 1 | 100.00% | 100.00% | 100.00% | 0.00% |
| well_known | 1 | 0.00% | 0.00% | 93.33% | 0.00% |

## Requested → resembles

Counts use exact requested/other-edition matches first, then the closest alternative when it is strictly closer; remaining errors are `_unclassified`.

- **bsb-2025-third-printing**: _unclassified=6, asv-1901=1, bsb-2025-third-printing=2, web-classic-2020=1

Exact other-edition matches (separate from approximate resemblance):


## Stability

| Provider / model | Repeated cases | Output consistency | Exact recall |
| --- | ---: | ---: | ---: |
