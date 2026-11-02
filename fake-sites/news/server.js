const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');

// --- Configuration ---
const PORT = process.env.PORT;
if (!PORT) {
    console.error("FATAL: PORT environment variable is required.");
    process.exit(1);
}

const BIND_ADDR = process.env.CR_BIND_ADDR || '127.0.0.1';
const SITE_ID = process.env.SITE_ID || '1';
const SITE_NAME = process.env.SITE_NAME || 'The Daily Ledger';
const SITE_TAGLINE = process.env.SITE_TAGLINE || 'News you can trust';
const SITE_ACCENT = process.env.SITE_ACCENT || '#1d7d2f';
const SITE_SEED = process.env.SITE_SEED || 'default-seed';
const SITE_LOCALE = process.env.SITE_LOCALE || 'en';
const CR_PORT = process.env.CR_PORT || '';
const CR_VIEWER_HOST = process.env.CR_VIEWER_HOST || 'app.crlocal';
// The viewer gateway serves the events iframe; without a public port (site run
// standalone, outside the dev stack) there is no gateway to load it from.
const EMBED_SCRIPT_URL = CR_PORT
    ? `https://${CR_VIEWER_HOST}:${CR_PORT}/api/embed.js`
    : '';

// --- PRNG ---
// Simple hash to convert string seed to 32-bit integer
function xmur3(str) {
    let h = 1779033703 ^ str.length;
    for (let i = 0; i < str.length; i++) {
        h = Math.imul(h ^ str.charCodeAt(i), 3432918353);
        h = h << 13 | h >>> 19;
    }
    return function() {
        h = Math.imul(h ^ (h >>> 16), 2246822507);
        h = Math.imul(h ^ (h >>> 13), 3266489909);
        return (h ^= h >>> 16) >>> 0;
    }
}

// Mulberry32 PRNG
function mulberry32(a) {
    return function() {
        var t = a += 0x6D2B79F5;
        t = Math.imul(t ^ t >>> 15, t | 1);
        t ^= t + Math.imul(t ^ t >>> 7, t | 61);
        return ((t ^ t >>> 14) >>> 0) / 4294967296;
    }
}

const seedFn = xmur3(SITE_SEED + SITE_ID);
const random = mulberry32(seedFn());

function randomInt(min, max) {
    return Math.floor(random() * (max - min + 1)) + min;
}

function randomChoice(arr) {
    return arr[randomInt(0, arr.length - 1)];
}

// --- Content Generation ---
const SECTIONS = ['politics', 'technology', 'business', 'culture', 'science'];
const NOUNS = ['Policy', 'Market', 'System', 'Network', 'Election', 'Economy', 'Innovation', 'Crisis', 'Reform', 'Strategy', 'Agreement', 'Debate', 'Trend', 'Shift', 'Vision'];
const ADJECTIVES = ['Global', 'Local', 'Emerging', 'Critical', 'Sustainable', 'Digital', 'Public', 'Private', 'Future', 'Historic', 'Radical', 'Subtle', 'Massive', 'Unexpected', 'Inevitable'];
const VERBS = ['Transforms', 'Challenges', 'Boosts', 'Threatens', 'Redefines', 'Expands', 'Limits', 'Accelerates', 'Delays', 'Inspires', 'Provokes', 'Secures', 'Destroys', 'Creates', 'Reveals'];

function generateHeadline() {
    return `${randomChoice(ADJECTIVES)} ${randomChoice(NOUNS)} ${randomChoice(VERBS)} ${randomChoice(ADJECTIVES)} ${randomChoice(NOUNS)}`;
}

function generateSlug(headline) {
    return headline.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/(^-|-$)/g, '');
}

function generateParagraph() {
    const sentences = [];
    const numSentences = randomInt(4, 8);
    for (let i = 0; i < numSentences; i++) {
        const words = [];
        const numWords = randomInt(10, 20);
        for (let j = 0; j < numWords; j++) {
            words.push(randomChoice(NOUNS).toLowerCase());
        }
        words[0] = words[0].charAt(0).toUpperCase() + words[0].slice(1);
        sentences.push(words.join(' ') + '.');
    }
    return sentences.join(' ');
}

