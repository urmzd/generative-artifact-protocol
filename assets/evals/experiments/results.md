# GAP Experiment Results

**Model:** `gpt-5.4-mini` | **Experiments:** 18

| Experiment | Fmt | Base Out | GAP Out | Out Δ | Parse | Apply | Seq Sim | F1 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 001-html-dashboard-ecommerce | text/html |  45029 |   2321 |  94.8% | 4/4 | 4/4 | 0.669 | 0.341 |
| 011-python-fastapi-users | text/x-pyt |   4885 |    531 |  89.1% | 3/3 | 3/3 | 0.604 | 0.646 |
| 019-js-express-api | text/javas |  14962 |   1077 |  92.8% | 3/3 | 2/3 | 0.415 | 0.415 |
| 020-ts-react-form | text/types |  14641 |   4554 |  68.9% | 3/3 | 2/3 | 0.587 | 0.554 |
| 023-json-openapi-spec | applicatio |  17805 |   1808 |  89.8% | 4/4 | 1/4 | 0.509 | 0.235 |
| 026-json-api-response-users | applicatio |  21177 |   3990 |  81.2% | 3/3 | 2/3 | 0.616 | 0.737 |
| 029-yaml-docker-compose-microservices | text/x-yam |   8102 |   1579 |  80.5% | 4/4 | 4/4 | 0.567 | 0.570 |
| 034-md-readme-cli | text/markd |   5214 |    644 |  87.6% | 3/3 | 3/3 | 0.463 | 0.481 |
| 039-css-design-system | text/css |  29484 |   2228 |  92.4% | 4/4 | 4/4 | 0.486 | 0.521 |
| 041-rust-cli-file-processor | text/x-rus |  11626 |   2444 |  79.0% | 3/3 | 3/3 | 0.527 | 0.461 |
| 044-go-http-server | text/x-go |  11717 |   4199 |  64.2% | 3/3 | 2/3 | 0.491 | 0.610 |
| 046-shell-deploy-script | text/x-sh |  11333 |   2074 |  81.7% | 3/3 | 3/3 | 0.315 | 0.234 |
| 048-svg-bar-chart | image/svg+ |   8242 |   2778 |  66.3% | 3/3 | 2/3 | 0.623 | 0.412 |
| 051-toml-cargo-workspace | text/x-tom |   1189 |    288 |  75.8% | 2/2 | 2/2 | 0.555 | 0.543 |
| 053-xml-maven-pom | applicatio |   4228 |   1497 |  64.6% | 3/3 | 3/3 | 0.490 | 0.334 |
| 055-java-spring-controller | text/x-jav |   8608 |   1519 |  82.4% | 3/3 | 2/3 | 0.389 | 0.403 |
| 056-ruby-rails-model | text/x-rub |   2423 |    291 |  88.0% | 2/2 | 2/2 | 0.533 | 0.402 |
| 057-sql-schema-ecommerce | text/x-sql |    869 |   8333 | -858.9% | 3/3 | 3/3 | 0.104 | 0.069 |

**Totals:** Output savings: 81.0% | Parse: 56/56 | Apply: 47/56

## Run validity

- ⚠ **1/18** GAP runs are **degenerate** (artifact never changed — all edits no-ops). Their "output savings" are illusory and must be excluded from any headline.

## Cost analysis — init-inclusive, cache regimes (MODELED $)

Prices (USD/1M): input $0.250, cached-input $0.025, output $2.000. Output is never cached.

| Flow | Cost (cache off) | Cost (cache observed) | Cost (cache theoretical-best) |
|---|---:|---:|---:|
| Base (Scenario A, full regen) | $0.6967 | $0.5976 | $0.5854 |
| GAP (Scenario C, envelopes) | $0.3169 | $0.2989 | $0.3169 |

**GAP savings vs base:** 54.5% (cache off) → 45.9% (base perfectly cached, GAP not).
Even with a perfectly hot cache on the baseline, GAP's advantage survives — the residual is the output-token win, which no cache can discount.

**Break-even** (cumulative GAP cost < perfectly-cached base): reached in 11/18 experiments, mean edit turn 1.3.

## Agent loop — Effect 2 (orchestrator context separation), MODELED

Orchestrator-wallet input tokens summed across eligible experiments, as the orchestrator spends extra reasoning turns holding the artifact. **KeepLatest** (steelman baseline) keeps only the current body in context; **Accumulate** (worst case) retains every version; **GAP** holds a handle. This is a *separate ledger* from the edit work above (the maintain wallet = Scenario C).

| Extra turns | KeepLatest in | Accumulate in | GAP in | Re-reads avoided | GAP savings vs KeepLatest | GAP $ vs KeepLatest $ |
|---:|---:|---:|---:|---:|---:|---:|
| +0 | 259860 | 709324 | 50322 | 70 | 80.6% | $0.0126 vs $0.0650 |
| +2 | 381374 | 1229044 | 74636 | 104 | 80.4% | $0.0187 vs $0.0953 |
| +5 | 563645 | 2008624 | 111106 | 155 | 80.3% | $0.0278 vs $0.1409 |
| +10 | 867430 | 3307924 | 171890 | 240 | 80.2% | $0.0430 vs $0.2169 |

Across 17 eligible experiments. The KeepLatest column grows linearly with reasoning turns and Accumulate quadratically, while GAP stays flat — every re-read avoided is a full artifact body the orchestrator never pays to re-ingest.

## Latency (mean over edit turns, MEASURED)

Wall-clock includes network + queueing, not pure prefill/decode.

| Flow | TTFT | TTLT | Total latency |
|---|---:|---:|---:|
| Base (full regen) | 5 ms | 16643 ms | 17131 ms |
| GAP (envelopes) | 26 ms | 3397 ms | 4170 ms |
