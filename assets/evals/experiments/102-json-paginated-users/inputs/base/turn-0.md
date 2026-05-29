Create a realistic JSON API response for a paginated users endpoint.

The top-level object MUST have exactly two keys:

- pagination: an object with these fields, in this order: page (integer, set to 1), per_page (integer, set to 20), total (integer, set to 100), total_pages (integer, set to 5).
- data: an array of EXACTLY 100 user objects. These 100 users represent the full dataset across 5 pages of 20 users each (page 1 = users 1-20, page 2 = users 21-40, page 3 = users 41-60, page 4 = users 61-80, page 5 = users 81-100).

Every user object MUST have these fields, in this exact order, with consistent shapes so each is addressable by JSON Pointer:

- id: integer, sequential from 1 to 100 (the user at array index N has id N+1).
- name: realistic full name string.
- email: realistic email string.
- role: one of "admin", "editor", "viewer", "billing".
- team: one of "platform", "growth", "support", "data".
- active: boolean.

Use realistic, varied names, emails, roles, and teams. Output the 100 user objects in id order (1 through 100). Output raw JSON only.