function generateArticle(id) {
    const headline = generateHeadline();
    const section = randomChoice(SECTIONS);
    const numParagraphs = randomInt(15, 30); // Ensure >= 1500 words roughly (15-30 paras * ~100 words)
    const body = [];
    for (let i = 0; i < numParagraphs; i++) {
        body.push(generateParagraph());
    }
    
    // Generate a deterministic date in the past year
    const dateOffset = randomInt(0, 365 * 24 * 60 * 60 * 1000);
    const date = new Date(1700000000000 - dateOffset); // Arbitrary fixed base date for determinism

    return {
        id,
        headline,
        slug: generateSlug(headline) + '-' + id,
        standfirst: generateParagraph().substring(0, 150) + '...',
        byline: `${randomChoice(['Alex', 'Sam', 'Jordan', 'Taylor', 'Casey'])} ${randomChoice(['Smith', 'Doe', 'Johnson', 'Brown', 'Williams'])}`,
        date: date.toISOString(),
        section,
        body
    };
}

const articles = [];
for (let i = 0; i < 30; i++) {
    articles.push(generateArticle(i));
}

// --- HTML Templates ---
function escapeHtml(unsafe) {
    return String(unsafe)
         .replace(/&/g, "&amp;")
         .replace(/</g, "&lt;")
         .replace(/>/g, "&gt;")
         .replace(/"/g, "&quot;")
         .replace(/'/g, "&#039;");
}

const FONT_STACKS = [
    'system-ui, "Segoe UI", Roboto, Helvetica, Arial, sans-serif',
    'Georgia, "Times New Roman", Times, serif',
    '"Courier New", Courier, monospace',
    'Impact, Consolas, "Courier New", monospace'
];
const SITE_FONT = FONT_STACKS[xmur3(SITE_NAME)() % FONT_STACKS.length];

function renderLayout(title, content) {
    return `<!DOCTYPE html>
<html lang="${escapeHtml(SITE_LOCALE)}">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>${escapeHtml(title)} - ${escapeHtml(SITE_NAME)}</title>
    <meta name="description" content="${escapeHtml(SITE_TAGLINE)}">
    <link rel="stylesheet" href="/pico.min.css">
    ${EMBED_SCRIPT_URL ? `<script src="${escapeHtml(EMBED_SCRIPT_URL)}" defer></script>` : ''}
    <style>
        :root {
            --pico-primary: ${escapeHtml(SITE_ACCENT)};
            --pico-primary-hover: ${escapeHtml(SITE_ACCENT)};
            --pico-font-family: ${SITE_FONT};
        }
        body {
            display: flex;
            flex-direction: column;
            min-height: 100vh;
        }
        main {
            flex: 1;
        }
        .skip-link {
            position: absolute;
            top: -40px;
            left: 0;
            background: var(--pico-primary);
            color: white;
            padding: 8px;
            z-index: 100;
        }
        .skip-link:focus {
            top: 0;
        }
        header.site-header {
            border-bottom: 2px solid var(--pico-primary);
            margin-bottom: var(--pico-spacing);
            padding-top: var(--pico-spacing);
            padding-bottom: var(--pico-spacing);
        }
        .site-title {
            font-size: 2rem;
            font-weight: bold;
            color: var(--pico-primary);
            margin: 0;
            text-decoration: none;
        }
        .site-tagline {
            font-size: 1rem;
            color: var(--pico-muted-color);
            margin: 0;
        }
        nav.main-nav ul {
            margin: 0;
            padding: 0;
        }
        footer.site-footer {
            border-top: 1px solid var(--pico-muted-border-color);
            margin-top: var(--pico-spacing);
            padding-top: var(--pico-spacing);
            padding-bottom: var(--pico-spacing);
            text-align: center;
        }
    </style>
</head>
<body>
    <a href="#main-content" class="skip-link">Skip to main content</a>
    <header class="site-header container">
        <hgroup>
            <h1><a href="/" class="site-title">${escapeHtml(SITE_NAME)}</a></h1>
            <p class="site-tagline">${escapeHtml(SITE_TAGLINE)}</p>
        </hgroup>
        <nav class="main-nav">
            <ul>
                ${SECTIONS.map(s => `<li><a href="/section/${escapeHtml(s)}">${escapeHtml(s.charAt(0).toUpperCase() + s.slice(1))}</a></li>`).join('')}
            </ul>
        </nav>
    </header>
    <main id="main-content" class="container">
        ${content}
    </main>
    <footer class="site-footer container">
        <p>&copy; 2026 ${escapeHtml(SITE_NAME)}. All rights reserved.</p>
    </footer>
</body>
</html>`;
}

function renderArticleList(articlesList) {
    return `<div class="grid">
        ${articlesList.map(a => `
            <article>
                <header>
                    <small><a href="/section/${escapeHtml(a.section)}">${escapeHtml(a.section.toUpperCase())}</a></small>
                    <h2><a href="/article/${escapeHtml(a.slug)}">${escapeHtml(a.headline)}</a></h2>
                </header>
                <p>${escapeHtml(a.standfirst)}</p>
                <footer>
                    <small>By ${escapeHtml(a.byline)} on <time datetime="${escapeHtml(a.date)}">${escapeHtml(a.date.split('T')[0])}</time></small>
                </footer>
            </article>
        `).join('')}
    </div>`;
}

function renderFrontPage() {
    return renderLayout('Home', `
        <h2>Latest News</h2>
        ${renderArticleList(articles.slice(0, 12))}
    `);
}

function renderSectionPage(section) {
    const sectionArticles = articles.filter(a => a.section === section);
    if (sectionArticles.length === 0) return null;
    return renderLayout(section.charAt(0).toUpperCase() + section.slice(1), `
        <h2>${escapeHtml(section.charAt(0).toUpperCase() + section.slice(1))} News</h2>
        ${renderArticleList(sectionArticles)}
    `);
}

function renderArticlePage(slug) {
    const article = articles.find(a => a.slug === slug);
    if (!article) return null;
    return renderLayout(article.headline, `
        <article>
            <header>
                <small><a href="/section/${escapeHtml(article.section)}">${escapeHtml(article.section.toUpperCase())}</a></small>
                <h1>${escapeHtml(article.headline)}</h1>
                <p><strong>${escapeHtml(article.standfirst)}</strong></p>
                <p><small>By ${escapeHtml(article.byline)} | Published <time datetime="${escapeHtml(article.date)}">${escapeHtml(article.date.split('T')[0])}</time></small></p>
            </header>
            <div class="article-body">
                ${article.body.map(p => `<p>${escapeHtml(p)}</p>`).join('')}
            </div>
        </article>
    `);
}

// --- Server ---
const server = http.createServer((req, res) => {
    const url = new URL(req.url, `http://${req.headers.host}`);
    const pathname = url.pathname;

    if (pathname === '/healthz') {
        res.writeHead(200, { 'Content-Type': 'text/plain' });
        res.end('ok');
        return;
    }

    if (pathname === '/favicon.ico') {
        res.writeHead(204);
        res.end();
        return;
    }

    if (pathname === '/pico.min.css') {
        const cssPath = path.join(__dirname, 'node_modules', '@picocss', 'pico', 'css', 'pico.min.css');
        fs.readFile(cssPath, (err, data) => {
            if (err) {
                res.writeHead(404);
                res.end('Not found');
                return;
            }
            res.writeHead(200, { 'Content-Type': 'text/css' });
            res.end(data);
        });
        return;
    }

    if (pathname === '/') {
        res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
        res.end(renderFrontPage());
        return;
    }

    if (pathname.startsWith('/section/')) {
        const section = pathname.split('/')[2];
        const html = renderSectionPage(section);
        if (html) {
            res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
            res.end(html);
            return;
        }
    }

    if (pathname.startsWith('/article/')) {
        const slug = pathname.split('/')[2];
        const html = renderArticlePage(slug);
        if (html) {
            res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
            res.end(html);
            return;
        }
    }

    res.writeHead(404, { 'Content-Type': 'text/html; charset=utf-8' });
    res.end(renderLayout('Not Found', '<h2>404 Not Found</h2><p>The page you are looking for does not exist.</p>'));
});

server.listen(PORT, BIND_ADDR, () => {
    console.log(`${SITE_NAME} (ID: ${SITE_ID}) listening on http://${BIND_ADDR}:${PORT}`);
});
