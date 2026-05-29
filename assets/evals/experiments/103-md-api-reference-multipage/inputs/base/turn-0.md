Create a large multi-page API reference document in Markdown for a fictional cloud platform REST API.

Organize the document into EXACTLY 6 pages, where each page is a level-2 heading (`## `). Use these exact page titles in this exact order:

## Page 1: Accounts
## Page 2: Billing
## Page 3: Compute
## Page 4: Storage
## Page 5: Networking
## Page 6: Monitoring

Each page documents EXACTLY 15 endpoints, for a total of EXACTLY 90 endpoint entries.

Each endpoint is a level-3 heading (`### `) and MUST follow this exact, stable, addressable format:

### EP-PNN Title
**Method:** `GET`
**Path:** `/v1/...`

| Parameter | Type | Required | Description |
|---|---|---|---|
| ... | ... | ... | ... |

**Response:**
```json
{ ... }
```

ID rules (mandatory, deterministic):
- Each endpoint heading begins with a stable ID of the form `EP-PNN` where P is the page number (1-6) and NN is the two-digit endpoint index within the page (01-15).
- Page 1 uses IDs EP-101 through EP-115, Page 2 uses EP-201 through EP-215, Page 3 uses EP-301 through EP-315, Page 4 uses EP-401 through EP-415, Page 5 uses EP-501 through EP-515, Page 6 uses EP-601 through EP-615.
- So the full set of 90 IDs is EP-101..EP-115, EP-201..EP-215, EP-301..EP-315, EP-401..EP-415, EP-501..EP-515, EP-601..EP-615, in that order.

Per-endpoint rules:
- `**Method:**` is one of GET, POST, PUT, PATCH, DELETE wrapped in backticks.
- `**Path:**` is a versioned path beginning with `/v1/` wrapped in backticks, e.g. `/v1/accounts`.
- A Markdown parameters table with header row `| Parameter | Type | Required | Description |` and at least 2 parameter rows.
- A `**Response:**` label followed by a fenced ```json code block with a realistic example object.

Make the content realistic and varied across pages (Accounts, Billing, Compute, Storage, Networking, Monitoring). Output the full document with all 90 endpoints fully filled in.