# GAP Experiment Results

**Models:** gpt-4o-mini | **Metrics files:** 96 | **Timestamp range:** 2026-07-15T01:53:06Z -> 2026-07-15T18:35:28Z

This report is generated from `metrics.json` files in this working tree. Token counts and reliability are measured. Dollar costs are modeled from measured tokens using GPT-4o mini launch rates: input $0.15/M, output $0.60/M. Degenerate GAP runs and runs without both base and GAP economics are excluded from savings aggregates.

## Validity

| Set | Count |
|---|---:|
| Metrics files | 96 |
| Degenerate GAP runs excluded from economics | 28 |
| Missing comparable economics excluded from economics | 1 |
| Comparable economics set | 67 |

## Reliability

| Scope | Edit turns | Misses | Miss rate | Parse | Validation | Invalid envelope | Apply | Request | Unknown | Repairs |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| All runs with reliability | 290 | 115 | 39.7% | 0 | 0 | 0 | 115 | 0 | 0 | 0 |

A miss means the GAP attempt did not produce an applied edit. The production fallback assumption is: pay for the failed GAP attempt, then run the baseline full-regeneration edit.

## Economics

| Metric | Value |
|---|---:|
| Experiments | 67 |
| Edit turns | 195 |
| Misses | 29 (14.9%) |
| Base edit tokens | 1,393,218 |
| GAP fallback-adjusted edit tokens | 867,185 |
| Saved edit tokens | 526,033 |
| Fallback-adjusted token savings | 37.8% |
| Init-inclusive fallback token savings | 34.0% |
| Base modeled cost | $0.3896 |
| GAP fallback modeled cost | $0.2127 |
| Modeled cost saved | $0.1770 |

Interpretation: GAP is not a blanket cost win. It reliably reduces output tokens, but small artifacts can lose because target inventory, system instructions, and envelope structure add input overhead.

## Where GAP Saves Most

### By Artifact Size

| Segment | Experiments | Turns | Miss rate | Saved tokens | Fallback savings | Amortized savings | Modeled cost saved |
|---|---:|---:|---:|---:|---:|---:|---:|
| <2KB | 8 | 16 | 6.2% | -863 | -5.8% | -16.5% | $0.0012 |
| 2-5KB | 33 | 90 | 7.8% | 84,738 | 33.2% | 26.9% | $0.0379 |
| 5-10KB | 19 | 59 | 16.9% | 152,797 | 42.5% | 37.9% | $0.0580 |
| 10-25KB | 5 | 20 | 35.0% | 108,520 | 34.2% | 32.5% | $0.0329 |
| >=25KB | 2 | 10 | 40.0% | 180,841 | 40.5% | 38.2% | $0.0470 |

### By Edit Count

| Segment | Experiments | Turns | Miss rate | Saved tokens | Fallback savings | Amortized savings | Modeled cost saved |
|---|---:|---:|---:|---:|---:|---:|---:|
| 2 edits | 22 | 44 | 4.5% | 28,317 | 28.5% | 19.1% | $0.0171 |
| 3 edits | 32 | 96 | 13.5% | 101,878 | 31.4% | 26.1% | $0.0431 |
| 4 edits | 10 | 40 | 25.0% | 139,738 | 33.9% | 32.2% | $0.0487 |
| 5 edits | 3 | 15 | 26.7% | 256,100 | 45.9% | 43.4% | $0.0681 |

### By Format

