# Fake News Site

A zero-build Node.js news website parameterised by environment variables. Used as a stand-in "creator" website for local dev stack testing.

## Environment Variables

| Variable | Description | Default |
|---|---|---|
| \`PORT\` | **Required.** The port to listen on. | (none) |
| \`CR_BIND_ADDR\` | The address to bind to. | \`127.0.0.1\` |
| \`SITE_ID\` | Unique identifier for the site. | \`1\` |
| \`SITE_NAME\` | The name of the news outlet. | \`The Daily Ledger\` |
| \`SITE_TAGLINE\` | The tagline of the news outlet. | \`News you can trust\` |
| \`SITE_ACCENT\` | CSS colour for the site's accent (primary colour). | \`#1d7d2f\` |
| \`SITE_SEED\` | Seed string for deterministic content generation. | \`default-seed\` |
| \`SITE_LOCALE\` | Locale for the HTML \`lang\` attribute. | \`en\` |
| \`CR_PORT\` | The public HTTPS port the site is reachable on through the proxy. When set, every page loads the events embed script from the viewer gateway. | (none) |
| \`CR_VIEWER_HOST\` | Viewer gateway host serving \`/api/embed.js\`. | \`app.crlocal\` |

## Example Invocations

### Site 1: The Daily Ledger
\`\`\`bash
PORT=8081 SITE_ID=1 SITE_NAME="The Daily Ledger" SITE_TAGLINE="News you can trust" SITE_ACCENT="#1d7d2f" SITE_SEED="ledger-seed" node server.js
\`\`\`

### Site 2: Kompas Weekly
\`\`\`bash
PORT=8082 SITE_ID=2 SITE_NAME="Kompas Weekly" SITE_TAGLINE="Your weekly compass" SITE_ACCENT="#b00020" SITE_SEED="kompas-seed" node server.js
\`\`\`
