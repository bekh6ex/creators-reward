// SPDX-FileCopyrightText: 2023 The creators-reward contributors
// SPDX-License-Identifier: MIT
//
// Creators Reward embed script. A creator site includes it with
//   <script src="https://<viewer gateway>/api/embed.js" defer></script>
// and it adds the events iframe to the page. It is served outside /api/events
// so its request, which carries this page as Referer, never carries the
// viewer's events token.
(function () {
    "use strict";

    // Captured synchronously: document.currentScript is null inside callbacks.
    // The iframe is a sibling of this script, so it is served by whichever
    // gateway served the script - no host is hardcoded into the page.
    const script = document.currentScript;
    if (!script) return;
    const IFRAME_URL = new URL("events/web/iframe.html", script.src).href;
    const IFRAME_ORIGIN = new URL(IFRAME_URL).origin;

    // Hidden = 1px, transparent, non-interactive - NOT display:none, which some
    // browsers treat as a hidden page and throttle. This way the iframe keeps
    // listening for messages.
    const STYLE_VISIBLE =
        "position:fixed;bottom:0;left:0;width:100%;height:48px;border:none;" +
        "z-index:2147483647;background:#fff;box-shadow:0 -2px 6px rgba(0,0,0,.3)";
    const STYLE_HIDDEN =
        "position:fixed;bottom:0;left:0;width:1px;height:1px;opacity:0;" +
        "pointer-events:none;border:none;z-index:-1";

    function embed() {
        // Included twice on one page: keep a single iframe.
        if (document.querySelector("iframe[data-creators-reward]")) return;

        const iframe = document.createElement("iframe");
        iframe.src = IFRAME_URL;
        iframe.title = "Creators Reward";
        iframe.dataset.creatorsReward = "";
        // The Storage Access API needs allow-scripts, allow-same-origin (an
        // opaque origin is auto-rejected) and
        // allow-storage-access-by-user-activation; allow="storage-access"
        // delegates the Permissions Policy. allow-popups(-to-escape-sandbox)
        // lets the frame open our pages top-level.
        iframe.setAttribute(
            "sandbox",
            "allow-scripts allow-same-origin allow-popups " +
                "allow-popups-to-escape-sandbox allow-storage-access-by-user-activation"
        );
        iframe.setAttribute("allow", "storage-access");
        // The frame learns the site from the postMessage below, which the
        // browser attests. No referrer, so the iframe request carrying the
        // viewer's token does not also name the site.
        iframe.referrerPolicy = "no-referrer";
        iframe.addEventListener("load", () => {
            iframe.contentWindow.postMessage({cmd: "site"}, IFRAME_ORIGIN);
        });
        iframe.style.cssText = STYLE_VISIBLE;

        // Messages from the iframe are validated by window identity, not origin.
        window.addEventListener("message", (event) => {
            if (event.source !== iframe.contentWindow) return;
            const cmd = event.data && event.data.cmd;
            if (cmd === "hide") iframe.style.cssText = STYLE_HIDDEN;
            else if (cmd === "show") iframe.style.cssText = STYLE_VISIBLE;
        });

        document.body.appendChild(iframe);
    }

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", embed);
    } else {
        embed();
    }
})();