| Segment | Experiments | Turns | Miss rate | Saved tokens | Fallback savings | Amortized savings | Modeled cost saved |
|---|---:|---:|---:|---:|---:|---:|---:|
| text/html | 13 | 45 | 24.4% | 220,709 | 35.5% | 33.7% | $0.0710 |
| application/xml | 3 | 11 | 27.3% | 109,330 | 56.2% | 51.6% | $0.0305 |
| application/json | 2 | 9 | 33.3% | 67,548 | 35.0% | 32.5% | $0.0205 |
| text/x-yaml | 8 | 22 | 13.6% | 29,497 | 42.0% | 36.6% | $0.0113 |
| text/x-python | 10 | 28 | 7.1% | 22,111 | 30.9% | 25.5% | $0.0103 |
| text/x-rust | 4 | 11 | 0.0% | 17,291 | 46.4% | 39.4% | $0.0070 |
| text/javascript | 3 | 10 | 10.0% | 21,320 | 44.0% | 37.7% | $0.0064 |
| text/x-go | 4 | 10 | 20.0% | 13,243 | 36.7% | 33.2% | $0.0053 |
| text/markdown | 4 | 10 | 10.0% | 10,478 | 36.5% | 29.7% | $0.0046 |
| text/typescript | 4 | 11 | 18.2% | 10,803 | 34.7% | 28.3% | $0.0044 |
| text/css | 3 | 8 | 0.0% | 894 | 3.0% | -8.9% | $0.0032 |
| text/x-sh | 2 | 4 | 0.0% | 2,485 | 37.9% | 33.0% | $0.0012 |
| image/svg+xml | 1 | 2 | 0.0% | 1,884 | 34.3% | 22.6% | $0.0011 |
| text/x-ruby | 1 | 2 | 0.0% | 981 | 32.0% | 16.7% | $0.0006 |
| text/x-sql | 1 | 3 | 0.0% | 360 | 5.8% | 0.7% | $0.0001 |
| text/x-java | 1 | 3 | 0.0% | -416 | -7.6% | -6.9% | $0.0000 |
| text/x-toml | 3 | 6 | 16.7% | -2,485 | -68.9% | -73.7% | $-0.0004 |

## Largest Absolute Savings

| Experiment | Format | Turns | Miss rate | Saved tokens | Fallback savings | Amortized savings | Modeled cost saved |
|---|---|---:|---:|---:|---:|---:|---:|
| `105-xml-rss-feed-multipage` | `application/xml` | 5 | 20.0% | 107,581 | 65.4% | 61.6% | $0.0281 |
| `102-json-paginated-users` | `application/json` | 5 | 0.0% | 75,259 | 67.9% | 64.5% | $0.0211 |
| `101-html-catalog-multipage` | `text/html` | 5 | 60.0% | 73,260 | 26.0% | 24.5% | $0.0189 |
| `008-html-admin-users` | `text/html` | 4 | 0.0% | 25,386 | 60.2% | 58.3% | $0.0094 |
| `033-yaml-cloudformation` | `text/x-yaml` | 4 | 0.0% | 22,604 | 67.2% | 62.6% | $0.0076 |
| `010-html-kanban` | `text/html` | 4 | 0.0% | 20,442 | 62.4% | 57.9% | $0.0067 |
| `002-html-dashboard-analytics` | `text/html` | 4 | 25.0% | 18,225 | 42.1% | 41.0% | $0.0058 |
| `070-html-data-visualization` | `text/html` | 4 | 50.0% | 18,566 | 30.7% | 32.1% | $0.0056 |
| `074-html-status-page` | `text/html` | 4 | 25.0% | 17,122 | 48.0% | 47.0% | $0.0056 |
| `001-html-dashboard-ecommerce` | `text/html` | 4 | 50.0% | 16,212 | 34.2% | 33.8% | $0.0054 |
| `003-html-landing-saas` | `text/html` | 3 | 0.0% | 12,669 | 64.0% | 59.5% | $0.0046 |
| `020-ts-react-form` | `text/typescript` | 3 | 0.0% | 11,342 | 79.2% | 70.2% | $0.0037 |
| `041-rust-cli-file-processor` | `text/x-rust` | 3 | 0.0% | 9,254 | 69.1% | 62.6% | $0.0034 |
| `018-js-react-data-table` | `text/javascript` | 4 | 25.0% | 11,082 | 47.6% | 44.3% | $0.0033 |
| `065-html-email-newsletter` | `text/html` | 3 | 0.0% | 8,481 | 59.0% | 52.5% | $0.0032 |

## Strongest Amortized Percentage Savings

