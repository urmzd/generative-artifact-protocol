Create a docker-compose.yaml for local development of a web application.

Include:
- App service: Node.js with hot reload, volume mounts, debugger port
- PostgreSQL: with initialization SQL, persistent volume, exposed port
- Redis: latest image, persistent volume
- Mailhog: for email testing (SMTP + web UI)
- Environment variables from .env file
- Named volumes for data persistence

Use section IDs: services, volumes

Use AAP section markers to delineate each major block.
Wrap each logical section with `# region id` and `# endregion id`.


Output raw code only. No markdown fences, no explanation.