| Experiment | Format | Turns | Miss rate | Saved tokens | Fallback savings | Amortized savings | Modeled cost saved |
|---|---|---:|---:|---:|---:|---:|---:|
| `020-ts-react-form` | `text/typescript` | 3 | 0.0% | 11,342 | 79.2% | 70.2% | $0.0037 |
| `102-json-paginated-users` | `application/json` | 5 | 0.0% | 75,259 | 67.9% | 64.5% | $0.0211 |
| `033-yaml-cloudformation` | `text/x-yaml` | 4 | 0.0% | 22,604 | 67.2% | 62.6% | $0.0076 |
| `041-rust-cli-file-processor` | `text/x-rust` | 3 | 0.0% | 9,254 | 69.1% | 62.6% | $0.0034 |
| `105-xml-rss-feed-multipage` | `application/xml` | 5 | 20.0% | 107,581 | 65.4% | 61.6% | $0.0281 |
| `066-python-fastapi-auth` | `text/x-python` | 3 | 0.0% | 6,786 | 64.6% | 60.0% | $0.0024 |
| `081-go-grpc-service` | `text/x-go` | 3 | 0.0% | 6,429 | 64.5% | 59.8% | $0.0022 |
| `003-html-landing-saas` | `text/html` | 3 | 0.0% | 12,669 | 64.0% | 59.5% | $0.0046 |
| `008-html-admin-users` | `text/html` | 4 | 0.0% | 25,386 | 60.2% | 58.3% | $0.0094 |
| `010-html-kanban` | `text/html` | 4 | 0.0% | 20,442 | 62.4% | 57.9% | $0.0067 |
| `016-python-websocket-chat` | `text/x-python` | 3 | 0.0% | 5,046 | 59.2% | 53.2% | $0.0019 |
| `065-html-email-newsletter` | `text/html` | 3 | 0.0% | 8,481 | 59.0% | 52.5% | $0.0032 |
| `074-html-status-page` | `text/html` | 4 | 25.0% | 17,122 | 48.0% | 47.0% | $0.0056 |
| `075-rust-error-types` | `text/x-rust` | 2 | 0.0% | 2,978 | 53.7% | 46.9% | $0.0013 |
| `047-shell-setup-dev` | `text/x-sh` | 2 | 0.0% | 2,371 | 50.1% | 45.8% | $0.0010 |

## Negative Savings

| Experiment | Format | Turns | Miss rate | Saved tokens | Fallback savings | Amortized savings | Modeled cost saved |
|---|---|---:|---:|---:|---:|---:|---:|
| `051-toml-cargo-workspace` | `text/x-toml` | 2 | 0.0% | -1,465 | -122.7% | -124.0% | $-0.0003 |
| `060-toml-rustfmt` | `text/x-toml` | 2 | 50.0% | -870 | -114.9% | -115.0% | $-0.0002 |
| `071-python-orm-models` | `text/x-python` | 3 | 0.0% | -4,122 | -92.7% | -80.4% | $-0.0006 |
| `086-ts-react-hooks` | `text/typescript` | 3 | 66.7% | -3,943 | -55.4% | -46.8% | $-0.0008 |
| `054-xml-rss-feed` | `application/xml` | 3 | 66.7% | -3,638 | -19.8% | -23.9% | $0.0001 |
| `039-css-design-system` | `text/css` | 4 | 0.0% | -2,190 | -19.4% | -27.7% | $-0.0001 |
| `042-rust-http-client` | `text/x-rust` | 3 | 0.0% | -951 | -15.2% | -18.9% | $0.0002 |
| `012-python-cli-log-analyzer` | `text/x-python` | 3 | 66.7% | -1,110 | -10.3% | -10.6% | $0.0002 |
| `077-json-database-seed` | `application/json` | 4 | 75.0% | -7,711 | -9.4% | -10.3% | $-0.0006 |
| `052-toml-pyproject` | `text/x-toml` | 2 | 0.0% | -150 | -9.1% | -15.5% | $0.0000 |
| `068-yaml-github-actions-release` | `text/x-yaml` | 3 | 33.3% | -543 | -9.1% | -11.2% | $0.0001 |
| `055-java-spring-controller` | `text/x-java` | 3 | 0.0% | -416 | -7.6% | -6.9% | $0.0000 |
| `073-yaml-terraform-vars` | `text/x-yaml` | 2 | 0.0% | -163 | -7.0% | -22.2% | $0.0002 |
| `088-yaml-k8s-helm-values` | `text/x-yaml` | 3 | 33.3% | -192 | -3.2% | -1.0% | $0.0001 |

## Operating Guidance

- Prefer GAP for artifacts above roughly 2 KB, repeated records/pages, dashboards, catalogs, feeds, API payloads, and code files with stable section boundaries.
- Avoid GAP for tiny config files and one-off edits where the marker and supervisor inventory overhead can exceed the full-regeneration baseline.
- Treat fallback-adjusted and init-inclusive savings as the headline economics. Raw output savings alone overstates value when misses or setup overhead are present.
- Continue improving miss rate on high-value large HTML/XML/JSON cases first. Those cases dominate absolute token and dollar savings.
